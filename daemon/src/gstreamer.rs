use std::sync::Mutex;

use gst::{Element, glib, StateChangeError, StateChangeSuccess};
use gst::prelude::*;
use gst_sdp::SDPMessage;
use gst_webrtc::{WebRTCSDPType, WebRTCSessionDescription};
use once_cell::sync::Lazy;

//Gstreamer handles count to prevent deinitialization of gstreamer
static HANDLES_COUNT: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));

#[derive(Debug)]
pub enum GstInitializationError {
    Init(glib::Error),
    Element(glib::BoolError),
}

pub struct H264Settings {
    pub nvidia_encoder: bool,
}

impl H264Settings {}

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

pub struct GstHandle {
    pipeline: gst::Pipeline,
    webrtcbin: Element,
    encoder: Element,
    ximagesrc: Element,
    pulsesrc: Element
}

//Custom drop logic to deinit gstreamer when all handles are dropped
impl Drop for GstHandle {
    fn drop(&mut self) {
        let mut handles_count = HANDLES_COUNT.lock().unwrap();

        match self.pipeline.set_state(gst::State::Null) {
            Err(e) => {
                println!("Failed to stop pipeline: {:?}", e);
            }
            _ => {}
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
    pub fn new(encoder_to_use: VideoEncoderType, audio_source: &str, audio_ssrc: u32, xid: u64, video_ssrc: u32, discord_address: &str) -> Result<Self, GstInitializationError> {
        gst::init()?;
        *HANDLES_COUNT.lock().unwrap() += 1;

        //Create a new GStreamer pipeline
        let pipeline = gst::Pipeline::new(None);

        //--VIDEO--

        //Create a new ximagesrc to get video from the X server
        let ximagesrc = gst::ElementFactory::make("ximagesrc", None)?;

        ximagesrc.set_property_from_str("show-pointer", "1");
        //Set xid based on constructor parameter to get video only from the specified X window
        ximagesrc.set_property("xid", xid);

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

        encoder_pay.set_property("ssrc", video_ssrc)


        //--AUDIO--

        //Create a new pulsesrc to get audio from the PulseAudio server
        let pulsesrc = gst::ElementFactory::make("pulsesrc", None)?;
        //Set the audio device based on constructor parameter (should be the sink of the audio application)
        pulsesrc.set_property_from_str("device", audio_source);

        //Create a new audioconvert to allow encoding of the raw audio
        let audioconvert = gst::ElementFactory::make("audioconvert", None)?;
        //Encoder for the raw audio to opus
        let opusenc = gst::ElementFactory::make("opusenc", None)?;
        //Opus encapsulator for rtp
        let rtpopuspay = gst::ElementFactory::make("rtpopuspay", None)?;
        rtpopuspay.set_property("ssrc", audio_ssrc);


        //--DESTINATION--

        //Create a new webrtcbin to connect the pipeline to the WebRTC peer
        let webrtcbin = gst::ElementFactory::make("webrtcbin", None)?;

        let mut sdp = SDPMessage::new();
        sdp.set_connection("IN", "IP4", discord_address, 1, 1);

        let webrtc_desc = WebRTCSessionDescription::new(
            WebRTCSDPType::Offer,
            sdp
        );

        let promise = gst::Promise::with_change_func(|_reply| {
        });

        webrtcbin.emit_by_name::<()>("set-remote-description", &[&webrtc_desc, &promise]);

        //--COMMON--

        let queue = gst::ElementFactory::make("queue", None)?;


        pipeline.add_many(&[&ximagesrc, &videoconvert, &encoder, &encoder_pay, &webrtcbin])?;
        Element::link_many(&[&ximagesrc, &videoconvert, &encoder, &encoder_pay, &webrtcbin])?;

        Ok(GstHandle {
            pipeline,
            webrtcbin,
            encoder,
            ximagesrc,
            pulsesrc,
        })
    }

    pub fn change_audio_source(&self, audio_device: &str) {
        self.pulsesrc.set_property_from_str("device", audio_device);
    }

    pub fn change_video_source(&self, xid: u64) {
        self.ximagesrc.set_property("xid", xid);
    }

    pub fn start(&self) -> Result<StateChangeSuccess, StateChangeError> {
        self.pipeline.set_state(gst::State::Playing)
    }
}