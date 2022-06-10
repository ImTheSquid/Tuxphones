use std::{thread, time::Duration};

use tuxphones::pulse::PulseHandle;
use tuxphones::gstreamer::{EncryptionAlgorithm, GstHandle, H264Settings, VideoEncoderType};

fn main() {
    // test_gst();
    // test_pulse();
}

fn test_gst() {
    println!("Hello, world!");
    let mut pipeline = GstHandle::new(
        VideoEncoderType::H264(H264Settings {nvidia_encoder: false}), 0, 30, 345600,
        0, 0, 0,
        "127.0.0.1:25555", EncryptionAlgorithm::aead_aes256_gcm, vec![2, 2, 2]
    ).unwrap();

}

fn test_pulse() {
    println!("Hello, world!");
    let mut handle = PulseHandle::new().expect("Failed!");

    println!("Sinks ====");
    let sinks = handle.get_sinks();
    sinks.into_iter().for_each(|f| {
        println!("{} (IDX: {})", f.name, f.index);
    });

    println!("Audio Applications ====");
    let audio_apps = handle.get_audio_applications();
    audio_apps.into_iter().for_each(|f | {
        println!("{} (PID: {})", f.name, f.pid);
    });

    println!("Setup capture ====");
    handle.setup_audio_capture(Some("alsa_output.pci-0000_0d_00.4.analog-stereo")).expect("Failed to setup capture!");

    println!("Start capture ====");
    handle.start_capture(6).expect("Failed to start capture!");
    thread::sleep(Duration::from_secs(5));

    println!("Stop capture ====");
    handle.stop_capture();
}
