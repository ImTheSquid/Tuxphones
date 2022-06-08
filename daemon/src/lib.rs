use std::{sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::Duration, thread};

use pulse::PulseHandle;
use socket::{receive::SocketListenerCommand, send::Application};
use x::XServerHandle;

// Makes sure typing is preserved
use u32 as pid;
use u32 as xid;

pub mod pulse;
pub mod socket;
pub mod x;

pub struct CommandProcessor {
    thread: Option<thread::JoinHandle<()>>
}

impl CommandProcessor {
    pub fn new(receiver: mpsc::Receiver<SocketListenerCommand>, run: Arc<AtomicBool>, sleep_time: Duration) -> Self {
        let thread = thread::spawn(move || {
            let mut pulse = PulseHandle::new().unwrap();
            let x = XServerHandle::new().unwrap();

            loop {
                match receiver.try_recv() {
                    Ok(cmd) => match cmd {
                        SocketListenerCommand::StartStream { ip, port, key, pid, resolution, ssrc } => todo!(),
                        SocketListenerCommand::StopStream => todo!(),
                        SocketListenerCommand::GetInfo => {
                            let apps = pulse.get_audio_applications();
                            match socket::send::application_info(&apps
                                .into_iter()
                                .filter_map(|a| {
                                    return match x.xid_from_pid(a.pid) {
                                        Ok(xid) => if let Some(xid) = xid {
                                            Some((a, xid))
                                        } else {None},
                                        Err(_) => None,
                                    }
                                })
                                .map(|(a, xid)| Application { name: a.name, pid: a.pid, xid })
                                .collect()) {
                                Ok(_) => {},
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
                            if !run.load(Ordering::SeqCst) {
                                println!("Command processor shut down");
                                break;
                            }
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