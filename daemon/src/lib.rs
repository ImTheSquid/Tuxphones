use std::{sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, SystemTime, UNIX_EPOCH}, thread, process::Command};

use gstreamer::GstHandle;
use pulse::PulseHandle;
use socket::{receive::SocketListenerCommand, send::Application};
use sysinfo::{SystemExt, ProcessExt, Process, Pid, PidExt};
use tracing::{error, info};
use x::XServerHandle;

// Makes sure typing is preserved
use u32 as pid;
use u32 as xid;

mod pulse;
mod gstreamer;
mod socket;
mod x;
pub websocket;
mod discord;

pub use socket::receive;

use crate::gstreamer::{H264Settings, VideoEncoderType, EncryptionAlgorithm};

pub struct CommandProcessor {
    thread: Option<thread::JoinHandle<()>>
}

impl CommandProcessor {
    pub fn new(receiver: mpsc::Receiver<SocketListenerCommand>, run: Arc<AtomicBool>, sleep_time: Duration) -> Self {
        let thread = thread::spawn(move || {
            let mut pulse = match PulseHandle::new() {
                Ok(handle) => handle,
                Err(e) => {
                    error!("Pulse error: {}", e);
                    run.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let x = match XServerHandle::new() {
                Ok(handle) => handle,
                Err(e) => {
                    error!("X Server error: {}", e);
                    run.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut gstreamer: Option<GstHandle> = None;

            loop {
                if !run.load(Ordering::SeqCst) {
                    info!("Command processor shut down");
                    break;
                }

                match receiver.try_recv() {
                    Ok(cmd) => {
                        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                        match cmd {
                            SocketListenerCommand::StartStream { 
                                pid,
                                xid,
                                resolution,
                                framerate,
                                server_id,
                                user_id,
                                token,
                                session_id,
                                endpoint
                            } => {
                                info!("[StartStream:{}] Command received", start_time);
                                match pulse.setup_audio_capture(None) {
                                    Ok(_) => {},
                                    Err(e) => {
                                        error!("Failed to setup pulse capture: {}", e);
                                        continue;
                                    }
                                }

                                match pulse.start_capture(pid) {
                                    Ok(_) => {},
                                    Err(e) => {
                                        error!("Failed to start pulse capture: {}", e);
                                        continue;
                                    }
                                }
                                
                                // Quick and drity check to try to detect Nvidia drivers
                                let mut nvidia_encoder = false;
                                if let Some(out) = Command::new("lspci").arg("-nnk").output().ok() {
                                    nvidia_encoder = String::from_utf8_lossy(&out.stdout).contains("nvidia");
                                }

                                todo!("Implement GStreamer with new params");
                                /*gstreamer = match GstHandle::new(
                                    VideoEncoderType::H264(H264Settings { nvidia_encoder }),
                                    xid.into(),
                                    resolution,
                                    frame_rate.into(),
                                    audio_ssrc,
                                    video_ssrc,
                                    rtx_ssrc,
                                    &format!("{}:{}", ip, port),
                                    EncryptionAlgorithm::aead_aes256_gcm,
                                    key
                                ) {
                                    Ok(handle) => Some(handle),
                                    Err(e) => {
                                        error!("GStreamer error: {}", e);
                                        run.store(false, Ordering::SeqCst);
                                        continue;
                                    },
                                };

                                match gstreamer.as_ref().unwrap().start() {
                                    Ok(_) => {},
                                    Err(e) => {
                                        error!("GStreamer startup error: {}", e);
                                        continue;
                                    }
                                }*/

                                info!("[StartStream:{}] Command processed (stream started)", start_time);
                            },
                            SocketListenerCommand::StopStream => {
                                info!("[StopStream:{}] Command received", start_time);

                                // Kill gstreamer instance
                                gstreamer.take();

                                pulse.stop_capture();
                                pulse.teardown_audio_capture();

                                info!("[StopStream:{}] Command processed (stream stopped)", start_time);
                            },
                            SocketListenerCommand::GetInfo { xids } => {
                                info!("[GetInfo:{}] Command received", start_time);

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
                                // Find all processes with given name
                                let mut system = sysinfo::System::new();
                                system.refresh_processes();
                                let processes_with_cmd: Vec<(&Pid, &Process)> = system.processes()
                                    .into_iter()
                                    .filter(|(_, p)| p.cmd().len() > 0)
                                    .collect();
                                
                                for app in &apps {
                                    for (proc_pid, process) in &processes_with_cmd {
                                        let cmd_strings: Vec<&str> = process.cmd()[0].split(' ').collect();
                                        // If the command matches the Pulse application name
                                        if cmd_strings[0].ends_with(&format!("/{}", &app.name)) {
                                            // And the PID of an XID window matches the PID of the found process
                                            if let Some((xid, _)) = xid_pid.iter().find(|(_, pid)| *pid == proc_pid.as_u32()) {
                                                // Push the application and go to the next one
                                                found_applications.push(Application {
                                                    name: app.name.clone(),
                                                    pid: app.pid,
                                                    xid: *xid,
                                                });
                                                break;
                                            }
                                        }
                                    }
                                }

                                match socket::send::application_info(&found_applications) {
                                    Ok(_) => info!("[GetInfo:{}] Command processed (applications found: {})", start_time, found_applications.len()),
                                    Err(e) => error!("Failed to send application data: {}", e)
                                }
                            }
                        }
                    },
                    Err(e) => match e {
                        mpsc::TryRecvError::Disconnected => {
                            error!("Failed to watch for receiver: {}", e);
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
