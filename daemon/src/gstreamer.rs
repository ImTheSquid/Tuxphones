use std::str::FromStr;
use std::sync::Arc;

use gst::{debug_bin_to_dot_data, DebugGraphDetails, Element, glib, PadLinkError, StateChangeError, StateChangeSuccess};
use gst::prelude::*;
use tokio::runtime::Handle;
use tracing::{debug, error, info, trace};
use webrtcredux::sdp::{SdpProp, MediaType, MediaProp, NetworkType, AddressType, LineEnding};
use webrtcredux::{RTCIceServer, RTCSdpType, RTCIceGathererState, RTCSdpSemantics, RTCBundlePolicy};
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use crate::{receive::StreamResolutionInformation, ToGst, xid};
use crate::receive::IceData;

use webrtcredux::webrtcredux::{
    sdp::{SDP},
    WebRtcRedux,
};

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
#[allow(dead_code)]
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

pub struct GstHandle {
    pipeline: gst::Pipeline,
    // webrtcbin: Element,
    webrtcredux: Arc<AsyncMutex<WebRtcRedux>>,
    encoder: Element,
    encoder_type: VideoEncoderType
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
    pub async fn new(
        encoder_to_use: VideoEncoderType, xid: xid, resolution: StreamResolutionInformation, fps: i32, ice: IceData
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

        //Chose encoder based on constructor params
        let encoder = match encoder_to_use {
            VideoEncoderType::H264(settings) => {
                //Use nvidia encoder based on settings
                if settings.nvidia_encoder {
                    let nvh264enc = gst::ElementFactory::make("nvh264enc", None)?;
                    nvh264enc.set_property("gop-size", 2560i32);
                    nvh264enc.set_property_from_str("rc-mode", "cbr-ld-hq");
                    nvh264enc.set_property("zerolatency", true);
                    nvh264enc
                } else {
                    let x264enc = gst::ElementFactory::make("x264enc", None)?;
                    x264enc.set_property("threads", 12u32);
                    x264enc.set_property_from_str("tune", "zerolatency");
                    x264enc.set_property_from_str("speed-preset", "ultrafast");
                    x264enc.set_property("key-int-max", 2560u32);
                    x264enc.set_property("b-adapt", false);
                    x264enc.set_property("vbv-buf-capacity", 120u32);
                    x264enc
                }
            }
            VideoEncoderType::VP8 => {
                let vp8enc = gst::ElementFactory::make("vp8enc", None)?;
                vp8enc.set_property("threads", 12i32);
                vp8enc.set_property("cpu-used", -16i32);
                vp8enc.set_property_from_str("end-usage", "cbr");
                vp8enc.set_property("buffer-initial-size", 100i32);
                vp8enc.set_property("buffer-optimal-size", 120i32);
                vp8enc.set_property("buffer-size", 150i32);
                vp8enc.set_property("max-intra-bitrate", 250i32);
                vp8enc.set_property_from_str("error-resilient", "default");
                vp8enc.set_property("lag-in-frames", 0i32);
                vp8enc
            }
            VideoEncoderType::VP9 => {
                let vp9enc = gst::ElementFactory::make("vp9enc", None)?;
                vp9enc.set_property("threads", 12i32);
                vp9enc.set_property("cpu-used", -16i32);
                vp9enc.set_property_from_str("end-usage", "cbr");
                vp9enc.set_property("buffer-initial-size", 100i32);
                vp9enc.set_property("buffer-optimal-size", 120i32);
                vp9enc.set_property("buffer-size", 150i32);
                vp9enc.set_property("max-intra-bitrate", 250i32);
                vp9enc.set_property_from_str("error-resilient", "default");
                vp9enc.set_property("lag-in-frames", 0i32);
                vp9enc
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
        opusenc.set_property("bitrate", 32000i32);
        opusenc.set_property_from_str("bitrate-type", "cbr");
        opusenc.set_property("dtx", true);
        opusenc.set_property("inband-fec", true);



        //--DESTINATION--

        let webrtcredux = Arc::new(AsyncMutex::new(WebRtcRedux::default()));
        webrtcredux.lock().await.set_tokio_runtime(Handle::current());
        webrtcredux.lock().await.set_sdp_semantics(RTCSdpSemantics::UnifiedPlan);
        webrtcredux.lock().await.set_bundle_policy(RTCBundlePolicy::MaxBundle);

        let servers = ice.urls.into_iter().map(|url| {
            if url.starts_with("turn") {
                RTCIceServer {
                    urls: vec![url],
                    username: ice.username.clone(),
                    credential: ice.credential.clone(),
                    .. RTCIceServer::default()
                }
            } else {
                RTCIceServer {
                    urls: vec![url],
                    .. RTCIceServer::default()
                }
            }
        }).collect::<Vec<_>>();

        debug!("Using ICE servers: {:#?}", servers);

        webrtcredux.lock().await.add_ice_servers(servers);

        //queues
        let video_encoder_queue = gst::ElementFactory::make("queue", None)?;
        let audio_encoder_queue = gst::ElementFactory::make("queue", None)?;
        let video_webrtc_queue = gst::ElementFactory::make("queue", None)?;
        let audio_webrtc_queue = gst::ElementFactory::make("queue", None)?;


        //Add elements to the pipeline
        pipeline.add_many(&[
            &ximagesrc, &videoscale, &capsfilter, &videoconvert, &encoder,
            &video_encoder_queue, &video_webrtc_queue,
            &pulsesrc, &audioconvert, &audio_capsfilter, &opusenc,
            &audio_encoder_queue, &audio_webrtc_queue,
            webrtcredux.lock().await.upcast_ref::<gst::Element>()])?;

        //Link video elements
        // Element::link_many(&[&ximagesrc, &videoscale, &capsfilter, &videoconvert, &video_encoder_queue, &encoder, &encoder_pay, &video_payload_caps, &video_webrtc_queue, &webrtcbin])?;
        Element::link_many(&[&ximagesrc, &videoscale, &capsfilter, &videoconvert, &video_encoder_queue, &encoder, &video_webrtc_queue, webrtcredux.lock().await.upcast_ref::<gst::Element>()])?;

        //Link audio elements
        Element::link_many(&[&pulsesrc, &audioconvert, &audio_encoder_queue, &opusenc, &audio_webrtc_queue, webrtcredux.lock().await.upcast_ref::<gst::Element>()])?;

        // Debug diagram
        let out = debug_bin_to_dot_data(&pipeline, DebugGraphDetails::ALL);
        std::fs::write("/tmp/tuxphones_gst.dot", out.as_str()).unwrap();

        Ok(GstHandle {
            pipeline,
            webrtcredux,
            encoder,
            encoder_type: encoder_to_use
        })
    }

    pub async fn start(&self, to_ws_tx: mpsc::Sender<ToWs>, from_ws_rx: mpsc::Receiver<ToGst>) -> Result<StateChangeSuccess, StateChangeError> {
        self.pipeline.set_state(gst::State::Playing)?;
        let encoder = self.encoder_type;

        let arc_from_ws = Arc::new(AsyncMutex::new(from_ws_rx));

        self.webrtcredux.lock().await.on_peer_connection_state_change(Box::new(|state| {
            debug!("[WebRTC] Peer connection state changed to: {}", state);

            Box::pin(async {})
        })).await.expect("Failed to set on peer connection state change");

        self.webrtcredux.lock().await.on_ice_connection_state_change(Box::new(|state| {
            debug!("[WebRTC] ICE connection state changed to: {}", state);

            Box::pin(async {})
        })).await.expect("Failed to set on ice connection state change");

        // let redux_arc = self.webrtcredux.clone();
        self.webrtcredux.lock().await.on_ice_candidate(Box::new(move |candidate| {
            // let redux_arc = redux_arc.clone();

            Box::pin(async move {
                if let Some(candidate) = candidate {
                    debug!("ICE Candidate: {:#?}", candidate.to_json().await.unwrap());
                }
                // redux_arc.lock().await.add_ice_candidate(candidate.unwrap().to_json().await.unwrap()).await.unwrap();
            })
        })).await.expect("Failed ice candidate");

        let redux_arc = self.webrtcredux.clone();
        self.webrtcredux.lock().await.on_negotiation_needed(Box::new(move || {
            let redux_arc = redux_arc.clone();

            info!("[WebRTC] Negotiation needed");

            Box::pin(async move {
                // Waits for all tracks to be added to create full SDP
                redux_arc.lock().await.wait_for_all_tracks().await;

                let offer = redux_arc.lock().await.create_offer(None).await.expect("Failed to create offer");

                trace!("[WebRTC] Generated local SDP: {:#?}", offer);

                redux_arc.lock().await.set_local_description(&offer, RTCSdpType::Offer).await.expect("Failed to set local description");

                info!("[WebRTC] Local description set");
            })
        })).await.expect("Failed to set on negotiation needed");

        let redux_arc = self.webrtcredux.clone();
        self.webrtcredux.lock().await.on_ice_gathering_state_change(Box::new(move |state| {
            debug!("[WebRTC] ICE gathering state changed to: {}", state);

            let redux_arc = redux_arc.clone();
            let to_ws_tx = to_ws_tx.clone();
            let from_ws_rx = arc_from_ws.clone();

            if state != RTCIceGathererState::Complete {
                return Box::pin(async {});
            }
            
            Box::pin(async move {
                let local = redux_arc.lock().await.local_description().await.unwrap().unwrap();

                let video_media: &SdpProp = local.props.iter().find(|v| match *v {
                    SdpProp::Media { r#type, .. } => {
                        *r#type == MediaType::Video
                    },
                    _ => false
                }).unwrap();

                let (video_ssrc, video_payload_type, rtx_payload_type) = if let SdpProp::Media { props, .. } = video_media {
                    let mut ssrc = 0u32;
                    let mut video_payload = 0u8;
                    let mut rtx_payload = 0u8;

                    for prop in props {
                        match prop {
                            MediaProp::Attribute { key, value } => {
                                match key {
                                    v if *v == "rtpmap".to_string() => {
                                        match value {
                                            Some(val) => {
                                                let num = val.clone().split(' ').collect::<Vec<_>>()[0].parse::<u8>().unwrap();
                                                if val.ends_with(&format!("{}/90000", encoder.type_string())) && video_payload == 0 {
                                                    video_payload = num;
                                                } else if val.ends_with("rtx/90000") && rtx_payload == 0 {
                                                    rtx_payload = num;
                                                }
                                            },
                                            None => unreachable!()
                                        }
                                    },
                                    v if *v == "ssrc".to_string() => {
                                        ssrc = match value {
                                            Some(val) => val.clone().split(' ').collect::<Vec<_>>()[0].parse::<u32>().unwrap(),
                                            None => unreachable!(),
                                        };
                                    },
                                    _ => continue
                                }
                            },
                            _ => continue
                        }
                    }

                    (ssrc, video_payload, rtx_payload)
                } else { unreachable!() };

                let audio_media: &SdpProp = local.props.iter().find(|v| match *v {
                    SdpProp::Media { r#type, .. } => {
                        *r#type == MediaType::Audio
                    },
                    _ => false
                }).unwrap();

                let audio_ssrc = if let SdpProp::Media { props, .. } = audio_media {
                    props.into_iter().find_map(|p| match p {
                        MediaProp::Attribute {key, value} => {
                            if key != "ssrc" {
                                return None;
                            }
                            let val = match value {
                                Some(val) => val.clone(),
                                None => unreachable!(),
                            };
                            Some(val.split(' ').collect::<Vec<_>>()[0].parse::<u32>().unwrap())
                        },
                        _ => None
                    }).unwrap()
                } else { unreachable!() };

                to_ws_tx.send(ToWs {
                    ssrcs: StreamSSRCs {
                        audio: audio_ssrc,
                        video: video_ssrc,
                        rtx: 0
                    },
                    local_sdp: local.to_string(LineEnding::LF),
                    video_payload_type,
                    rtx_payload_type,
                }).await.unwrap();

                let from_ws = from_ws_rx.lock().await.recv().await.unwrap();

                match SDP::from_str(&from_ws.remote_sdp).unwrap().props.pop().unwrap() {
                    SdpProp::Media { ports, props, .. } => {
                        let mut main_ip = None;
                        let mut fingerprint = None;
                        let mut ufrag = None;
                        let mut pwd = None;
                        let mut candidate = None;

                        for prop in props {
                            let current = prop.clone();
                            match prop {
                                MediaProp::Connection { address, .. } => main_ip = Some(address),
                                MediaProp::Attribute { key, value: _ } => {
                                    match &key[..] {
                                        "candidate" => candidate = Some(current),
                                        "fingerprint" => fingerprint = Some(current),
                                        "ice-ufrag" => ufrag = Some(current),
                                        "ice-pwd" => pwd = Some(current),
                                        _ => continue
                                    }
                                }
                                _ => continue
                            }
                        }

                        let connection = MediaProp::Connection {
                            net_type: NetworkType::Internet,
                            address_type: AddressType::IPv4,
                            address: main_ip.unwrap(),
                            ttl: Some(127),
                            num_addresses: Some(1),
                            suffix: None,
                        };

                        let base_media_props = vec![
                            connection,
                            // candidate.unwrap(),
                            fingerprint.unwrap(),
                            ufrag.unwrap(),
                            pwd.unwrap(),
                            MediaProp::Attribute {
                                key: "rtcp-mux".to_string(),
                                value: None
                            },
                            MediaProp::Attribute {
                                key: "rtcp".to_string(),
                                value: Some(ports[0].to_string())
                            },
                            MediaProp::Attribute {
                                key: "setup".to_string(),
                                value: Some("passive".to_string())
                            },
                            MediaProp::Attribute {
                                key: "inactive".to_string(),
                                value: None
                            }
                        ];

                        let mut video_vec_attrs = ["ccm fir", "nack", "nack pli", "goog-remb", "transport-cc"].into_iter().map(|val| {
                            MediaProp::Attribute {
                                key: "rtcp-fb".to_string(),
                                value: Some(format!("{} {}", video_payload_type, val))
                            }
                        }).collect::<Vec<_>>();

                        video_vec_attrs.append(&mut vec![
                            MediaProp::Attribute {
                                key: "fmtp".to_string(),
                                value: Some(format!("{} x-google-max-bitrate=2500;level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f", video_payload_type))
                            },
                            MediaProp::Attribute {
                                key: "fmtp".to_string(),
                                value: Some(format!("{} apt={}", rtx_payload_type, video_payload_type))
                            },
                            MediaProp::Attribute {
                                key: "mid".to_string(),
                                value: Some(0.to_string())
                            },
                            MediaProp::Attribute {
                                key: "rtpmap".to_string(),
                                value: Some(format!("{} {}/90000", video_payload_type, encoder.type_string()))
                            },
                            MediaProp::Attribute {
                                key: "rtpmap".to_string(),
                                value: Some(format!("{} rtx/90000", rtx_payload_type))
                            },
                            candidate.unwrap(),
                            MediaProp::Attribute {
                                key: "end-of-candidates".to_string(),
                                value: None
                            }
                        ]);

                        let video_media = SdpProp::Media {
                            r#type: MediaType::Video,
                            ports: ports.clone(),
                            protocol: format!("UDP/TLS/RTP/SAVPF {} {}", video_payload_type, rtx_payload_type),
                            format: "".to_string(),
                            props: base_media_props.clone().into_iter().chain(video_vec_attrs.into_iter()).collect::<Vec<_>>()
                        };

                        let audio_vec_attrs = vec![
                            MediaProp::Attribute {
                                key: "fmtp".to_string(),
                                value: Some("111 minptime=10;useinbandfec=1;usedtx=1".to_string())
                            },
                            MediaProp::Attribute {
                                key: "maxptime".to_string(),
                                value: Some(60.to_string())
                            },
                            MediaProp::Attribute {
                                key: "rtpmap".to_string(),
                                value: Some("111 opus/48000/2".to_string())
                            },
                            MediaProp::Attribute {
                                key: "rtcp-fb".to_string(),
                                value: Some("111 transport-cc".to_string())
                            },
                            MediaProp::Attribute {
                                key: "mid".to_string(),
                                value: Some(1.to_string())
                            }
                        ];

                        let audio_media = SdpProp::Media {
                            r#type: MediaType::Audio,
                            ports,
                            protocol: "UDP/TLS/RTP/SAVPF 111".to_string(),
                            format: "".to_string(),
                            props: base_media_props.clone().into_iter().chain(audio_vec_attrs.into_iter()).collect::<Vec<_>>()
                        };

                        // Generate answer
                        let answer = SDP { props: vec![
                            SdpProp::Version(0),
                            SdpProp::Origin { 
                                username: "-".to_string(), 
                                session_id: "1420070400000".to_string(), 
                                session_version: 0, 
                                net_type: NetworkType::Internet, 
                                address_type: AddressType::IPv4, 
                                address: "127.0.0.1".to_string() 
                            },
                            SdpProp::SessionName("-".to_string()),
                            SdpProp::Timing {
                                start: 0,
                                stop: 0
                            },
                            SdpProp::Attribute {
                                key: "msid-semantic".to_string(),
                                value: Some(" WMS *".to_string())
                            },
                            SdpProp::Attribute {
                                key: "group".to_string(),
                                value: Some("BUNDLE 0 1".to_string())
                            },
                            video_media,
                            audio_media
                        ]};

                        trace!("[WebRTC] Generated remote SDP: {:#?}", answer);

                        redux_arc.lock().await.set_remote_description(&answer, RTCSdpType::Answer).await.expect("Failed to set remote description");

                        info!("[WebRTC] Remote description set");
                    }
                    _ => unreachable!()
                }
            })
        })).await.expect("Failed to set on ice gathering change");

        Ok(StateChangeSuccess::Success)
    }
}