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
    pub credential: String
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
        let mut conn = self.connections.lock().await;
        let mut to_remove = Vec::new();
        for (addr, stream) in conn.iter_mut() {
            match stream.send(Message::Text(serde_json::to_string(data).unwrap())).await {
                Ok(()) => {},
                Err(e) => match e {
                    Error::ConnectionClosed => {to_remove.push(addr.clone());},
                    _ => return Err(e)
                }
            }
        }

        for rm in to_remove {
            conn.remove(&rm);
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