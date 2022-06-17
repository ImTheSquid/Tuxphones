pub mod websocket {
    use std::sync::Arc;
    use std::time::Duration;

    use async_std::{task};
    use async_std::sync::Mutex;
    use async_tungstenite::async_std::{connect_async, ConnectStream};
    use async_tungstenite::tungstenite::Message;
    use async_tungstenite::WebSocketStream;
    use futures_util::{SinkExt};
    use futures_util::stream::{SplitSink, StreamExt};
    use lazy_static::lazy_static;
    use regex::Regex;
    use tracing::{debug, error, info, trace};

    use crate::discord_op::opcodes::*;

    const API_VERSION: u8 = 7;

    lazy_static! {
        static ref OPCODE_REGEX: Regex = Regex::new(r#""op":(?P<op>\d+)"#).unwrap();
    }

    #[derive(Debug)]
    pub struct WebsocketConnection {
        ws_write: SplitSink<WebSocketStream<ConnectStream>, Message>,
    }

    impl Drop for WebsocketConnection {
        fn drop(&mut self) {
            info!("Closing websocket connection");
        }
    }

    impl WebsocketConnection {
        #[tracing::instrument]
        pub async fn new(endpoint: String) -> Result<Arc<Mutex<Self>>, async_tungstenite::tungstenite::Error> {
            //v7 is going to be deprecated according to discord's docs (https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87) but is the one that discord client still use for video streams
            let (ws_stream, response) = connect_async(format!("wss://{}/?v={}", endpoint, API_VERSION)).await?;

            debug!("Connected with response code: {:?}", response.status());

            let (ws_write, ws_read) = ws_stream.split();

            let ws_connection = Arc::new(Mutex::new(Self {
                ws_write
            }));

            let ws_listener = ws_read.for_each({
                let ws_connection = ws_connection.clone();
                move |msg| {
                    let ws_connection = ws_connection.clone();
                    async move {
                        let mut msg = match msg {
                            Ok(ws_msg) => {
                                match ws_msg.to_text() {
                                    Ok(msg) => msg.to_string(),
                                    Err(e) => {
                                        error!("Failed to convert websocket message to text: {:?}", e);
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error reading websocket message: {:?}", e);
                                return;
                            }
                        };

                        //Quick way to patch the opcode to be a string waiting https://github.com/serde-rs/serde/pull/2056 to be merged
                        msg = OPCODE_REGEX.replace(&msg, "\"op\":\"$op\"").to_string();

                        let msg: IncomingWebsocketMessage = match serde_json::from_str(&msg) {
                            Ok(msg) => msg,
                            Err(e) => {
                                trace!("Websocket message: {:?}", msg);
                                error!("Failed to deserialize websocket message: {:?}", e);
                                return;
                            }
                        };

                        trace!("{:?}", msg);

                        match msg {
                            IncomingWebsocketMessage::OpCode2(data) => {}
                            IncomingWebsocketMessage::OpCode6(data) => {}
                            IncomingWebsocketMessage::OpCode8(data) => {
                                debug!("Websocket heartbeat interval: {}", data.heartbeat_interval);
                                task::spawn(async move {
                                    loop {
                                        task::sleep(Duration::from_millis(data.heartbeat_interval)).await;
                                        ws_connection.lock().await.send_heartbeat().await;
                                        debug!("Sent websocket heartbeat");
                                    }
                                });
                            }
                        }
                    }
                }
            });

            //TODO: Figure out if the task is killed when the struct is dropped or if should be killed manually
            let reader = task::spawn(ws_listener);

            Ok(ws_connection)
        }


        /// auth the websocket connection using opcode 0
        /// https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87
        #[tracing::instrument]
        pub async fn auth(&mut self, server_id: String, session_id: String, token: String, user_id: String) -> Result<(), async_tungstenite::tungstenite::Error> {
            let ws_message = OutgoingWebsocketMessage::OpCode0(OpCode0 {
                server_id,
                session_id,
                streams: vec![GatewayStream {
                    stream_type: "video".to_string(),
                    rid: 100,
                    quality: 100,

                    active: None,
                    ssrc: None,
                    rtx_ssrc: None,
                    max_bitrate: None,
                    max_framerate: None,
                    max_resolution: None,
                }],
                token,
                user_id,
                video: true,
            }).to_json();

            trace!("{:?}", ws_message);

            self.ws_write.send(Message::Text(ws_message)).await?;

            Ok(())
        }

        #[tracing::instrument]
        pub async fn send_heartbeat(&mut self) -> Result<(), async_tungstenite::tungstenite::Error> {
            let ws_message = OutgoingWebsocketMessage::OpCode3(OpCode3_6 {
                //TODO: Set to a random value and check if it's the same as the one received
                d: 0,
            }).to_json();

            trace!("Sending heartbeat message: {}", ws_message);

            self.ws_write.send(Message::Text(ws_message)).await?;

            Ok(())
        }
    }
}