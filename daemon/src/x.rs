use std::{thread::{self, JoinHandle}, time::Duration, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use async_std::{channel::Sender, task};
use sysinfo::{SystemExt, ProcessExt, PidExt};
use tracing::error;
use xcb::{res::{QueryClientIds, ClientIdSpec, ClientIdMask}, x::{Event::{ConfigureNotify, PropertyNotify}, ChangeProperty, self, ChangeWindowAttributes, GetWindowAttributes, Cw, EventMask, GetProperty}, Xid};
use crate::{pid, xid};

pub struct XServerHandle {
    connection: xcb::Connection,
    /// List of PIDs that are related to Xorg
    xorg_procs: Vec<pid>
}

impl XServerHandle {
    pub fn new() -> Result<Self, xcb::Error> {
        // Connect to the server
        let (conn, _) = xcb::Connection::connect(None)?;

        // Get the current Xorg process to make sure XServer isn't falsely recognizing windows (cached)
        let mut system = sysinfo::System::new();
        system.refresh_processes();
        let xorg_procs = system.processes_by_name("Xorg")
            .into_iter()
            .map(|p| p.pid().as_u32())
            .collect();

        Ok(XServerHandle { connection: conn, /*cache: HashMap::new(), last_cache_wipe: None,*/ xorg_procs })
    }

    /// Attempts to derive a PID from an XID
    pub fn pid_from_xid(&self, xid: xid) -> Result<Option<pid>, xcb::Error> {
        // Create request
        let cookie = self.connection.send_request(&QueryClientIds {
            specs: &[ClientIdSpec {
                client: xid,
                mask: ClientIdMask::LOCAL_CLIENT_PID
            }]
        });

        let reply = self.connection.wait_for_reply(cookie)?;

        if let Some(val) = reply.ids().next() {
            return Ok(if !val.value().is_empty() && !self.xorg_procs.iter().any(|v| *v == val.value()[0]) {
                Some(val.value()[0])
            } else {
                None
            });
        }

        Ok(None)
    }
}

pub struct XResizeWatcher {
    run: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>
}

pub enum XResizeEvent {
    Show,
    Hide,
    Size(Size)
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Size {
    pub width: u32,
    pub height: u32
}

impl Drop for XResizeWatcher {
    fn drop(&mut self) {
        self.run.store(false, Ordering::SeqCst);
        self.thread.take().unwrap().join().expect("Unable to join thread");
    }
}

impl XResizeWatcher {
    pub fn new(xid: xid, on_event: Sender<XResizeEvent>, sleep_time: Duration) -> Result<Self, xcb::Error> {
        // Connect to the server
        let (connection, _) = xcb::Connection::connect(None)?;

        let window = unsafe { xcb::XidNew::new(xid) };
        
        // Subscribe to events
        connection.send_request(&ChangeWindowAttributes {
            window,
            value_list: &[Cw::EventMask(EventMask::STRUCTURE_NOTIFY | EventMask::PROPERTY_CHANGE)]
        });

        let run = Arc::new(AtomicBool::new(true));
        let r = run.clone();

        let thread = thread::spawn(move || {
            let mut last_size: Option<Size> = None;
            while r.load(Ordering::SeqCst) {
                match connection.poll_for_event() {
                    Ok(e) => if let Some(e) = e {
                        if let xcb::Event::X(e) = e {
                            match e {
                                // Listen for size changes
                                ConfigureNotify(e) => {
                                    let size = Size { width: e.width().into(), height: e.height().into() };

                                    // Don't send window relocation events (size stays the same)
                                    if let Some(last_size) = last_size.as_ref() {
                                        if *last_size == size {
                                            continue;
                                        }
                                    } else {
                                        let _ = last_size.insert(size);
                                    }

                                    if let Err(e) = task::block_on(on_event.send(XResizeEvent::Size(size))) {
                                        error!("Failed to send resolution change info: {e}");
                                    }
                                },
                                // Listen for show/hide
                                PropertyNotify(e) => if e.atom().resource_id() == 321 {
                                    let cookie = connection.send_request(&GetProperty {
                                        delete: false,
                                        window,
                                        property: unsafe { xcb::XidNew::new(321) },
                                        r#type: x::ATOM_ATOM,
                                        long_offset: 0,
                                        long_length: 4
                                    });

                                    match connection.wait_for_reply(cookie) {
                                        Ok(res) => {
                                            if let Err(e) = task::block_on(on_event.send(if res.value::<u32>().iter().any(|v| *v == 325) { // Hide
                                                XResizeEvent::Hide
                                            } else if res.value::<u32>().iter().any(|v| *v == 348) { // Show
                                                XResizeEvent::Show
                                            } else { continue; })) {
                                                error!("Failed to send visibility change info: {e}");
                                            }
                                        }
                                        Err(e) => error!("Failed to get window property: {e}")
                                    }
                                },
                                _ => {}
                            }
                        }
                    } else {
                        thread::sleep(sleep_time);
                    },
                    Err(e) => {
                        error!("Failed to poll for X event: {e}");
                        thread::sleep(sleep_time);
                    }
                }
            }
        });

        Ok(XResizeWatcher { thread: Some(thread), run })
    }
}