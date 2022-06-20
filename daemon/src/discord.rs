pub mod websocket {
    use std::borrow::Borrow;
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
    use crate::gstreamer::EncryptionAlgorithm;

    const API_VERSION: u8 = 7;
    const MAX_BITRATE: u32 = 80000000;

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
        pub async fn new(
            endpoint: String,
            max_framerate: u8,
            max_resolution: GatewayResolution,
            rtc_connection_id: String,
            ip: String
        ) -> Result<Arc<Mutex<Self>>, async_tungstenite::tungstenite::Error> {
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
                    let ws_write_arc = ws_connection.clone();

                    // Clone Strings to move across threads
                    let endpoint = endpoint.clone();
                    let rtc_connection_id = rtc_connection_id.clone();
                    let ip = ip.clone();
                    let max_resolution = max_resolution.clone();
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

                        let nonce = Arc::new(Mutex::new(None));

                        match msg {
                            IncomingWebsocketMessage::OpCode2(data) => {
                                if !data.modes.contains(&EncryptionAlgorithm::aead_aes256_gcm.to_gst_str().to_string()) {
                                    panic!("No supported encryption mode!");
                                }

                                ws_write_arc.lock().await.send_stream_information(
                                    data.ssrc, 
                                    data.streams[0].rtx_ssrc.unwrap(), 
                                    data.streams[0].ssrc.unwrap(),
                                    max_resolution, 
                                    max_framerate, 
                                    data.port, 
                                    rtc_connection_id, 
                                    endpoint, 
                                    ip
                                ).await.expect("Failed to send stream information");
                            }
                            IncomingWebsocketMessage::OpCode6(data) => {
                                if let Some(nonce) = nonce.lock().await.as_ref() {
                                    if *nonce != data.d {
                                        error!("Heartbeat nonce values didn't match!");
                                    }
                                }
                            }
                            IncomingWebsocketMessage::OpCode8(data) => {
                                debug!("Websocket heartbeat interval: {}", data.heartbeat_interval);
                                task::spawn(async move {
                                    loop {
                                        task::sleep(Duration::from_millis(data.heartbeat_interval)).await;
                                        // TODO: Better error handling
                                        let _ = nonce.lock().await.insert(ws_write_arc.lock().await.send_heartbeat().await.expect("Failed to send heartbeat"));
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
                    rid: "100".to_string(),
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
        pub async fn send_heartbeat(&mut self) -> Result<u64, async_tungstenite::tungstenite::Error> {
            let nonce = rand::random();
            let ws_message = OutgoingWebsocketMessage::OpCode3(OpCode3_6 {
                d: nonce,
            }).to_json();

            trace!("Sending heartbeat message: {}", ws_message);

            self.ws_write.send(Message::Text(ws_message)).await?;

            Ok(nonce)
        }

        #[tracing::instrument]
        async fn send_partial_stream_information(
            &mut self,
            audio_ssrc: u32,
            rtx_ssrc: u32,
            video_ssrc: u32,
            max_resolution: GatewayResolution,
            max_framerate: u8,
            active: bool
        ) -> Result<(), async_tungstenite::tungstenite::Error> {
            let ws12 = OutgoingWebsocketMessage::OpCode12(OpCode12 { 
                audio_ssrc, 
                rtx_ssrc, 
                video_ssrc, 
                streams: vec![
                    GatewayStream { 
                        stream_type: "video".to_string(), 
                        rid: "100".to_string(), 
                        quality: 100, 
                        active: Some(active), 
                        ssrc: Some(audio_ssrc), 
                        rtx_ssrc: Some(rtx_ssrc), 
                        max_bitrate: Some(MAX_BITRATE), 
                        max_framerate: Some(max_framerate), 
                        max_resolution: Some(max_resolution) 
                    }
                ] 
            }).to_json();

            self.ws_write.send(Message::Text(ws12)).await?;
            Ok(())
        }

        #[tracing::instrument]
        async fn send_stream_information(
            &mut self, 
            audio_ssrc: u32, 
            rtx_ssrc: u32, 
            video_ssrc: u32,
            max_resolution: GatewayResolution,
            max_framerate: u8,
            port: u16,
            rtc_connection_id: String,
            endpoint: String,
            ip: String
        ) -> Result<(), async_tungstenite::tungstenite::Error> {
            self.send_partial_stream_information(
                audio_ssrc,
                rtx_ssrc,
                video_ssrc,
                max_resolution.clone(),
                max_framerate,
                false
            ).await?;

            let ws1 = OutgoingWebsocketMessage::OpCode1(OpCode1 { 
                address: ip.clone(), 
                port, 
                experiments: vec![], 
                mode: EncryptionAlgorithm::aead_aes256_gcm, 
                protocol: "udp".to_string(), 
                rtc_connection_id, 
                codecs: vec![
                    GatewayCodec {
                        name: "H264".to_string(),
                        codec_type: PayloadType::Video,
                        priority: 1000,
                        payload_type: 101,
                        rtx_payload_type: Some(102)
                    },
                    GatewayCodec {
                        name: "opus".to_string(),
                        codec_type: PayloadType::Audio,
                        priority: 1000,
                        payload_type: 120,
                        rtx_payload_type: None
                    }
                ], 
                data: OpCode1Data { 
                    address: ip, 
                    mode: EncryptionAlgorithm::aead_aes256_gcm, 
                    port
                }
            }).to_json();

            self.ws_write.send(Message::Text(ws1)).await?;

            // TODO: clean this up
            self.send_partial_stream_information(
                audio_ssrc,
                rtx_ssrc,
                video_ssrc,
                max_resolution.clone(),
                max_framerate,
                false
            ).await?;

            self.send_partial_stream_information(
                audio_ssrc,
                rtx_ssrc,
                video_ssrc,
                max_resolution,
                max_framerate,
                true
            ).await?;

            Ok(())
        }
    }
}