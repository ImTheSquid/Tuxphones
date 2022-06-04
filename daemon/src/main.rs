use std::{thread, time::Duration};

use tuxphones::pulse::PulseHandle;

fn main() {
    // test_pulse();
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
