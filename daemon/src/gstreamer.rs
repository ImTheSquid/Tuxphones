use std::sync::Mutex;

use gst::{Element, glib, PadLinkError, StateChangeError, StateChangeSuccess};
use gst::prelude::*;
use gst_sdp::SDPMessage;
use gst_webrtc::{WebRTCSDPType, WebRTCSessionDescription};
use once_cell::sync::Lazy;
use tracing::error;

use crate::{receive::StreamResolutionInformation, xid};

//Gstreamer handles count to prevent deinitialization of gstreamer
static HANDLES_COUNT: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));

#[derive(Debug)]
pub enum GstInitializationError {
    Init(glib::Error),
    Element(glib::BoolError),
    Pad(PadLinkError),
}

impl std::fmt::Display for GstInitializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            GstInitializationError::Init(e) => format!("Initialization error: {}", e),
            GstInitializationError::Element(e) => format!("Element error: {}", e),
            GstInitializationError::Pad(e) => format!("Pad error: {}", e),
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

//Allowing non camel case names for this struct to match the discord encryption algorithm names
#[allow(non_camel_case_types)]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum EncryptionAlgorithm {
    aead_aes256_gcm
}

impl EncryptionAlgorithm {
    pub fn to_gst_str(&self) -> &'static str {
        match self {
            EncryptionAlgorithm::aead_aes256_gcm => "aes-256-gcm",
        }
    }

    pub fn to_discord_str(&self) -> &'static str {
        match self {
            EncryptionAlgorithm::aead_aes256_gcm => "aead_aes256_gcm",
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
        let mut handles_count = HANDLES_COUNT.lock().unwrap();

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
    pub fn new(
        encoder_to_use: VideoEncoderType, xid: xid, resolution: StreamResolutionInformation, fps: i32,
        audio_ssrc: u32, video_ssrc: u32, rtx_ssrc: u32,
        discord_address: &str, encryption_algorithm: EncryptionAlgorithm, key: Vec<u8>
    ) -> Result<Self, GstInitializationError> {
        gst::init()?;
        *HANDLES_COUNT.lock().unwrap() += 1;

        //Create a new GStreamer pipeline
        let pipeline = gst::Pipeline::new(None);

        //--VIDEO--

        //Create a new ximagesrc to get video from the X server
        let ximagesrc = gst::ElementFactory::make("ximagesrc", None)?;

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
            caps_options.as_ref()
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

        encoder_pay.set_property("ssrc", video_ssrc);


        //--AUDIO--

        //Create a new pulsesrc to get audio from the PulseAudio server
        let pulsesrc = gst::ElementFactory::make("pulsesrc", None)?;
        //Set the audio device based on constructor parameter (should be the sink of the audio application)
        pulsesrc.set_property_from_str("device", "tuxphones");

        //Create a new audioconvert to allow encoding of the raw audio
        let audioconvert = gst::ElementFactory::make("audioconvert", None)?;
        //Encoder for the raw audio to opus
        let opusenc = gst::ElementFactory::make("opusenc", None)?;
        //Opus encapsulator for rtp
        let rtpopuspay = gst::ElementFactory::make("rtpopuspay", None)?;
        rtpopuspay.set_property("ssrc", audio_ssrc);


        //--DESTINATION--

        //mux
        let rtpmux = gst::ElementFactory::make("rtpmux", None)?;
        rtpmux.set_property("ssrc", rtx_ssrc);
        rtpmux.add_pad(&gst::GhostPad::new(Some("vsink"), gst::PadDirection::Sink))?;
        rtpmux.add_pad(&gst::GhostPad::new(Some("asink"), gst::PadDirection::Sink))?;
        let video_sink = rtpmux.static_pad("vsink").unwrap();
        let audio_sink = rtpmux.static_pad("asink").unwrap();

        //encryption
        let srtpenc = gst::ElementFactory::make("srtpenc", None)?;
        srtpenc.set_property_from_str("rtcp-cipher", encryption_algorithm.to_gst_str());
        srtpenc.set_property("key", gst::Buffer::from_slice(key));
        srtpenc.add_pad(&gst::GhostPad::new(Some("src"), gst::PadDirection::Src))?;
        let srtp_src = srtpenc.static_pad("src").unwrap();

        //Create a new webrtcbin to connect the pipeline to the WebRTC peer
        let webrtcbin = gst::ElementFactory::make("webrtcbin", None)?;
        webrtcbin.add_pad(&gst::GhostPad::new(Some("sink"), gst::PadDirection::Sink))?;
        let webrtcbin_sink = webrtcbin.static_pad("sink").unwrap();

        let mut sdp = SDPMessage::new();
        sdp.set_connection("IN", "IP4", discord_address, 1, 1);

        let webrtc_desc = WebRTCSessionDescription::new(
            WebRTCSDPType::Offer,
            sdp
        );

        let promise = gst::Promise::with_change_func(|_reply| {
        });

        webrtcbin.emit_by_name::<()>("set-remote-description", &[&webrtc_desc, &promise]);


        //Link encoderpay to rtpmux video sink
        gst::Pad::link(&encoder_pay.static_pad("src").unwrap(), &video_sink)?;
        //Link rtpopuspay to rtpmux audio sink
        gst::Pad::link(&rtpopuspay.static_pad("src").unwrap(), &audio_sink)?;

        //Add elements to the pipeline
        pipeline.add_many(&[
            &ximagesrc, &capsfilter, &videoconvert, &encoder, &encoder_pay,
            &pulsesrc, &audioconvert, &opusenc, &rtpopuspay,
            &rtpmux, &srtpenc,
            &webrtcbin])?;
        //Link video elements
        Element::link_many(&[&ximagesrc, &capsfilter, &videoconvert, &encoder, &encoder_pay])?;
        //Link audio elements
        Element::link_many(&[&pulsesrc, &audioconvert, &opusenc, &rtpopuspay])?;
        //Link rtpmux with the encoder
        Element::link(&rtpmux, &srtpenc)?;
        //Link webrtcbin sink with the rtpmux src
        gst::Pad::link(&srtp_src, &webrtcbin_sink)?;

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