use std::{
    panic,
    process,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::{
    signal::{ctrl_c, unix::SignalKind},
    sync::mpsc,
};
use tokio::sync::Mutex;
use tracing::{error, info, Level};
use tracing::level_filters::LevelFilter;
use tracing_log::LogTracer;
use tracing_subscriber::{filter, Layer};
use tracing_subscriber::layer::SubscriberExt;

use tuxphones::{CommandProcessor, socket::WebSocket};

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
    if std::env::var("GST_DEBUG").is_err() {
        std::env::set_var("GST_DEBUG", gst_level.to_string());
    }

    //Filter to show only untargeted logs
    let tuxphones_target_filter = filter::filter_fn(|meta| {
        meta.target() == "tuxphones"
    });

    // Stdout logging
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    // Generic log file
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::fs::File::create("tux.log").unwrap());
    let generic_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_filter(tuxphones_target_filter.clone());

    // SDP log file
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::fs::File::create("sdp.log").unwrap());
    let sdp_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_target(false)
        .with_filter(filter::filter_fn(|meta| {
            meta.target() == "sdp"
        }));

    let subscriber = tracing_subscriber::registry()
        .with(stdout_log
            .with_filter(
                LevelFilter::from_level(log_level)
            )
            .with_filter(tuxphones_target_filter)
        )
        .with(generic_log)
        .with(sdp_log);

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
