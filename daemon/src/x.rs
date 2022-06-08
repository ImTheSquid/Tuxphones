use xcb::res::{QueryClientIds, ClientIdSpec, ClientIdMask};
use crate::{pid, xid};

pub struct XServerHandle {
    connection: xcb::Connection
}

impl XServerHandle {
    pub fn new() -> Result<Self, xcb::Error> {
        // Connect to the server
        let (conn, _) = xcb::Connection::connect(None)?;

        Ok(XServerHandle { connection: conn })
    }

    pub fn xid_from_pid(self: &Self, pid: pid) -> Result<Option<xid>, xcb::Error> {
        // Create request
        let cookie = self.connection.send_request(&QueryClientIds {
            specs: &[ClientIdSpec {
                client: pid,
                mask: ClientIdMask::LOCAL_CLIENT_PID
            }]
        });

        let reply = self.connection.wait_for_reply(cookie)?;

        if let Some(val) = reply.ids().next() {
            return Ok(if val.value().len() > 0 {
                Some(val.value()[0])
            } else {
                None
            })
        }

        Ok(None)
    }
}