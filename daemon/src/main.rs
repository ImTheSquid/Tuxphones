use std::{time::Duration, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, process, panic};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use tuxphones::{receive::SocketListener, CommandProcessor};

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Failed to set global logging default subscriber: {}", e);
        }
    }

    let run = Arc::new(AtomicBool::new(true));
    let r= Arc::clone(&run);

    // Ctrl+C handling
    match ctrlc::set_handler(move || {
        info!("Interrupt!");
        r.store(false, Ordering::SeqCst);
    }) {
        Ok(_) => {},
        Err(e) => {
            error!("Failed to set interrupt handler! {}", e);
            process::exit(1);
        }
    }

    // Panic handling
    // https://stackoverflow.com/questions/35988775/how-can-i-cause-a-panic-on-a-thread-to-immediately-end-the-main-thread
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    let (sender, receiver) = mpsc::channel();

    let mut socket_watcher = match SocketListener::new(sender.clone(), Arc::clone(&run), Duration::from_millis(500)) {
        Ok(s) => s,
        Err(_) => {
            error!("Error creating socket watcher!");
            process::exit(2);
        }
    };

    let mut command_processor = CommandProcessor::new(receiver, sender.clone(), Arc::clone(&run), Duration::from_millis(500));

    info!("Daemon started");

    socket_watcher.join();
    command_processor.join();
}
