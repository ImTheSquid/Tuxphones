use std::{time::Duration, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, process};
use tuxphones::{receive::SocketListener, CommandProcessor};

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

    let mut socket_watcher = match SocketListener::new(sender.clone(), Arc::clone(&run), Duration::from_millis(500)) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error creating socket watcher!");
            process::exit(2);
        }
    };

    let mut command_processor = CommandProcessor::new(receiver, Arc::clone(&run), Duration::from_millis(500));

    println!("Daemon started");

    socket_watcher.join();
    command_processor.join();
}
