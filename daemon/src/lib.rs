use std::{sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, SystemTime, UNIX_EPOCH}, thread};

use pulse::PulseHandle;
use socket::{receive::SocketListenerCommand, send::Application};
use sysinfo::{SystemExt, ProcessExt, Process, Pid, PidExt};
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

            let x = match XServerHandle::new() {
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
                    Ok(cmd) => {
                        let time_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                        match cmd {
                            SocketListenerCommand::StartStream { ip: _, port: _, key: _, pid, resolution: _, frame_rate: _, ssrc: _ } => {
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
                            SocketListenerCommand::GetInfo { xids } => {
                                println!("[GetInfo:{}] Command received", time_start);

                                // Find all PIDs of given XIDs
                                let xid_pid: Vec<(xid, pid)> = xids
                                    .into_iter()
                                    .filter_map(|xid| {
                                        if let Some(Some(pid)) = x.pid_from_xid(xid).ok() {
                                            return Some((xid, pid));
                                        }

                                        None
                                    })
                                    .collect();

                                // Do initial matching against returned Pulse PIDs
                                let mut apps = pulse.get_audio_applications();
                                let mut found_applications = vec![];
                                for (xid, pid) in &xid_pid {
                                    if let Some(idx) = apps.iter().position(|app| app.pid == *pid) {
                                        let app = apps.remove(idx);
                                        found_applications.push(Application {
                                            name: app.name,
                                            pid: *pid,
                                            xid: *xid,
                                        });
                                    }   
                                }

                                // If there are more Pulse applications to resolve, lookup process name and try to find pair with given PID for XID
                                // Find all processes with given case-insensitive name
                                let mut system = sysinfo::System::new();
                                system.refresh_processes();
                                let processes_with_cmd: Vec<(&Pid, &Process)> = system.processes()
                                    .into_iter()
                                    .filter(|(_, p)| p.cmd().len() > 0)
                                    .collect();
                                
                                for app in &apps {
                                    for (proc_pid, process) in &processes_with_cmd {
                                        let split: Vec<&str> = process.cmd()[0].split(' ').collect();
                                        if split[0].ends_with(&format!("/{}", &app.name)) {
                                            if let Some((xid, _)) = xid_pid.iter().find(|(_, pid)| *pid == proc_pid.as_u32()) {
                                                found_applications.push(Application {
                                                    name: app.name.clone(),
                                                    pid: app.pid,
                                                    xid: *xid,
                                                });
                                            }
                                        }
                                    }
                                }

                                match socket::send::application_info(&found_applications) {
                                    Ok(_) => println!("[GetInfo:{}] Command processed (applications found: {})", time_start, found_applications.len()),
                                    Err(e) => eprintln!("Failed to send application data: {}", e)
                                }
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