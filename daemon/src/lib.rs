use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, self}};

use sysinfo::{Pid, PidExt, Process, ProcessExt, SystemExt};
use tracing::{error, info};

use pulse::PulseHandle;
use socket::{SocketListenerCommand, Application, WebSocket};
pub use socket::receive;
use x::XServerHandle;
// Makes sure typing is preserved
use u32 as pid;
use u32 as xid;

use crate::{discord::websocket::{ToGst, WebsocketConnection}, x::XResizeWatcher};
use crate::gstreamer::{GstHandle, H264Settings, ToWs, VideoEncoderType};

use tokio::{sync::{mpsc::{self, Receiver, Sender, channel}, Mutex}, time::sleep};

mod pulse;
mod gstreamer;
pub mod socket;
mod x;
mod discord;
mod discord_op;

pub struct CommandProcessor {
    thread: Option<tokio::task::JoinHandle<()>>,
}

impl CommandProcessor {
    pub fn new(mut receiver: mpsc::Receiver<SocketListenerCommand>, ws_sender: mpsc::Sender<SocketListenerCommand>, run: Arc<AtomicBool>, sleep_time: Duration, websocket: Arc<Mutex<WebSocket>>) -> Self {
        let thread = tokio::spawn(async move {
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

            let mut last_stream_preview: Option<time::Instant> = None;
            let mut current_xid = None;

            let mut gst_is_loaded = false;

            let mut resize_watcher = None;
            let mut ws: Option<WebsocketConnection> = None;
            let mut stream = None;

            loop {
                if !run.load(Ordering::SeqCst) {
                    // Kill websocket if still running
                    ws.take();
                    stream.take();
                    current_xid.take();
                    if gst_is_loaded {
                        unsafe {gst::deinit();}
                    }
                    info!("Command processor shut down");
                    break;
                }

                match receiver.try_recv() {
                    Ok(cmd) => {
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
                                rtc_connection_id,
                                endpoint,
                                ip,
                                ice
                            } => {
                                info!("[StartStream] Command received");
                                match pulse.setup_audio_capture(None) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("Failed to setup pulse capture: {}", e);
                                        continue;
                                    }
                                }

                                match pulse.start_capture(pid) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("Failed to start pulse capture: {}", e);
                                        continue;
                                    }
                                }

                                let _ = current_xid.insert(xid);

                                let (to_ws_tx, from_gst_rx): (Sender<ToWs>, Receiver<ToWs>) = channel(10);
                                let (to_gst_tx, from_ws_rx): (Sender<ToGst>, Receiver<ToGst>) = channel(10);
                                let (gst_resize_tx, gst_resize_rx) = channel(10);

                                let _ = resize_watcher.insert(match XResizeWatcher::new(xid, gst_resize_tx, Duration::from_secs(1)) {
                                    Ok(watcher) => watcher,
                                    Err(e) => {
                                        error!("Failed to start resize watcher: {}", e);
                                        continue;
                                    }
                                });

                                // Quick and drity check to try to detect Nvidia drivers
                                // TODO: Find a better way to do this
                                //let nvidia_encoder = if let Ok(out) = Command::new("lspci").arg("-nnk").output() {
                                //     String::from_utf8_lossy(&out.stdout).contains("nvidia")
                                //} else { false };

                                if !gst_is_loaded {
                                    gst_is_loaded = true;
                                    gst::init().expect("Failed to intialize gstreamer");
                                }

                                let gst = GstHandle::new(
                                    VideoEncoderType::H264(H264Settings{nvidia_encoder: false}),
                                    xid,
                                    resolution.clone(),
                                    framerate.into(),
                                    *ice,
                                ).await.expect("Failed to initialize gstreamer pipeline");
                                gst.start(to_ws_tx, from_ws_rx).await.expect("Failed to start stream");

                                let _ = stream.insert(gst);

                                ws = match WebsocketConnection::new(
                                    endpoint,
                                    framerate,
                                    resolution,
                                    rtc_connection_id,
                                    ip,
                                    server_id,
                                    session_id,
                                    token,
                                    user_id,
                                    from_gst_rx,
                                    to_gst_tx,
                                    ws_sender.clone(),
                                ).await {
                                    Ok(ws_handle) => Some(ws_handle),
                                    Err(e) => {
                                        error!("Failed to create websocket connection: {:?}", e);
                                        continue;
                                    }
                                };

                                info!("[StartStream] Command processed (stream started)");
                            }
                            SocketListenerCommand::StopStream | SocketListenerCommand::StopStreamInternal => {
                                info!("[StopStream] Command received");

                                // Kill gstreamer and ws
                                ws.take();
                                stream.take();

                                resize_watcher.take();

                                pulse.stop_capture();
                                pulse.teardown_audio_capture();

                                info!("[StopStream] Command processed (stream stopped)");

                                // If stream was stopped internally, send a notification to the client
                                if cmd == SocketListenerCommand::StopStreamInternal {
                                    if let Err(e) = websocket.lock().await.stream_stop_internal().await {
                                        error!("Failed to notify client of internal stream stop: {:?}", e);
                                    }
                                }
                            }
                            SocketListenerCommand::GetInfo { xids } => {
                                info!("[GetInfo] Command received");

                                // Find all PIDs of given XIDs
                                let xid_pid: Vec<(xid, pid)> = xids
                                    .into_iter()
                                    .filter_map(|xid| {
                                        if let Ok(Some(pid)) = x.pid_from_xid(xid) {
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
                                    .iter()
                                    .filter(|(_, p)| !p.cmd().is_empty())
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

                                match websocket.lock().await.application_info(&found_applications).await {
                                    Ok(_) => info!("[GetInfo] Command processed (applications found: {})", found_applications.len()),
                                    Err(e) => error!("Failed to send application data: {}", e)
                                }
                            }
                        }
                    }
                    Err(e) => match e {
                        mpsc::error::TryRecvError::Disconnected => {
                            error!("Failed to watch for receiver: {}", e);
                            run.store(false, Ordering::SeqCst);
                            break;
                        }
                        mpsc::error::TryRecvError::Empty => {
                            // Check if time to send a stream preview
                            let send_preview = if stream.is_some() {
                                if let Some(last) = last_stream_preview {
                                    if time::Instant::now().duration_since(last) > Duration::from_secs(10 * 60) {
                                        true
                                    } else { false }
                                } else {
                                    true
                                }
                            } else { false };

                            if send_preview {
                                let _ = last_stream_preview.insert(time::Instant::now());
                                info!("Sending stream preview");
                                if let Err(e) = websocket.lock().await.stream_preview(&x.take_screenshot(current_xid.unwrap()).unwrap()).await {
                                    error!("Failed to send stream preview: {}", e);
                                }
                            }

                            sleep(sleep_time).await;
                        }
                    }
                }
            }
        });

        CommandProcessor { thread: Some(thread) }
    }

    /// Waits for the `CommandProcessor`'s internal thread to join.
    pub async fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.await.expect("Unable to join thread");
        }
    }
}
