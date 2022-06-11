use std::{time::Duration, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, process};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use tuxphones::{receive::SocketListener, CommandProcessor};

#[tokio::main]
async fn main() {
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

    let (sender, receiver) = mpsc::channel();

    let mut socket_watcher = match SocketListener::new(sender.clone(), Arc::clone(&run), Duration::from_millis(500)) {
        Ok(s) => s,
        Err(_) => {
            error!("Error creating socket watcher!");
            process::exit(2);
        }
    };

    let mut command_processor = CommandProcessor::new(receiver, Arc::clone(&run), Duration::from_millis(500));

    info!("Daemon started");

    socket_watcher.join();
    command_processor.join();
}
