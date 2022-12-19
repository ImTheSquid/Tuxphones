use std::{
    panic,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Mutex;
use tokio::{
    signal::{ctrl_c, unix::SignalKind},
    sync::mpsc,
};
use tracing::{error, info, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;
use tuxphones::{socket::WebSocket, CommandProcessor};

#[tokio::main]
async fn main() {
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
                _ => Level::ERROR,
            };
            gst_level = level.clamp(0, 4);
        }
    }

    // Only set GST_DEBUG if not set already
    if let Err(_) = std::env::var("GST_DEBUG") {
        std::env::set_var("GST_DEBUG", gst_level.to_string());
    }

    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();

    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to set global logging default subscriber: {}", e);
        }
    }

    LogTracer::init().unwrap();

    let run = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&run);

    // Ctrl+C handling
    // match ctrlc::set_handler(move || {
    //     info!("Interrupt!");
    //     r.store(false, Ordering::SeqCst);
    // }) {
    //     Ok(_) => {},
    //     Err(e) => {
    //         error!("Failed to set interrupt handler! {}", e);
    //         process::exit(1);
    //     }
    // }

    let (sender, receiver) = mpsc::channel(1000);

    let socket_watcher: Arc<Mutex<WebSocket>> = match WebSocket::new(9000, sender.clone()).await {
        Ok(s) => Arc::new(Mutex::new(s)),
        Err(_) => {
            error!("Error creating socket watcher!");
            process::exit(2);
        }
    };

    let mut command_processor = CommandProcessor::new(
        receiver,
        sender.clone(),
        Arc::clone(&run),
        Duration::from_millis(500),
        socket_watcher.clone(),
    );

    info!("Daemon started");

    let mut sig = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sig.recv() => {},
        _ = ctrl_c() => {}
    }

    r.store(false, Ordering::SeqCst);

    socket_watcher.lock().await.abort().await;
    command_processor.join().await;
}
