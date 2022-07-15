use std::sync::{Arc, Mutex};

use async_std::task;
use gst::{debug_bin_to_dot_data, DebugGraphDetails, Element, glib, PadLinkError, Promise, StateChangeError, StateChangeSuccess};
use gst::prelude::*;
use gst_sdp::{SDPMedia, SDPMessage};
use gst_webrtc::{WebRTCICEGatheringState, WebRTCRTPTransceiver, WebRTCSDPType, WebRTCSessionDescription};
use once_cell::sync::Lazy;
use tracing::{debug, error, info, trace};

use crate::{receive::StreamResolutionInformation, ToGst, xid};
use crate::receive::IceData;

#[derive(Debug)]
pub enum GstInitializationError {
    Init(glib::Error),
    Element(glib::BoolError),
    Pad(PadLinkError),
}

#[derive(Debug)]
pub struct StreamSSRCs {
    pub audio: u32,
    pub video: u32,
    pub rtx: u32,
}

#[derive(Debug)]
pub struct ToWs {
    pub ssrcs: StreamSSRCs,
    pub local_sdp: String,
    pub video_payload_type: u8,
    pub rtx_payload_type: u8
}

impl std::fmt::Display for GstInitializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            GstInitializationError::Init(e) => format!("Initialization error: {:?}", e),
            GstInitializationError::Element(e) => format!("Element error: {:?}", e),
            GstInitializationError::Pad(e) => format!("Pad error: {:?}", e),
        };
        f.write_str(&str)
    }
}

#[derive(Clone, Copy)]
pub struct H264Settings {
    pub nvidia_encoder: bool,
}

#[derive(Clone, Copy)]
pub enum VideoEncoderType {
    H264(H264Settings),
    VP8,
    VP9,
}

impl VideoEncoderType {
    fn type_string(self) -> &'static str {
        match self {
            VideoEncoderType::H264(_) => "H264",
            VideoEncoderType::VP8 => "VP8",
            VideoEncoderType::VP9 => "VP9",
        }
    }
}

impl From<glib::Error> for GstInitializationError {
    fn from(error: glib::Error) -> Self {
        GstInitializationError::Init(error)
    }
}

impl From<glib::BoolError> for GstInitializationError {
    fn from(error: glib::BoolError) -> Self {
        GstInitializationError::Element(error)
    }
}

impl From<PadLinkError> for GstInitializationError {
    fn from(error: PadLinkError) -> Self {
        GstInitializationError::Pad(error)
    }
}

#[derive(Debug)]
pub struct GstHandle {
    pipeline: gst::Pipeline,
    webrtcbin: Element,
    encoder: Element,
}

//Custom drop logic to deinit gstreamer when all handles are dropped
impl Drop for GstHandle {
    fn drop(&mut self) {
        info!("dropping GstHandle");
        // Debug diagram
        let out = debug_bin_to_dot_data(&self.pipeline, DebugGraphDetails::ALL);
        std::fs::write("/tmp/tuxphones_gstdrop.dot", out.as_str()).unwrap();

        if let Err(e) = self.pipeline.set_state(gst::State::Null) {
            error!("Failed to stop pipeline: {:?}", e);
        };
    }
}

impl GstHandle {
    /// # Arguments
    /// * `sdp` - SDP message from discord, CRLF line endings are required (\r\n)
    pub fn new(
        encoder_to_use: VideoEncoderType, xid: xid, resolution: StreamResolutionInformation, fps: i32, ice: IceData, to_ws_tx: async_std::channel::Sender<ToWs>, from_ws_rx: async_std::channel::Receiver<ToGst>,
    ) -> Result<Self, GstInitializationError> {
        //Create a new GStreamer pipeline
        let pipeline = gst::Pipeline::new(None);

        //--VIDEO--

        //Create a new ximagesrc to get video from the X server
        let ximagesrc = gst::ElementFactory::make("ximagesrc", None)?;

        let videoscale = gst::ElementFactory::make("videoscale", None)?;

        //Creating a capsfilter to set the resolution and the fps
        let capsfilter = gst::ElementFactory::make("capsfilter", None)?;

        let fps_frac = gst::Fraction::new(fps, 1);

        //Create a vector containing the option of the gst caps
        let mut caps_options: Vec<(&str, &(dyn ToSendValue + Sync))> = vec![("framerate", &fps_frac)];

        //If the resolution is specified, add it to the caps
        let width = resolution.width as i32;
        let height = resolution.height as i32;
        if resolution.is_fixed {
            caps_options.push(("width", &width));
            caps_options.push(("height", &height));
        };


        capsfilter.set_property("caps", &gst::Caps::new_simple(
            "video/x-raw",
            caps_options.as_ref(),
        ));

        ximagesrc.set_property_from_str("show-pointer", "1");
        //Set xid based on constructor parameter to get video only from the specified X window
        ximagesrc.set_property("xid", xid as u64);

        //Create a new videoconvert to allow encoding of the raw video
        let videoconvert = gst::ElementFactory::make("videoconvert", None)?;

        //Chose encoder and rtp encapsulator based on constructor params
        let (encoder, encoder_pay) = match encoder_to_use {
            VideoEncoderType::H264(settings) => {
                (
                    //Use nvidia encoder based on settings
                    if settings.nvidia_encoder {
                        gst::ElementFactory::make("nvh264enc", None)?
                    } else {
                        gst::ElementFactory::make("x264enc", None)?
                    },
                    gst::ElementFactory::make("rtph264pay", None)?
                )
            }
            VideoEncoderType::VP8 => {
                (
                    gst::ElementFactory::make("vp8enc", None)?,
                    gst::ElementFactory::make("rtpvp8pay", None)?
                )
            }
            VideoEncoderType::VP9 => {
                (
                    gst::ElementFactory::make("vp9enc", None)?,
                    gst::ElementFactory::make("rtpvp9pay", None)?
                )
            }
        };



        //--AUDIO--

        // Caps filter for audio from conversion to encoding
        let audio_capsfilter = gst::ElementFactory::make("capsfilter", None)?;

        //Create a vector containing the option of the gst caps
        let caps_options: Vec<(&str, &(dyn ToSendValue + Sync))> = vec![("channels", &2), ("rate", &48000)];

        audio_capsfilter.set_property("caps", &gst::Caps::new_simple(
            "audio/x-raw",
            caps_options.as_ref(),
        ));

        //Create a new pulsesrc to get audio from the PulseAudio server
        let pulsesrc = gst::ElementFactory::make("pulsesrc", None)?;
        //Set the audio device based on constructor parameter (should be the sink of the audio application)
        pulsesrc.set_property_from_str("device", "tuxphones.monitor");

        //Create a new audioconvert to allow encoding of the raw audio
        let audioconvert = gst::ElementFactory::make("audioconvert", None)?;
        //Encoder for the raw audio to opus
        let opusenc = gst::ElementFactory::make("opusenc", None)?;
        //Opus encapsulator for rtp
        let rtpopuspay = gst::ElementFactory::make("rtpopuspay", None)?;

        //--DESTINATION--

        //Create a new Ti i to connect the pipeline to the WebRTC peer
        let webrtcbin = gst::ElementFactory::make("webrtcbin", None)?;
        webrtcbin.set_property_from_str("bundle-policy", "max-bundle");

        //TODO: Use filter_map instead
        let stun_server = ice.urls.iter().find(|url| url.starts_with("stun:")).unwrap().replace("stun:", "stun://");

        //TODO: Find a way to sanitize ice.username and ice.password
        let turn_auth = format!("turn://{}:{}@", ice.username, ice.credential.replace('+', "-").replace('/', "_"));
        let turn_servers = ice.urls.iter().filter(|url| url.starts_with("turn:")).map(|url| url.replace("turn:", &turn_auth)).collect::<Vec<_>>();
        debug!("Using STUN server: {:?}", stun_server);
        debug!("Using TURN servers: {:?}", turn_servers);
        webrtcbin.set_property_from_str("stun-server", &stun_server);

        //TODO: Use the for after instead of this before release
        webrtcbin.set_property_from_str("turn-server", &turn_servers[0]);
        /*
        for turn_server in turn_servers {
            webrtcbin.emit_by_name::<bool>("add-turn-server", &[&turn_server]);
        }
         */

        //queues
        let video_encoder_queue = gst::ElementFactory::make("queue", None)?;
        let audio_encoder_queue = gst::ElementFactory::make("queue", None)?;
        let video_webrtc_queue = gst::ElementFactory::make("queue", None)?;
        let audio_webrtc_queue = gst::ElementFactory::make("queue", None)?;

        /*let video_payload_caps = gst::ElementFactory::make("capsfilter", None)?;
        //Create a vector containing the option of the gst caps
        //TODO: Use a const for this since is the same that should be sent through the websocket with the opcode 1
        let caps_options: Vec<(&str, &(dyn ToSendValue + Sync))> = vec![("payload", &127)];

        video_payload_caps.set_property("caps", &gst::Caps::new_simple(
            "application/x-rtp",
            caps_options.as_ref(),
        ));

        let audio_payload_caps = gst::ElementFactory::make("capsfilter", None)?;
        //Create a vector containing the option of the gst caps
        //TODO: Use a const for this since is the same that should be sent through the websocket with the opcode 1
        let caps_options: Vec<(&str, &(dyn ToSendValue + Sync))> = vec![("payload", &111)];

        audio_payload_caps.set_property("caps", &gst::Caps::new_simple(
            "application/x-rtp",
            caps_options.as_ref(),
        ));*/


        //Add elements to the pipeline
        pipeline.add_many(&[
            &ximagesrc, &videoscale, &capsfilter, &videoconvert, &encoder, &encoder_pay, //&video_payload_caps,
            &video_encoder_queue, &video_webrtc_queue,
            &pulsesrc, &audioconvert, &audio_capsfilter, &opusenc, &rtpopuspay, //&audio_payload_caps,
            &audio_encoder_queue, &audio_webrtc_queue,
            &webrtcbin])?;

        //Link video elements
        Element::link_many(&[&ximagesrc, &videoscale, &capsfilter, &videoconvert, &video_encoder_queue, &encoder, &encoder_pay, /*&video_payload_caps,*/ &video_webrtc_queue, &webrtcbin])?;

        //Setting do-nack on webrtcbin video webrtctransceiver to true for rtx
        let video_transceiver = webrtcbin.static_pad("sink_0").unwrap().property::<WebRTCRTPTransceiver>("transceiver");
        video_transceiver.set_property("do-nack", true);
        video_transceiver.set_property("fec-type", gst_webrtc::WebRTCFECType::UlpRed);

        //Link audio elements
        Element::link_many(&[&pulsesrc, &audioconvert, &audio_capsfilter, &audio_encoder_queue, &opusenc, &rtpopuspay, /*&audio_payload_caps,*/ &audio_webrtc_queue, &webrtcbin])?;

        webrtcbin.connect("on-negotiation-needed", false, move |value| {
            info!("[WebRTC] Negotiation needed");

            let webrtcbin = Arc::new(Mutex::new(value[0].get::<Element>().unwrap()));

            let create_offer_options = gst::Structure::new_empty("create_offer_options");

            webrtcbin.lock().unwrap().emit_by_name::<()>("create-offer", &[&create_offer_options, &Promise::with_change_func({
                let webrtcbin = webrtcbin.clone();
                move |result| {
                    match result {
                        Ok(offer) => {
                            info!("[WebRTC] Offer created");

                            let session_description = offer.unwrap().get::<WebRTCSessionDescription>("offer").unwrap();

                            let sdp: String = session_description.sdp().as_text().unwrap().replace("\r\n", "\n");
                            trace!("[WebRTC] Offer: {:?}", sdp);
                            webrtcbin.lock().unwrap().emit_by_name::<()>("set-local-description", &[&session_description, &None::<Promise>]);
                        }
                        Err(error) => {
                            error!("[WebRTC] Failed to create offer: {:?}", error);
                            //TODO: Return an error to the new call by making this method async or blocking (Preferably async)
                        }
                    }
                }
            })]);
            None
        });

        #[cfg(debug_assertions)]
        webrtcbin.connect("on-ice-candidate", true, move |value| {
            //let webrtcbin = Arc::new(Mutex::new(value[0].get::<Element>().unwrap()));
            let candidate = value[2].get::<String>();
            debug!("[WebRTC] ICE candidate received: {:?}", candidate);
            None
        });

        webrtcbin.connect_notify(Some("ice-gathering-state"), move |webrtcbin, _| {
            let to_ws_tx = to_ws_tx.clone();
            let from_ws_rx = from_ws_rx.clone();

            let state = webrtcbin.property::<WebRTCICEGatheringState>("ice-gathering-state");
            debug!("[WebRTC] ICE gathering state changed: {:?}", state);
            if state == WebRTCICEGatheringState::Complete {
                let local_description = webrtcbin.property::<WebRTCSessionDescription>("local-description");
                let local_sdp = local_description.sdp();
                debug!("[WebRTC] Local media count: {}", local_sdp.medias_len());
                let sdp_filtered = get_filtered_sdp(local_sdp.as_text().unwrap());

                let video_pad = webrtcbin.static_pad("sink_0").unwrap();
                let audio_pad = webrtcbin.static_pad("sink_1").unwrap();

                let video_ssrc: u32 = get_cap_value_from_str::<u32>(&video_pad.caps().unwrap(), "ssrc").unwrap_or(0);
                let audio_ssrc: u32 = get_cap_value_from_str::<u32>(&audio_pad.caps().unwrap(), "ssrc").unwrap_or(0);
                // let rtx_ssrc: u32 = sdp_filtered.split('\n').find(|line| line.starts_with("a=ssrc-group:FID")).unwrap().split(' ').collect::<Vec<&str>>()[2].parse().unwrap();

                let media_string = sdp_filtered.split('\n').find(|line| line.starts_with("m=video")).unwrap().split(' ').collect::<Vec<&str>>();
                let mut media_iter = local_sdp.medias();
                let video_media = media_iter.next().unwrap();
                let video_payload_type = media_string[3].parse().unwrap(); // [3]
                let rtx_payload_type = media_string[4].parse().unwrap(); // [4]

                // let ufrag = video_media.attribute_val("ice-ufrag").unwrap();
                // let pwd = video_media.attribute_val("ice-pwd").unwrap();
                // let fingerprint = video_media.attribute_val("fingerprint").unwrap();
                let rtx_ssrc: u32 = video_media.attribute_val("ssrc-group").unwrap().split(' ').collect::<Vec<&str>>()[2].parse().unwrap();

                // TODO: Extract audio payload type
                let audio_payload_type = sdp_filtered.split('\n').find(|line| line.starts_with("m=video")).unwrap().split(' ').collect::<Vec<&str>>()[3].parse().unwrap();

                trace!("[WebRTC] Transcirver: {:?}", video_transceiver);

                info!("[WebRTC] Local description set");
                trace!("[WebRTC] Video SSRC: {:?}", video_ssrc);
                trace!("[WebRTC] Audio SSRC: {:?}", audio_ssrc);
                trace!("[WebRTC] RTX SSRC: {:?}", rtx_ssrc);

                match task::block_on(to_ws_tx.send(ToWs {
                    ssrcs: StreamSSRCs {
                        audio: audio_ssrc,
                        video: video_ssrc,
                        rtx: rtx_ssrc
                    },
                    local_sdp: sdp_filtered,
                    video_payload_type,
                    rtx_payload_type
                })) {
                    Ok(_) => {
                        debug!("[WebRTC->WS] SDP and SSRCs sent to websocket");
                    }
                    Err(e) => {
                        //TODO: Handle error
                        error!("[WebRTC] Failed to send local SDP and SSRCs to websocket: {:?}", e);
                    }
                };

                // Type conversion
                let video_payload_type = video_payload_type.into();
                let rtx_payload_type = rtx_payload_type.into();

                let from_ws = task::block_on(from_ws_rx.recv()).unwrap();
                debug!("[WebRTC] Received remote SDP from ws");
                trace!("[WebRTC] Remote SDP: {:?}", from_ws.remote_sdp);

                let original_sdp_entries = from_ws.remote_sdp.split('\n').collect::<Vec<&str>>();

                let candidate = &original_sdp_entries.clone().into_iter().find(|line| line.starts_with("a=candidate")).unwrap()[12..];
                let fingerprint = &original_sdp_entries.clone().into_iter().find(|line| line.starts_with("a=fingerprint")).unwrap()[14..];
                let ufrag = &original_sdp_entries.clone().into_iter().find(|line| line.starts_with("a=ice-ufrag")).unwrap()[12..];
                let pwd = &original_sdp_entries.clone().into_iter().find(|line| line.starts_with("a=ice-pwd")).unwrap()[10..];

                let main_port = original_sdp_entries[0].split(' ').nth(1).unwrap();
                let main_port = main_port.parse::<u16>().unwrap();
                let main_ip = from_ws.remote_sdp.split([' ', '\n']).find(|line| line.matches('.').count() == 3).unwrap();
                debug!("[WebRTC] IP: {}:{}", main_ip, main_port);

                let mut edited_sdp_message = SDPMessage::new();
                //Hardcoded description values
                edited_sdp_message.set_version("0");
                edited_sdp_message.set_origin("-", "1420070400000", "0", "IN", "IP4", "127.0.0.1");
                edited_sdp_message.set_session_name("-");
                edited_sdp_message.add_attribute("msid-semantic", Some(" WMS *"));

                const MAX_MID: u8 = 1; // used to be 21
                edited_sdp_message.add_attribute("group", Some(&format!("BUNDLE {}", (0..=MAX_MID).into_iter().map(|v| v.to_string()).collect::<Vec<String>>().join(" "))));

                for i in 0..=MAX_MID {
                    let attrs = MediaAttributes {
                        main_ip,
                        mid: i,
                        port: main_port,
                        ufrag,
                        pwd,
                        fingerprint,
                        candidate
                    };

                    if i == 0 {
                        edited_sdp_message.add_media(new_sdp_video_media(video_payload_type, rtx_payload_type, &encoder_to_use,  &attrs));
                    } else {
                        edited_sdp_message.add_media(new_sdp_audio_media(audio_payload_type, &attrs));
                    }
                }

                debug!("[WebRTC] Generated media count: {}", edited_sdp_message.medias_len());


                trace!("[WebRTC] Edited SDP: {:?}", edited_sdp_message.as_text());

                let webrtc_desc = WebRTCSessionDescription::new(
                    WebRTCSDPType::Answer,
                    edited_sdp_message,
                );
                webrtcbin.emit_by_name::<()>("set-remote-description", &[&webrtc_desc, &None::<gst::Promise>]);
                debug!("[WebRTC] Remote description set");
            }
        });

        // Debug diagram
        let out = debug_bin_to_dot_data(&pipeline, DebugGraphDetails::ALL);
        std::fs::write("/tmp/tuxphones_gst.dot", out.as_str()).unwrap();

        Ok(GstHandle {
            pipeline,
            webrtcbin,
            encoder,
        })
    }

    pub fn start(&self) -> Result<StateChangeSuccess, StateChangeError> {
        self.pipeline.set_state(gst::State::Playing)
    }
}

pub fn get_cap_value_from_str<'l, T: glib::value::FromValue<'l>>(caps: &'l gst::Caps, value: &str) -> Option<T> {
    match caps.structure(0).unwrap().get::<T>(value) {
        Ok(value) => Some(value),
        Err(_) => None,
    }
}

fn get_filtered_sdp(sdp: String) -> String {
    sdp.split("\r\n")
        .filter_map(|line| {
            if !line.starts_with("a=candidate") {
                return Some(line.to_string());
            }

            let tokens = line.split(' ').collect::<Vec<&str>>();

            if tokens[2] == "TCP" {
                return None;
            }

            Some(tokens.join(" "))
        }).collect::<Vec<String>>().join("\n")
}

fn new_sdp_audio_media(payload: u32, attrs: &MediaAttributes) -> SDPMedia {
    let mut media = SDPMedia::new();
    media.set_media("audio");
    media.set_proto(format!("UDP/TLS/RTP/SAVPF {}", payload).as_str());

    media.add_attribute("fmtp", Some(&format!("{} minptime=10;useinbandfec=1;usedtx=1", payload)));

    media.add_attribute("maxptime", Some("60"));

    media.add_attribute("rtpmap", Some(&format!("{} opus/90000", payload)));

    media.add_attribute("rtcp-fb", Some(&format!("{} transport-cc", payload)));

    [
        "1 urn:ietf:params:rtp-hdrext:ssrc-audio-level", 
        "3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01"
    ].into_iter().for_each(|ext| {
        media.add_attribute("extmap", Some(ext));
    });

    fill_media_minimal(&mut media, attrs);

    media
}

fn new_sdp_video_media(payload: u32, rtx_payload: u32, encoder: &VideoEncoderType, attrs: &MediaAttributes) -> SDPMedia {
    let mut media = SDPMedia::new();
    media.set_media("video");
    media.set_proto(format!("UDP/TLS/RTP/SAVPF {} {}", payload, rtx_payload).as_str());

    media.add_attribute("fmtp", Some(&format!("{} x-google-max-bitrate=2500;level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f", payload)));
    media.add_attribute("fmtp", Some(&format!("{} apt={}", rtx_payload, payload)));

    ["ccm fir", "nack", "nack pli", "goog-remb", "transport-cc"].into_iter().for_each(|val| {
        media.add_attribute("rtcp-fb", Some(&format!("{} {}", payload, val)));
    });

    [
        "2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time", 
        "3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01",
        "14 urn:ietf:params:rtp-hdrext:toffset",
        "13 urn:3gpp:video-orientation",
        "5 http://www.webrtc.org/experiments/rtp-hdrext/playout-delay"
    ].into_iter().for_each(|ext| {
        media.add_attribute("extmap", Some(ext));
    });

    media.add_attribute("rtpmap", Some(&format!("{} {}/90000", payload, encoder.type_string())));
    media.add_attribute("rtpmap", Some(&format!("{} rtx/90000", rtx_payload)));

    fill_media_minimal(&mut media, attrs);

    media
}

struct MediaAttributes<'a> {
    main_ip: &'a str,
    port: u16,
    mid: u8,
    ufrag: &'a str,
    pwd: &'a str,
    fingerprint: &'a str,
    candidate: &'a str
}

/// Fills the media with common attributes
fn fill_media_minimal(media: &mut SDPMedia, attrs: &MediaAttributes) {
    media.set_port_info(attrs.port.into(), 0);
    media.add_connection("IN", "IP4", attrs.main_ip, 127, 0);

    media.add_attribute("mid", Some(&attrs.mid.to_string()));

    // Should probably be None, but doing so causes a segfault
    media.add_attribute("rtcp-mux", Some(""));

    media.add_attribute("rtcp", Some(&attrs.port.to_string()));

    media.add_attribute("setup", Some("passive"));

    // Should probably be None, but doing so causes a segfault
    media.add_attribute("inactive", Some(""));

    media.add_attribute("ice-ufrag", Some(attrs.ufrag));
    media.add_attribute("ice-pwd", Some(attrs.pwd));

    media.add_attribute("fingerprint", Some(attrs.fingerprint));

    media.add_attribute("candidate", Some(attrs.candidate));
}