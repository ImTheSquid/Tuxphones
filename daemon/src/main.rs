use std::{fs, panic, process, sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
}, time::Duration};
use std::collections::HashMap;
use std::io::Write;
use std::str::FromStr;

use tokio::{
    signal::{ctrl_c, unix::SignalKind},
    sync::mpsc,
};
use tokio::sync::Mutex;
use tracing::{error, info, Level};
use tracing_log::LogTracer;
use tracing_subscriber::{filter, Layer};
use tracing_subscriber::filter::FilterExt;
use tracing_subscriber::layer::SubscriberExt;

use tuxphones::{CommandProcessor, socket::WebSocket};

#[tokio::main]
async fn main() {
    initialize_logging();

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

fn initialize_logging() {
    // "TUX_LOG=category=level,category=level..."
    // "TUX_FILE_LOG=category=level,category=level..."
    // "TUX_FILE_PATH=/path/to/folder"
    // TUX_FILE_PATH can include {date} and {time} which will be replaced with the current date and time and {pid} which will be replaced with the current process id
    //Where category is one of:
    //  - "tuxphones" for the daemon itself
    //  - "tuxphones::websocket" for the websocket
    //  - "tuxphones::command" for the command processor
    //  - "tuxphones::sdp" for loading SDP
    //And level is one of:
    //  - 0 = disabled
    //  - 1 = "error"
    //  - 2 = "debug"
    //  - 3 = "info"
    //  - 4 = "debug"
    //  - 5 = "trace"
    let console_categories: HashMap<String, i8> = std::env::var("TUX_LOG")
        .unwrap_or_else(|_| "tuxphones=3".to_string())
        .split(',')
        .map(|s| {
            let mut split = s.split('=');
            (split.next().unwrap().to_string(), split.next().unwrap().parse().unwrap())
        })
        .collect();

    let file_categories: HashMap<String, i8> = std::env::var("TUX_FILE_LOG")
        .unwrap_or_else(|_| "tuxphones::sdp=5".to_string())
        .split(',')
        .map(|s| {
            let mut split = s.split('=');
            (split.next().unwrap().to_string(), split.next().unwrap().parse().unwrap())
        })
        .collect();

    let mut file_subscribers = Vec::new();

    if !file_categories.is_empty() {
        let file_path = std::env::var("TUX_FILE_PATH").unwrap_or_else(|_| "/tmp/tuxphones-{date}-{time}-{pid}".to_string());

        //replace {date}, {time}, {pid} in the file path with the current date, time, process
        let file_path = file_path
            .replace("{date}", &chrono::Local::now().format("%Y:%m:%d").to_string())
            .replace("{time}", &chrono::Local::now().format("%H:%M:%S").to_string())
            .replace("{pid}", &process::id().to_string());

        //Create the folder file_path and if already exist add a -1, -2, -3, etc to the end of the folder name
        let mut file_path = std::path::PathBuf::from(file_path);
        let mut i = 0;
        while file_path.exists() {
            i += 1;
            file_path = file_path.with_file_name(format!("{}-{}", file_path.file_name().unwrap().to_str().unwrap(), i));
        }

        match fs::create_dir_all(&file_path) {
            Ok(_) => {
                if std::env::var("TUX_OPEN_LOG_ON_START").unwrap_or_else(|_| "false".to_string()).parse::<bool>().unwrap() {
                    match process::Command::new("xdg-open").arg(&file_path).spawn() {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Failed to open folder! {}", e);
                        }
                    }
                }
                //For each file_category create a file and a tracing_subscriber for it
                for (category, level) in file_categories {
                    let file = std::fs::File::create(format!("{}/{}.log", file_path.to_str().unwrap(), category)).unwrap();
                    //TODO: Figure out why the non_blocking wrapper doesn't work
                    //let (non_blocking, _guard) = tracing_appender::non_blocking(file);
                    let sdp_log = tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_writer(file)
                        .with_target(false)
                        .with_filter(filter::filter_fn(move |meta| {
                            meta.target() == category && meta.level() <= &Level::from_str(&level.to_string()).unwrap()
                        }))
                        .boxed();

                    file_subscribers.push(sdp_log);
                }
            }
            Err(_) => {
                eprintln!("Failed to create folder {}, file logging disabled", file_path.to_str().unwrap());
            }
        };
    }

    // Stdout logging
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    let subscriber = tracing_subscriber::registry()
        .with(stdout_log
            .with_filter(filter::filter_fn(move |meta| {
                console_categories.get(meta.target()).map(|level| {
                    meta.level() <= &Level::from_str(&level.to_string()).unwrap()
                }).unwrap_or(false)
            }))
        )
        .with(file_subscribers);
    error!("Logging initialized");

    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to set global logging default subscriber: {}", e);
        }
    }

    LogTracer::init().unwrap();
}