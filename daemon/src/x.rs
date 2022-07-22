use std::{thread::{self, JoinHandle}, time::Duration, sync::{atomic::{AtomicBool, Ordering}, Arc}, io::Cursor};

use async_std::{channel::Sender, task};
use image::{ImageBuffer, Rgb};
use sysinfo::{SystemExt, ProcessExt, PidExt};
use tracing::error;
use xcb::{res::{QueryClientIds, ClientIdSpec, ClientIdMask}, x::{Event::{ConfigureNotify, PropertyNotify}, self, ChangeWindowAttributes, Cw, EventMask, GetProperty, GetImage, GetGeometry, CreatePixmap}, Xid};
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

    pub fn take_screenshot(&self, xid: xid) -> Result<Vec<u8>, xcb::Error> {
        let size = window_size(&self.connection, xid)?;

        let cookie = self.connection.send_request(&GetImage {
            format: x::ImageFormat::ZPixmap, // jpg
            drawable: xcb::x::Drawable::Window(unsafe { xcb::XidNew::new(xid) }),
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
            plane_mask: u32::MAX,
        });

        let reply = self.connection.wait_for_reply(cookie)?;

        let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());

        let mut image: ImageBuffer<image::Rgba<u8>, _> = image::ImageBuffer::from_raw(size.width.into(), size.height.into(), reply.data().to_owned()).unwrap();
        // Convert BGRA to RGBA
        for pixel in image.pixels_mut() {
            pixel.0 = [pixel.0[2], pixel.0[1], pixel.0[0], pixel.0[3]];
        }

        // Resize image to reasonable thumbnail size
        let image = image::imageops::resize(&image, 512, 512, image::imageops::FilterType::Triangle);
        image.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
        
        Ok(buf.into_inner())
    }
}

pub struct XResizeWatcher {
    run: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub initial_size: Size
}

pub enum XResizeEvent {
    Show,
    Hide,
    Size(Size)
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Size {
    pub width: u16,
    pub height: u16
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

        let initial_size = window_size(&connection, xid)?;

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

        Ok(XResizeWatcher { thread: Some(thread), run, initial_size })
    }
}

fn window_size(conn: &xcb::Connection, xid: xid) -> Result<Size, xcb::Error> {
    let cookie = conn.send_request(&GetGeometry {
        drawable: x::Drawable::Window(unsafe { xcb::XidNew::new(xid) }),
    });

    let reply = conn.wait_for_reply(cookie)?;

    Ok(Size { width: reply.width(), height: reply.height() })
}