use std::{thread, time::Duration, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, process};

use tuxphones::{pulse::PulseHandle, socket::receive::SocketListener};

fn main() {
    let run = Arc::new(AtomicBool::new(true));
    let r= Arc::clone(&run);

    // Ctrl+C handling
    match ctrlc::set_handler(move || {
        println!("Interrupt!");
        r.store(false, Ordering::SeqCst);
    }) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Failed to set interrupt handler! {}", e);
            process::exit(1);
        }
    }

    let (sender, receiver) = mpsc::channel();

    let mut socket_watcher = match SocketListener::new(sender.clone(), Arc::clone(&run), Duration::from_secs(2)) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error creating socket watcher!");
            process::exit(2);
        }
    };

    socket_watcher.join();

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
