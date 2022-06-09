use std::{sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, SystemTime, UNIX_EPOCH}, thread};

use pulse::PulseHandle;
use socket::{receive::SocketListenerCommand, send::Application};
use x::XServerHandle;

// Makes sure typing is preserved
use u32 as pid;
use u32 as xid;

mod pulse;
mod socket;
mod x;

pub use socket::receive;

pub struct CommandProcessor {
    thread: Option<thread::JoinHandle<()>>
}

impl CommandProcessor {
    pub fn new(receiver: mpsc::Receiver<SocketListenerCommand>, run: Arc<AtomicBool>, sleep_time: Duration) -> Self {
        let thread = thread::spawn(move || {
            let mut pulse = match PulseHandle::new() {
                Ok(handle) => handle,
                Err(e) => {
                    eprintln!("Pulse error: {}", e);
                    run.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut x = match XServerHandle::new() {
                Ok(handle) => handle,
                Err(e) => {
                    eprintln!("X Server error: {}", e);
                    run.store(false, Ordering::SeqCst);
                    return;
                }
            };

            loop {
                if !run.load(Ordering::SeqCst) {
                    println!("Command processor shut down");
                    break;
                }

                match receiver.try_recv() {
                    Ok(cmd) => match cmd {
                        SocketListenerCommand::StartStream { ip: _, port: _, key: _, pid, resolution: _, ssrc: _ } => {
                            match pulse.setup_audio_capture(None) {
                                Ok(_) => {},
                                Err(e) => {
                                    eprintln!("Failed to setup pulse capture: {}", e);
                                    continue;
                                }
                            }

                            match pulse.start_capture(pid) {
                                Ok(_) => {},
                                Err(e) => {
                                    eprintln!("Failed to start pulse capture: {}", e);
                                    continue;
                                }
                            }
                        },
                        SocketListenerCommand::StopStream => {
                            pulse.stop_capture();
                            pulse.teardown_audio_capture();
                        },
                        SocketListenerCommand::GetInfo => {
                            let time_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                            println!("[GetInfo:{}] Command received", time_start);

                            let apps = pulse.get_audio_applications();
                            let apps = apps
                                .into_iter()
                                .filter_map(|a| {
                                    return match x.xid_from_pid_or_name(a.pid, &a.name) {
                                        Ok(xid) => if let Some(xid) = xid {
                                            Some((a, xid))
                                        } else {None},
                                        Err(e) => {
                                            match e {
                                                xcb::Error::Connection(_) => eprintln!("Connection Error"),
                                                xcb::Error::Protocol(e) => eprintln!("Error finding XID: {:#?}", e),
                                            }
                                            return None;
                                        },
                                    }
                                })
                                .map(|(a, xid)| Application { name: a.name, pid: a.pid, xid })
                                .collect();
                            match socket::send::application_info(&apps) {
                                Ok(_) => println!("[GetInfo:{}] Command processed (applications found: {})", time_start, apps.len()),
                                Err(e) => eprintln!("Failed to send application data: {}", e)
                            }
                        }
                    },
                    Err(e) => match e {
                        mpsc::TryRecvError::Disconnected => {
                            eprintln!("Failed to watch for receiver: {}", e);
                            run.store(false, Ordering::SeqCst);
                            break;
                        },
                        mpsc::TryRecvError::Empty => {
                            thread::sleep(sleep_time);
                        }
                    }
                }
            }
        });

        CommandProcessor { thread: Some(thread) }
    }

    /// Waits for the `CommandProcessor`'s internal thread to join.
    pub fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("Unable to join thread");
        }
    }
}