use async_tungstenite::{tokio::{accept_async, TokioAdapter}, tungstenite::{Error, Message}, WebSocketStream};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use tokio::{task::{JoinHandle, self}, net::{TcpListener, TcpStream}, sync::{mpsc, Mutex}};
use std::{sync::Arc, collections::HashMap, net::SocketAddr, fmt::Display};
use tracing::{error, trace}; 
use crate::{pid, xid};

type ConnectionsArc = Arc<Mutex<HashMap<SocketAddr, SplitSink<WebSocketStream<TokioAdapter<TcpStream>>, Message>>>>;
type CommandSender = mpsc::Sender<SocketListenerCommand>;

/// Listens on a socket for commands
pub struct WebSocket {
    thread: Option<JoinHandle<()>>,
    connections: ConnectionsArc
}

/// Possible errors when creating a `SocketListener`
#[derive(Debug, Clone)]
pub enum SocketListenerCreationError {
    // Unable to bind WebSocket to the specified port
    UnableToBindPort(u16)
}

/// Holds information relating to stream resolution
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub struct StreamResolutionInformation {
    pub width: u16,
    pub height: u16,
    /// Whether or not the stream resolution can change
    pub is_fixed: bool
}

/// Holds RTC ICE information
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub struct IceData {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
    pub ttl: String
}

/// Commands that can be received from the client plugin
#[derive(Deserialize, Debug, PartialEq, Eq)]
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
        ip: String,
        /// ICE Data
        ice: Box<IceData>
    },
    /// Stops the currently-running stream
    StopStream,
    /// Internal stop stream command, notifies client plugin
    StopStreamInternal,
    /// Gets info on which windows can have sound captured
    GetInfo { 
        /// XIDs available to Discord
        xids: Vec<xid> 
    }
}

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

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub struct StreamStop {}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub struct StreamPreview {
    jpg: String
}

impl Display for SocketListenerCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnableToBindPort(port) => f.write_str(&format!("Unable to bind to localhost on port {}", port))
        }
    }
}

impl WebSocket {
    pub async fn new(port: u16, sender: CommandSender) -> Result<WebSocket, SocketListenerCreationError> {
        let connections: ConnectionsArc = Arc::new(Mutex::new(HashMap::new()));

        let listener = match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(l) => l,
            Err(_) => return Err(SocketListenerCreationError::UnableToBindPort(port)),
        };

        // Spawn listener thread to check for commands sent to the socket
        let conn_arc = connections.clone();
        let thread = task::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(Self::accept_connection(stream, conn_arc.clone(), sender.clone()));
            }
        });

        Ok(WebSocket { thread: Some(thread), connections })
    }

    async fn send<T>(&self, data: &T) -> Result<(), Error> 
        where T: ?Sized + Serialize
    {
        for stream in self.connections.lock().await.values_mut() {
            stream.send(Message::Text(serde_json::to_string(data).unwrap())).await?;
        }
        Ok(())
    }

    pub async fn application_info(&self, apps: &Vec<Application>) -> Result<(), Error>  {
        self.send(&ApplicationList { apps }).await
    }

    pub async fn stream_stop_internal(&self) -> Result<(), Error>  {
        self.send(&StreamStop {}).await
    }

    pub async fn stream_preview(&self, data: &Vec<u8>) -> Result<(), Error>  {
        self.send(&StreamPreview { jpg: base64::encode(data) }).await
    }

    /// Kills the WebSocket thread and closes everything up
    pub async fn abort(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.abort();
        }
    }

    async fn handle_connection(stream: TcpStream, connections: ConnectionsArc, sender: CommandSender) -> Result<(), Error> {
        let addr = stream.peer_addr().unwrap();

        let ws_stream = match accept_async(stream).await {
            Ok(s) => s,
            Err(e) => {
                error!("{}", e.to_string());
                return Ok(());
            }
        };

        let (write, mut read) = ws_stream.split();

        connections.lock().await.insert(addr, write);
    
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if msg.is_text() {
                trace!("Received command: {}", msg.to_text().unwrap());

                match serde_json::from_str::<SocketListenerCommand>(msg.to_text().unwrap()) {
                    Ok(cmd) => match sender.send(cmd).await {
                        Ok(_) => {},
                        Err(e) => error!("Failed to send command: {}", e)
                    },
                    Err(e) => error!("Failed to deserialize command: {}", e),
                }
            }
        }
    
        Ok(())
    }
    
    async fn accept_connection(stream: TcpStream, connections: ConnectionsArc, sender: CommandSender) {
        let addr = stream.peer_addr().unwrap();
        if let Err(e) = Self::handle_connection(stream, connections.clone(), sender).await {
            match e {
                Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => {
                    connections.lock().await.remove(&addr);
                },
                err => error!("Error processing connection: {}", err),
            }
        }
    }
}

pub mod receive {
    // use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration, env, path::Path, os::unix::net::UnixListener, io::{self, Read}, fs};
    // use serde::Deserialize;
    // use tokio::{sync::mpsc, task::{self, JoinHandle}, time::sleep};
    // use tracing::{error, info, trace};
    // use crate::{pid, xid};

    // /// Listens on a socket for commands
    // pub struct SocketListener {
    //     thread: Option<JoinHandle<()>>
    // }

    // /// Possible errors when creating a `SocketListener`
    // #[derive(Debug, Clone)]
    // pub enum SocketListenerCreationError {
    //     /// The `HOME` environment variable is not defined.
    //     NoRuntimeDir,
    //     /// An error occurred while trying to create the socket.
    //     UnableToCreateSocket,
    //     /// An error occurred while trying to set the socket to non-blocking.
    //     UnableToSetNonBlocking
    // }

    // /// Holds information relating to stream resolution
    // #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    // #[serde(tag = "type")]
    // pub struct StreamResolutionInformation {
    //     pub width: u16,
    //     pub height: u16,
    //     /// Whether or not the stream resolution can change
    //     pub is_fixed: bool
    // }

    // /// Holds RTC ICE information
    // #[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
    // #[serde(tag = "type")]
    // pub struct IceData {
    //     pub urls: Vec<String>,
    //     pub username: String,
    //     pub credential: String,
    //     pub ttl: String
    // }

    // /// Commands that can be received from the client plugin
    // #[derive(Deserialize, Debug, PartialEq, Eq)]
    // #[serde(tag = "type")]
    // pub enum SocketListenerCommand {
    //     /// Starts a new soundshare stream
    //     StartStream { 
    //         /// Pulse PID
    //         pid: pid, 
    //         /// XID
    //         xid: xid, 
    //         /// Target resolution
    //         resolution: StreamResolutionInformation, 
    //         /// Target framerate
    //         framerate: u8, 
    //         /// Server ID
    //         server_id: String,
    //         /// User ID
    //         user_id: String,
    //         /// Voice access token
    //         token: String,
    //         /// Session ID
    //         session_id: String,
    //         /// RTC Connection ID
    //         rtc_connection_id: String,
    //         /// Target endpoint
    //         endpoint: String,
    //         /// Current public IP
    //         ip: String,
    //         /// ICE Data
    //         ice: Box<IceData>
    //     },
    //     /// Stops the currently-running stream
    //     StopStream,
    //     /// Internal stop stream command, notifies client plugin
    //     StopStreamInternal,
    //     /// Gets info on which windows can have sound captured
    //     GetInfo { 
    //         /// XIDs available to Discord
    //         xids: Vec<xid> 
    //     }
    // }

    // impl SocketListener {
    //     pub fn new(sender: mpsc::Sender<SocketListenerCommand>, run: Arc<AtomicBool>, sleep_time: Duration) -> Result<SocketListener, SocketListenerCreationError> {
    //         // Attempt to load env var
    //         let key = match env::var("HOME") {
    //             Ok(val) => val,
    //             Err(_) => return Err(SocketListenerCreationError::NoRuntimeDir)
    //         };
        
    //         let path = Path::new(&key).join(".config").join("tuxphones.sock");

    //         let listener = match UnixListener::bind(&path) {
    //             Ok(sock) => sock,
    //             Err(e) => {
    //                 error!("Failed to create listener: {}", e);
    //                 return Err(SocketListenerCreationError::UnableToCreateSocket)
    //             }
    //         };

    //         // Allows for constant event processing
    //         if let Err(e) = listener.set_nonblocking(true) {
    //             error!("Failed to set listener to non-blocking: {}", e);
    //             return Err(SocketListenerCreationError::UnableToSetNonBlocking);
    //         }

    //         // Spawn listener thread to check for commands sent to the socket
    //         let thread = task::spawn(async move {
    //             for stream in listener.incoming() {
    //                 match stream {
    //                     Ok(mut stream) => {
    //                         let mut buf = String::new();
    //                         match stream.read_to_string(&mut buf) {
    //                             Ok(_) => {}
    //                             Err(e) => {
    //                                 error!("Failed to read socket stream: {}", e);
    //                                 continue;
    //                             }
    //                         }

    //                         trace!("Received command: {}", buf);

    //                         match serde_json::from_str::<SocketListenerCommand>(&buf) {
    //                             Ok(cmd) => match sender.send(cmd).await {
    //                                 Ok(_) => {},
    //                                 Err(e) => error!("Failed to send command: {}", e)
    //                             },
    //                             Err(e) => error!("Failed to deserialize command: {}", e),
    //                         }
    //                     },
    //                     Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
    //                         if !run.load(Ordering::SeqCst) {
    //                             match fs::remove_file(&path) {
    //                                 Ok(()) => info!("Cleaned up socket"),
    //                                 Err(e) => error!("Failed to remove socket: {}", e)
    //                             }
    //                             break;
    //                         }
    //                         sleep(sleep_time).await;
    //                     }
    //                     Err(e) => error!("Failed to get stream: {}", e)
    //                 }
    //             }
    //         });

    //         Ok(SocketListener { thread: Some(thread) })
    //     }

    //     /// Waits for the `SocketListeners`'s internal thread to join.
    //     pub async fn join(&mut self) {
    //         if let Some(thread) = self.thread.take() {
    //             thread.await.expect("Unable to join thread");
    //         }
    //     }
    // }
}

pub mod send {
    // use std::{os::unix::net::UnixStream, env, path::Path, io::Write};
    // use crate::{pid, xid};

    // use serde::Serialize;
    // use tracing::error;

    // #[derive(Serialize)]
    // #[serde(tag = "type")]
    // struct ApplicationList<'a> {
    //     apps: &'a Vec<Application>
    // }

    // #[derive(Serialize, Debug)]
    // #[serde(tag = "type")]
    // pub struct Application {
    //     pub name: String,
    //     pub pid: pid,
    //     pub xid: xid
    // }

    // #[derive(Serialize, Debug)]
    // #[serde(tag = "type")]
    // pub struct StreamStop {}

    // #[derive(Serialize, Debug)]
    // #[serde(tag = "type")]
    // pub struct StreamPreview {
    //     jpg: String
    // }

    // #[derive(Debug)]
    // pub enum SocketError {
    //     ConnectionFailed,
    //     NoRuntimeDir,
    //     NoSocket,
    //     SerializationFailed,
    //     WriteFailed
    // }

    // impl std::fmt::Display for SocketError {
    //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //         f.write_str(match self {
    //             SocketError::ConnectionFailed => "Connection failed",
    //             SocketError::NoRuntimeDir => "No runtime directory",
    //             SocketError::NoSocket => "No foreign socket",
    //             SocketError::SerializationFailed => "Serialization failed",
    //             SocketError::WriteFailed => "Socket write failed",
    //         })
    //     }
    // }

    // fn connect_to_socket() -> Result<UnixStream, SocketError> {
    //     // Attempt to load env var
    //     let key = match env::var("HOME") {
    //         Ok(val) => val,
    //         Err(_) => return Err(SocketError::NoRuntimeDir)
    //     };
    
    //     let path = Path::new(&key).join(".config").join("tuxphonesjs.sock");

    //     if !path.exists() {
    //         return Err(SocketError::NoSocket);
    //     }

    //     match UnixStream::connect(&path) {
    //         Ok(s) => Ok(s),
    //         Err(e) => {
    //             error!("Socket connection error: {}", e);
    //             Err(SocketError::ConnectionFailed)
    //         },
    //     }
    // }

    // fn write_socket<T>(data: &T) -> Result<(), SocketError>
    //     where T: ?Sized + Serialize
    // {
    //     let mut socket = connect_to_socket()?;
    //     match serde_json::to_string(data) {
    //         Ok(s) => match socket.write(s.as_bytes()) {
    //             Ok(_) => Ok(()),
    //             Err(e) => {
    //                 error!("Write failed: {}", e);
    //                 Err(SocketError::WriteFailed)
    //             },
    //         },
    //         Err(e) => {
    //             error!("Serialization failed: {}", e);
    //             Err(SocketError::SerializationFailed)
    //         },
    //     }
    // }

    // pub fn application_info(apps: &Vec<Application>) -> Result<(), SocketError> {
    //     write_socket(&ApplicationList { apps })
    // }

    // pub fn stream_stop_internal() -> Result<(), SocketError> {
    //     write_socket(&StreamStop {})
    // }

    // pub fn stream_preview(data: &Vec<u8>) -> Result<(), SocketError> {
    //     write_socket(&StreamPreview { jpg: base64::encode(data) })
    // }
}