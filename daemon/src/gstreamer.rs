use gst::prelude::*;
use gst::{
    debug_bin_to_dot_data, glib, DebugGraphDetails, Element, PadLinkError, StateChangeError,
    StateChangeSuccess,
};
use tracing::{error, info};

use crate::{
    socket::{StreamResolutionInformation},
    xid,
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
    encoder: Element,
    encoder_type: VideoEncoderType,
}

//Custom drop logic to deinit gstreamer when all handles are dropped
impl Drop for GstHandle {
    fn drop(&mut self) {
        info!("dropping GstHandle");
        // Debug diagram
        let out = debug_bin_to_dot_data(&self.pipeline, DebugGraphDetails::ALL);
        //TODO: Move to logs folder
        std::fs::write("/tmp/tuxphones_gstdrop.dot", out.as_str()).unwrap();

        if let Err(e) = self.pipeline.set_state(gst::State::Null) {
            error!("Failed to stop pipeline: {:?}", e);
        };
    }
}

impl GstHandle {
    pub async fn new(
        encoder_to_use: VideoEncoderType,
        xid: xid,
        resolution: StreamResolutionInformation,
        fps: i32,
    ) -> Result<Self, GstInitializationError> {
        info!("Creating new GstHandle");
        //Create a new GStreamer pipeline
        let pipeline = gst::Pipeline::new(None);

        //--VIDEO--

        //Create a new ximagesrc to get video from the X server
        let ximagesrc = ximageredux::XImageRedux::default();

        let videoscale = gst::ElementFactory::make("videoscale").build()?;

        //Creating a capsfilter to set the resolution and the fps
        let capsfilter = gst::ElementFactory::make("capsfilter").build()?;

        let mut cap = gst::Caps::builder("video/x-raw")
            .field("frame_rate", gst::Fraction::new(fps, 1));

        //If the resolution is specified, add it to the caps
        if resolution.is_fixed {
            cap = cap
                .field("width", resolution.width as i32)
                .field("height", resolution.height as i32);
        };

        capsfilter.set_property(
            "caps",
            &cap.build(),
        );

        // ximagesrc.set_property_from_str("show-pointer", "1");
        //Set xid based on constructor parameter to get video only from the specified X window
        ximagesrc.set_property("xid", xid as u32);

        //Create a new videoconvert to allow encoding of the raw video
        let videoconvert = gst::ElementFactory::make("videoconvert").build()?;

        //Chose encoder based on constructor params
        let encoder = match encoder_to_use {
            VideoEncoderType::H264(settings) => {
                //Use nvidia encoder based on settings
                if settings.nvidia_encoder {
                    let nvh264enc = gst::ElementFactory::make("nvh264enc").build()?;
                    nvh264enc.set_property("gop-size", 2560i32);
                    nvh264enc.set_property_from_str("rc-mode", "cbr-ld-hq");
                    nvh264enc.set_property("zerolatency", true);
                    nvh264enc
                } else {
                    let x264enc = gst::ElementFactory::make("x264enc").build()?;
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
                let vp8enc = gst::ElementFactory::make("vp8enc").build()?;
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
                let vp9enc = gst::ElementFactory::make("vp9enc").build()?;
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
        let audio_capsfilter = gst::ElementFactory::make("capsfilter").build()?;

        let cap = gst::Caps::builder("audio/x-raw")
            .field("channels", 2)
            .field("rate", 48000);

        audio_capsfilter.set_property(
            "caps",
            &cap.build(),
        );

        //Create a new pulsesrc to get audio from the PulseAudio server
        let pulsesrc = gst::ElementFactory::make("pulsesrc").build()?;
        //Set the audio device based on constructor parameter (should be the sink of the audio application)
        pulsesrc.set_property_from_str("device", "tuxphones.monitor");

        //Create a new audioconvert to allow encoding of the raw audio
        let audioconvert = gst::ElementFactory::make("audioconvert").build()?;
        //Encoder for the raw audio to opus
        let opusenc = gst::ElementFactory::make("opusenc").build()?;
        opusenc.set_property("bitrate", 32000i32);
        opusenc.set_property_from_str("bitrate-type", "cbr");
        opusenc.set_property("inband-fec", true);
        opusenc.set_property("packet-loss-percentage", 50);

        //TODO: --DESTINATION--

        //queues
        let video_encoder_queue = gst::ElementFactory::make("queue").build()?;
        let audio_encoder_queue = gst::ElementFactory::make("queue").build()?;
        let video_webrtc_queue = gst::ElementFactory::make("queue").build()?;
        let audio_webrtc_queue = gst::ElementFactory::make("queue").build()?;

        //Add elements to the pipeline
        pipeline.add_many(&[
            ximagesrc.upcast_ref::<Element>(),
            &videoscale,
            &capsfilter,
            &videoconvert,
            &encoder,
            &video_encoder_queue,
            &video_webrtc_queue,
            &pulsesrc,
            &audioconvert,
            &audio_capsfilter,
            &opusenc,
            &audio_encoder_queue,
            &audio_webrtc_queue,
        ])?;

        //Link video elements
        Element::link_many(&[
            ximagesrc.upcast_ref::<Element>(),
            &videoscale,
            &capsfilter,
            &videoconvert,
            &video_encoder_queue,
            &encoder,
            &video_webrtc_queue,
        ])?;

        //Link audio elements
        Element::link_many(&[
            &pulsesrc,
            &audio_capsfilter,
            &audioconvert,
            &audio_encoder_queue,
            &opusenc,
            &audio_webrtc_queue,
        ])?;

        // Debug diagram
        let out = debug_bin_to_dot_data(&pipeline, DebugGraphDetails::ALL);
        //TODO: Move to logs folder
        std::fs::write("/tmp/tuxphones_gst.dot", out.as_str()).unwrap();

        Ok(GstHandle {
            pipeline,
            encoder,
            encoder_type: encoder_to_use,
        })
    }

    pub async fn start(
        &self,
    ) -> Result<StateChangeSuccess, StateChangeError> {
        self.pipeline.set_state(gst::State::Playing)?;

        Ok(StateChangeSuccess::Success)
    }
}
