pub mod receive {
    use std::{thread, sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::Duration, env, path::Path, os::unix::net::UnixListener, io::{self, Read}, fs};
    use serde::Deserialize;
    use tracing::{error, info};
    use crate::{pid, xid};

    /// Listens on a socket for commands
    pub struct SocketListener {
        thread: Option<thread::JoinHandle<()>>
    }

    /// Possible errors when creating a `SocketListener`
    #[derive(Debug, Clone)]
    pub enum SocketListenerCreationError {
        /// The `HOME` environment variable is not defined.
        NoRuntimeDir,
        /// An error occured while trying to create the socket.
        UnableToCreateSocket,
        /// An error occured while trying to set the socket to non-blocking.
        UnableToSetNonBlocking
    }

    /// Holds information relating to stream resolution
    #[derive(Deserialize, Debug, Clone)]
    #[serde(tag = "type")]
    pub struct StreamResolutionInformation {
        pub width: u16,
        pub height: u16,
        /// Whether or not the stream resolution can change
        pub is_fixed: bool
    }

    /// Commands that can be received from the client plugin
    #[derive(Deserialize, Debug)]
    #[serde(tag = "type")]
    pub enum SocketListenerCommand {
        /// Starts a new soundshare stream
        StartStream { 
            /// Pulse PID
            pid: pid, 
            /// XID
            xid: xid, 
            /// Target resolution
            resolution: StreamResolutionInformation, 
            /// Target framerate
            framerate: u8, 
            /// Server ID
            server_id: String,
            /// User ID
            user_id: String,
            /// Voice access token
            token: String,
            /// Session ID
            session_id: String,
            /// RTC Connection ID
            rtc_connection_id: String,
            /// Target endpoint
            endpoint: String,
            /// Current public IP
            ip: String
        },
        /// Stops the currently-running stream
        StopStream,
        /// Gets info on which windows can have sound captured
        GetInfo { 
            /// XIDs available to Discord
            xids: Vec<xid> 
        }
    }

    impl SocketListener {
        pub fn new(sender: mpsc::Sender<SocketListenerCommand>, run: Arc<AtomicBool>, sleep_time: Duration) -> Result<SocketListener, SocketListenerCreationError> {
            // Attempt to load env var
            let key = match env::var("HOME") {
                Ok(val) => val,
                Err(_) => return Err(SocketListenerCreationError::NoRuntimeDir)
            };
        
            let path = Path::new(&key).join(".config").join("tuxphones.sock");

            let listener = match UnixListener::bind(&path) {
                Ok(sock) => sock,
                Err(e) => {
                    error!("Failed to create listener: {}", e);
                    return Err(SocketListenerCreationError::UnableToCreateSocket)
                }
            };

            // Allows for constant event processing
            match listener.set_nonblocking(true) {
                Err(e) => {
                    error!("Failed to set listener to non-blocking: {}", e);
                    return Err(SocketListenerCreationError::UnableToSetNonBlocking);
                }
                Ok(()) => {}
            }

            // Spawn listener thread to check for commands sent to the socket
            let thread = thread::spawn(move || {
                for stream in listener.incoming() {
                    match stream {
                        Ok(mut stream) => {
                            let mut buf = String::new();
                            match stream.read_to_string(&mut buf) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("Failed to read socket stream: {}", e);
                                    continue;
                                }
                            }

                            match serde_json::from_str::<SocketListenerCommand>(&buf) {
                                Ok(cmd) => match sender.send(cmd) {
                                    Ok(_) => {},
                                    Err(e) => error!("Failed to send command: {}", e)
                                },
                                Err(e) => error!("Failed to deserialize command: {}", e),
                            }
                        },
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            if !run.load(Ordering::SeqCst) {
                                match fs::remove_file(&path) {
                                    Ok(()) => info!("Cleaned up socket"),
                                    Err(e) => error!("Failed to remove socket: {}", e)
                                }
                                break;
                            }
                            thread::sleep(sleep_time);
                        }
                        Err(e) => error!("Failed to get stream: {}", e)
                    }
                }
            });

            Ok(SocketListener { thread: Some(thread) })
        }

        /// Waits for the `SocketListeners`'s internal thread to join.
        pub fn join(&mut self) {
            if let Some(thread) = self.thread.take() {
                thread.join().expect("Unable to join thread");
            }
        }
    }
}

pub mod send {
    use std::{os::unix::net::UnixStream, env, path::Path, io::Write};
    use crate::{pid, xid};

    use serde::Serialize;
    use tracing::error;

    #[derive(Serialize)]
    #[serde(tag = "type")]
    struct ApplicationList<'a> {
        apps: &'a Vec<Application>
    }

    #[derive(Serialize, Debug)]
    #[serde(tag = "type")]
    pub struct Application {
        pub name: String,
        pub pid: pid,
        pub xid: xid
    }

    pub enum SocketError {
        ConnectionFailed,
        NoRuntimeDir,
        NoSocket,
        SerializationFailed,
        WriteFailed
    }

    impl std::fmt::Display for SocketError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(match self {
                SocketError::ConnectionFailed => "Connection failed",
                SocketError::NoRuntimeDir => "No runtime directory",
                SocketError::NoSocket => "No foreign socket",
                SocketError::SerializationFailed => "Serialization failed",
                SocketError::WriteFailed => "Socket write failed",
            })
        }
    }

    fn connect_to_socket() -> Result<UnixStream, SocketError> {
        // Attempt to load env var
        let key = match env::var("HOME") {
            Ok(val) => val,
            Err(_) => return Err(SocketError::NoRuntimeDir)
        };
    
        let path = Path::new(&key).join(".config").join("tuxphonesjs.sock");

        if !path.exists() {
            return Err(SocketError::NoSocket);
        }

        match UnixStream::connect(&path) {
            Ok(s) => Ok(s),
            Err(e) => {
                error!("Socket connection error: {}", e);
                Err(SocketError::ConnectionFailed)
            },
        }
    }

    fn write_socket<T>(data: &T) -> Result<(), SocketError>
        where T: ?Sized + Serialize
    {
        let mut socket = connect_to_socket()?;
        match serde_json::to_string(data) {
            Ok(s) => match socket.write(s.as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Write failed: {}", e);
                    return Err(SocketError::WriteFailed);
                },
            },
            Err(e) => {
                error!("Serialization failed: {}", e);
                return Err(SocketError::SerializationFailed);
            },
        }
    }

    pub fn application_info(apps: &Vec<Application>) -> Result<(), SocketError> {
        write_socket(&ApplicationList { apps })?;

        Ok(())
    }
}