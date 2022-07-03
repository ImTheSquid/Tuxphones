use std::{time::Duration, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, process};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use tuxphones::{receive::SocketListener, CommandProcessor};
use self_update::{cargo_crate_version, errors::Error, Status};

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

    match check_for_updates() {
        Ok(status) => match status {
            Status::UpToDate(_) => info!("Tuxphones is up-to-date!"),
            Status::Updated(new) => info!("Tuxphones updated to {new}!")
        },
        Err(e) => error!("Error fetching update: {e}")
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

fn check_for_updates() -> Result<Status, Error> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("ImTheSquid")
        .repo_name("Tuxphones")
        .bin_name("updated")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    Ok(status)
}
