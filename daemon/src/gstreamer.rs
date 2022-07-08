use std::sync::{Arc, Mutex};

use async_std::task;
use gst::{debug_bin_to_dot_data, DebugGraphDetails, Element, glib, PadLinkError, Promise, StateChangeError, StateChangeSuccess};
use gst::prelude::*;
use gst_webrtc::{WebRTCSessionDescription};
use once_cell::sync::Lazy;
use tracing::{debug, error, info, trace};

use crate::{receive::StreamResolutionInformation, ToGst, xid};
use crate::receive::IceData;

//Gstreamer handles count to prevent deinitialization of gstreamer
static HANDLES_COUNT: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));

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

pub struct H264Settings {
    pub nvidia_encoder: bool,
}

pub enum VideoEncoderType {
    H264(H264Settings),
    VP8,
    VP9,
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
        let mut handles_count = HANDLES_COUNT.lock().unwrap();

        // Debug diagram
        let out = debug_bin_to_dot_data(&self.pipeline, DebugGraphDetails::ALL);
        std::fs::write("/tmp/tuxphones_gstdrop.dot", out.as_str()).unwrap();

        self.pipeline.send_event(gst::event::Eos::new());
        if let Err(e) = self.pipeline.set_state(gst::State::Null) {
            error!("Failed to stop pipeline: {:?}", e);
        };

        //Gst should be destroyed only when there are no more handles
        if *handles_count > 0 {
            *handles_count -= 1;
            if *handles_count == 0 {
                unsafe {
                    gst::deinit();
                }
            }
        }
    }
}

impl GstHandle {
    /// # Arguments
    /// * `sdp` - SDP message from discord, CRLF line endings are required (\r\n)
    pub fn new(
        encoder_to_use: VideoEncoderType, xid: xid, resolution: StreamResolutionInformation, fps: i32, ice: IceData, to_ws_tx: async_std::channel::Sender<ToWs>, from_ws_rx: async_std::channel::Receiver<ToGst>,
    ) -> Result<Self, GstInitializationError> {
        gst::init()?;
        *HANDLES_COUNT.lock().unwrap() += 1;

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

        //Create a new webrtcbin to connect the pipeline to the WebRTC peer
        let webrtcbin = gst::ElementFactory::make("webrtcbin", None)?;

        //TODO: Use filter_map instead
        let stun_server = ice.urls.iter().find(|url| url.starts_with("stun:")).unwrap().replace("stun:", "stun://");

        //TODO: Find a way to sanitize ice.username and ice.password
        let turn_auth = format!("turn://{}:{}@", ice.username, ice.credential);
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

        webrtcbin.connect("on-negotiation-needed", false, move |value| {
            let to_ws_tx = to_ws_tx.clone();
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

                            let sdp = session_description.sdp().as_text().unwrap().replace("\r\n", "\n");
                            trace!("[WebRTC] Offer: {:?}", sdp);

                            let mut video_ssrc: u32 = 0;
                            let mut audio_ssrc: u32= 0;
                            let mut rtx_ssrc: u32= 0;


                            {
                                let webrtcbin = webrtcbin.lock().unwrap();
                                webrtcbin.emit_by_name::<()>("set-local-description", &[&session_description, &None::<Promise>]);

                                let video_pad = webrtcbin.static_pad("sink_0").unwrap();
                                let audio_pad = webrtcbin.static_pad("sink_1").unwrap();

                                video_ssrc = get_cap_value_from_str::<u32>(&video_pad.caps().unwrap(), "ssrc").unwrap_or(0);
                                audio_ssrc = get_cap_value_from_str::<u32>(&audio_pad.caps().unwrap(), "ssrc").unwrap_or(0);
                            }

                            info!("[WebRTC] Local description set");
                            trace!("[WebRTC] Video SSRC: {:?}", video_ssrc);
                            trace!("[WebRTC] Audio SSRC: {:?}", audio_ssrc);
                            trace!("[WebRTC] RTX SSRC: {:?}", rtx_ssrc);


                            match task::block_on(to_ws_tx.send(ToWs {
                                ssrcs: StreamSSRCs {
                                    audio: audio_ssrc,
                                    video: video_ssrc,
                                    rtx: rtx_ssrc,
                                },
                                local_sdp: sdp,
                            })) {
                                Ok(_) => {
                                    debug!("[WebRTC->WS] SDP and SSRCs sent to websocket");
                                }
                                Err(e) => {
                                    error!("[WebRTC] Failed to send local SDP and SSRCs to websocket: {:?}", e);
                                }
                            };
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

        /*
        OLD CODE TO SET remote description:

        let mut sdp_message = SDPMessage::new();
        sdp_message.set_uri(sdp_server);
        let webrtc_desc = WebRTCSessionDescription::new(
            WebRTCSDPType::Answer,
            sdp_message,
        );
        webrtcbin.emit_by_name::<()>("set-remote-description", &[&webrtc_desc, &None::<gst::Promise>]);

         */

        //queues
        let video_encoder_queue = gst::ElementFactory::make("queue", None)?;
        let audio_encoder_queue = gst::ElementFactory::make("queue", None)?;
        let video_webrtc_queue = gst::ElementFactory::make("queue", None)?;
        let audio_webrtc_queue = gst::ElementFactory::make("queue", None)?;

        //Add elements to the pipeline
        pipeline.add_many(&[
            &ximagesrc, &videoscale, &capsfilter, &videoconvert, &encoder, &encoder_pay,
            &video_encoder_queue, &video_webrtc_queue,
            &pulsesrc, &audioconvert, &audio_capsfilter, &opusenc, &rtpopuspay,
            &audio_encoder_queue, &audio_webrtc_queue,
            &webrtcbin])?;

        //Link video elements
        Element::link_many(&[&ximagesrc, &videoscale, &capsfilter, &videoconvert, &video_encoder_queue, &encoder, &encoder_pay, &video_webrtc_queue, &webrtcbin])?;

        //Link audio elements
        Element::link_many(&[&pulsesrc, &audioconvert, &audio_capsfilter, &audio_encoder_queue, &opusenc, &rtpopuspay, &audio_webrtc_queue, &webrtcbin])?;

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