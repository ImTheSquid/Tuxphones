use sysinfo::{SystemExt, ProcessExt, PidExt};
use xcb::res::{QueryClientIds, ClientIdSpec, ClientIdMask};
use crate::{pid, xid};

pub struct XServerHandle {
    connection: xcb::Connection,
    /// Stores a cache of pids to xids
    // cache: HashMap<pid, Option<xid>>,
    // last_cache_wipe: Option<SystemTime>,
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
    pub fn pid_from_xid(self: &Self, xid: xid) -> Result<Option<pid>, xcb::Error> {
        // Create request
        let cookie = self.connection.send_request(&QueryClientIds {
            specs: &[ClientIdSpec {
                client: xid,
                mask: ClientIdMask::LOCAL_CLIENT_PID
            }]
        });

        let reply = self.connection.wait_for_reply(cookie)?;

        if let Some(val) = reply.ids().next() {
            return Ok(if val.value().len() > 0 && !self.xorg_procs.iter().any(|v| *v == val.value()[0]) {
                Some(val.value()[0])
            } else {
                None
            });
        }

        Ok(None)
    }

    /*/// Checks the cache for a value and refeshes cache if needed
    fn check_cache(self: &mut Self, pid: pid) -> Option<Option<xid>> {
        if let Some(wipe) = self.last_cache_wipe {
            // If it's been more than 5 minutes since last cache wipe, wipe again
            if SystemTime::now().duration_since(wipe).unwrap_or(Duration::from_secs(10000000)) > Duration::from_secs(5 * 60) {
                println!("PID-XID cache expired");
                self.clear_cache();
            }
        }

        if let Some(xid) = self.cache.get(&pid) {
            return Some(*xid);
        }

        None
    }

    pub fn clear_cache(self: &mut Self) {
        self.cache.clear();
        self.last_cache_wipe = Some(SystemTime::now());
    }

    /// Finds XID from a PID or process name (case sensitive)
    pub fn xid_from_pid_or_name(self: &mut Self, pulse_pid: pid, name: &str) -> Result<Option<xid>, xcb::Error> {
        // Check cache first
        if let Some(xid) = self.check_cache(pulse_pid) {
            return match xid {
                Some(xid) => Ok(Some(xid)),
                None => Ok(None)
            };
        }

        println!("Cache miss for PID {}", pulse_pid);

        // Create request
        let cookie = self.connection.send_request(&QueryClients {});

        let reply = self.connection.wait_for_reply(cookie)?;
        let xids: Vec<xid> = reply.clients().into_iter().map(|c| c.resource_base).collect();

        if let Some(value) = self.find_pid_in_xids(&xids, pulse_pid, pulse_pid) {
            return value;
        }

        // If still haven't found XID, look through processes with a command that match the given name (since Pulse PIDs can be arbitrary) and see if each PID has an associated XID
        let mut system = sysinfo::System::new();
        system.refresh_processes();
        for (pid, process) in system.processes().into_iter().filter(|(_, p)| p.cmd().len() > 0) {
            let split: Vec<&str> = process.cmd()[0].split(' ').collect();
            if split[0].ends_with(name) {
                if let Some(value) = self.find_pid_in_xids(&xids, pid.as_u32(), pulse_pid) {
                    return value;
                }
            }
        }

        Ok(None)
    }

    fn find_pid_in_xids(self: &mut Self, xids: &Vec<u32>, pid: pid, pulse_pid: pid) -> Option<Result<Option<u32>, xcb::Error>> {
        for xid in xids {
            if let Some(res) = self.pid_from_xid(*xid).ok() {
                if res.is_some() && res.unwrap() == pid {
                    println!("Cache store {}:{}", pulse_pid, *xid);
                    self.cache.insert(pulse_pid, Some(*xid));
                    return Some(Ok(Some(*xid)));
                }
            }
        }

        self.cache.insert(pulse_pid, None);
        None
    }*/
}