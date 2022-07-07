use sysinfo::{SystemExt, ProcessExt, PidExt};
use xcb::res::{QueryClientIds, ClientIdSpec, ClientIdMask};
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