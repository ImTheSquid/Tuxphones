use std::{
    io::Cursor,
};

// use async_std::{channel::Sender, task};
use crate::{pid, xid};
use image::ImageBuffer;
use sysinfo::{PidExt, ProcessExt, SystemExt};
use xcb::{
    res::{ClientIdMask, ClientIdSpec, QueryClientIds},
    x::{
        self, GetGeometry, GetImage,
    },
};

pub struct XServerHandle {
    connection: xcb::Connection,
    /// List of PIDs that are related to Xorg
    xorg_procs: Vec<pid>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl XServerHandle {
    pub fn new() -> Result<Self, xcb::Error> {
        // Connect to the server
        let (conn, _) = xcb::Connection::connect(None)?;

        // Get the current Xorg process to make sure XServer isn't falsely recognizing windows (cached)
        let mut system = sysinfo::System::new();
        system.refresh_processes();
        let xorg_procs = system
            .processes_by_name("Xorg")
            .map(|p| p.pid().as_u32())
            .collect();

        Ok(XServerHandle {
            connection: conn,
            /*cache: HashMap::new(), last_cache_wipe: None,*/ xorg_procs,
        })
    }

    /// Attempts to derive a PID from an XID
    pub fn pid_from_xid(&self, xid: xid) -> Result<Option<pid>, xcb::Error> {
        // Create request
        let cookie = self.connection.send_request(&QueryClientIds {
            specs: &[ClientIdSpec {
                client: xid,
                mask: ClientIdMask::LOCAL_CLIENT_PID,
            }],
        });

        let reply = self.connection.wait_for_reply(cookie)?;

        if let Some(val) = reply.ids().next() {
            return Ok(
                if !val.value().is_empty() && !self.xorg_procs.iter().any(|v| *v == val.value()[0])
                {
                    Some(val.value()[0])
                } else {
                    None
                },
            );
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

        let mut image: ImageBuffer<image::Rgba<u8>, _> = ImageBuffer::from_raw(
            size.width.into(),
            size.height.into(),
            reply.data().to_owned(),
        )
        .unwrap();
        // Convert BGRA to RGBA
        for pixel in image.pixels_mut() {
            pixel.0 = [pixel.0[2], pixel.0[1], pixel.0[0], pixel.0[3]];
        }

        let (width, height) = calculate_aspect_ratio_fit(image.width(), image.height(), 512, 512);

        // Resize image to reasonable thumbnail size
        let image =
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Triangle);
        image.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();

        Ok(buf.into_inner())
    }
}

fn calculate_aspect_ratio_fit(
    src_width: u32,
    src_height: u32,
    max_width: u32,
    max_height: u32,
) -> (u32, u32) {
    let ratio = f64::min(
        max_width as f64 / src_width as f64,
        max_height as f64 / src_height as f64,
    );

    (
        (src_width as f64 * ratio).round() as u32,
        (src_height as f64 * ratio).round() as u32,
    )
}

fn window_size(conn: &xcb::Connection, xid: xid) -> Result<Size, xcb::Error> {
    let cookie = conn.send_request(&GetGeometry {
        drawable: x::Drawable::Window(unsafe { xcb::XidNew::new(xid) }),
    });

    let reply = conn.wait_for_reply(cookie)?;

    Ok(Size {
        width: reply.width(),
        height: reply.height(),
    })
}
