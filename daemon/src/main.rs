use std::{time::Duration, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, process, panic, env, path::Path, fs};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use tuxphones::{receive::SocketListener, CommandProcessor};

fn main() {
    // Figure out logging level
    let mut log_level = Level::INFO;
    let mut gst_level = 0;
    if let Ok(level) = std::env::var("TUX_LOG") {
        if let Ok(level) = level.parse::<u8>() {
            log_level = match level {
                1 => Level::WARN,
                2 => Level::INFO,
                3 => Level::DEBUG,
                4 => Level::TRACE,
                _ => Level::ERROR
            };
            gst_level = level.clamp(0, 4);
        }
    }

    // Only set GST_DEBUG if not set already
    if let Err(_) = std::env::var("GST_DEBUG") {
        std::env::set_var("GST_DEBUG", gst_level.to_string());
    }

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
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

        // Try to remove socket file
        match env::var("HOME") {
            Ok(val) => {
                let path = Path::new(&val).join(".config").join("tuxphones.sock");
                if let Err(e) = fs::remove_file(&path) {
                    error!("Error removing socket file: {e}");
                }
            },
            Err(e) => error!("Error removing socket file: {e}")
        }

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
