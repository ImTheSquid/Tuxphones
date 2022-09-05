pub mod websocket {
    use std::borrow::BorrowMut;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use tokio::sync::Mutex;
    use tokio::task;
    use tokio::sync::mpsc;
    use tokio::task::JoinHandle;
    // use async_std::{channel, task};
    // use async_std::sync::Mutex;
    // use async_std::task::JoinHandle;
    use async_tungstenite::tokio::{connect_async, ConnectStream};
    use async_tungstenite::tungstenite::{Message};
    use async_tungstenite::WebSocketStream;
    use futures_util::{SinkExt};
    use futures_util::stream::{SplitSink, StreamExt};
    use lazy_static::lazy_static;
    use rand::Rng;
    use regex::Regex;
    use serde_json::{Number, Value};
    use tokio::time::sleep;
    use tracing::{debug, error, info, trace};

    use crate::discord_op::opcodes::*;
    use crate::gstreamer::{StreamSSRCs, ToWs};
    use crate::receive::{SocketListenerCommand, StreamResolutionInformation};

    const API_VERSION: u8 = 7;
    const MAX_BITRATE: u32 = 1_000_000;

    lazy_static! {
        static ref OPCODE_REGEX: Regex = Regex::new(r#""op":(?P<op>\d+)"#).unwrap();
    }

    type WebSocketWrite = SplitSink<WebSocketStream<ConnectStream>, Message>;

    #[derive(Debug)]
    pub struct WebsocketConnection {
        task: Option<JoinHandle<()>>,
        heartbeat_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    }

    #[derive(Debug)]
    pub struct ToGst{
        pub remote_sdp: String
    }

    impl Drop for WebsocketConnection {
        fn drop(&mut self) {
            info!("Closing websocket connection");

            let heartbeat_task = self.heartbeat_task.clone();
            task::spawn(async move {
                if let Some(task) = heartbeat_task.lock().await.take() {
                    task.abort();
                }
            });

            if let Some(task) = self.task.take() {
                task.abort();
            }
        }
    }

    impl WebsocketConnection {
        #[tracing::instrument]
        pub async fn new(
            endpoint: String,
            max_framerate: u8,
            max_resolution: StreamResolutionInformation,
            rtc_connection_id: String,
            ip: String,
            server_id: String,
            session_id: String,
            token: String,
            user_id: String,
            mut from_gst_rx: mpsc::Receiver<ToWs>,
            to_gst_tx: mpsc::Sender<ToGst>,
            command_sender: mpsc::Sender<SocketListenerCommand>,
        ) -> Result<Self, async_tungstenite::tungstenite::Error> {
            //v7 is going to be deprecated according to discord's docs (https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87) but is the one that discord client still use for video streams
            let (mut ws_stream, response) = connect_async(format!("wss://{}/?v={}", endpoint, API_VERSION)).await?;

            if response.status() != 101 {
                error!("Connection failed with response code: {:?}", response.status());
                let _ = ws_stream.close(None).await;
                return Err(async_tungstenite::tungstenite::Error::ConnectionClosed);
            } else {
                info!("WebSocket connection successful");
            }

            let (ws_write, ws_read) = ws_stream.split();

            let ws_write = Arc::new(Mutex::new(ws_write));

            let op15_count = Arc::new(AtomicUsize::new(0));
            let nonce = Arc::new(Mutex::new(None));

            let heartbeat_task = Arc::new(Mutex::new(None));

            let ws_listener = ws_read.for_each({
                let heartbeat_task = heartbeat_task.clone();
                let ws_write = ws_write.clone();
                let command_sender = command_sender.clone();
                move |msg| {
                    let ws_write = ws_write.clone();
                    let command_sender = command_sender.clone();
                    let heartbeat_task = heartbeat_task.clone();

                    let op15_count = op15_count.clone();
                    let nonce_arc = nonce.clone();

                    let to_gst_tx = to_gst_tx.clone();

                    // Clone Strings to move across threads
                    // let ip = ip.clone();

                    async move {
                        let mut msg = match msg {
                            Ok(ws_msg) => {
                                // Handle close codes
                                if ws_msg.is_close() {
                                    debug!("WebSocket closed");
                                    if let Err(e) = command_sender.clone().send(SocketListenerCommand::StopStreamInternal).await {
                                        error!("Failed to notify command processor of stream stop: {e}");
                                    }

                                    return;
                                }

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
                                error!("Failed to deserialize websocket message: {}", msg);
                                error!("Deserialization returned error: {:?}", e);
                                return;
                            }
                        };

                        trace!("{:?}", msg);

                        match msg {
                            IncomingWebsocketMessage::OpCode4(data) => {
                                match to_gst_tx.send(ToGst {
                                    remote_sdp: data.sdp
                                }).await {
                                    Ok(_) => {
                                        debug!("[WS->WebRTC] remote SDP sent to gst");
                                    }
                                    Err(e) => {
                                        //TODO: Handle error
                                        debug!("Failed to send remote SDP to gst: {:?}", e);
                                    }
                                };
                                /*
                                task::spawn({
                                    let ws_write = ws_write.clone();
                                    async move {
                                        Self::send_partial_stream_information(
                                            ws_write.lock().await.borrow_mut(), 
                                            stream_ssrcs.ssrcs.audio,
                                            stream_ssrcs.ssrcs.rtx,
                                            stream_ssrcs.ssrcs.video,
                                            stream_ssrcs.local_sdp,
                                            GatewayResolution::from_socket_info(max_resolution), 
                                            max_framerate, 
                                            true
                                        ).await.expect("Failed to send stream information");
                                    }
                                });

                                 */
                            }
                            IncomingWebsocketMessage::OpCode6(data) => {
                                if let Some(nonce) = nonce_arc.lock().await.as_ref() {
                                    // Make sure casting is done correctly
                                    let received_nonce = if data.d.is_u64() {
                                        data.d.as_u64().unwrap()
                                    } else {
                                        data.d.as_str().unwrap().parse::<u64>().unwrap_or(0)
                                    };

                                    if *nonce != received_nonce {
                                        error!("Heartbeat nonce values didn't match!");
                                    }
                                }
                            }
                            IncomingWebsocketMessage::OpCode8(data) => {
                                debug!("Websocket heartbeat interval: {}", data.heartbeat_interval);

                                let _ = heartbeat_task.lock().await.insert(task::spawn(async move {
                                    let nonce_arc = nonce_arc.clone();
                                    let ws_write = ws_write.clone();
                                    let mut is_first = true;
                                    loop {
                                        let multiplier: f64 = if is_first {
                                            rand::thread_rng().gen_range(0.0..1.0)
                                        } else {
                                            1.0
                                        };

                                        sleep(Duration::from_millis(data.heartbeat_interval * multiplier as u64)).await;
                                        // TODO: Better error handling
                                        let _ = nonce_arc.lock().await.insert(Self::send_heartbeat(ws_write.lock().await.borrow_mut()).await.expect("Failed to send heartbeat"));
                                        debug!("Sent websocket heartbeat");
                                        is_first = false;
                                    }
                                }));
                            }
                            IncomingWebsocketMessage::OpCode15(_) => {
                                //TODO: Check if this is still needed
                                let _ = op15_count.fetch_add(1, Ordering::SeqCst);
                            }
                            IncomingWebsocketMessage::OpCode16(data) => {
                                info!("Voice Server: {}", data.voice);
                                info!("RTC Worker: {}", data.rtc_worker);
                            }
                            _ => {}
                        }
                    }
                }
            });

            let ws_listener_task = task::spawn(ws_listener);

            // TODO better error handling
            match Self::auth(ws_write.clone().lock().await.borrow_mut(), server_id, session_id, token, user_id).await {
                Ok(_) => {}
                Err(e) => {
                    ws_listener_task.abort();
                    return Err(e);
                }
            }

            let ws_data = from_gst_rx.recv().await.unwrap();
            debug!("Got data from gst: {:?}", ws_data);

            Self::send_stream_information(ws_write.clone().lock().await.borrow_mut(), ws_data, GatewayResolution::from_socket_info(max_resolution), max_framerate, rtc_connection_id).await.expect("Failed to send stream information");


            Ok(Self { task: Some(ws_listener_task), heartbeat_task })
        }


        /// auth the websocket connection using opcode 0
        /// https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87
        #[tracing::instrument]
        pub async fn auth(write: &mut WebSocketWrite, server_id: String, session_id: String, token: String, user_id: String) -> Result<(), async_tungstenite::tungstenite::Error> {
            let ws_message = OutgoingWebsocketMessage::OpCode0(OpCode0 {
                server_id,
                session_id,
                streams: vec![GatewayStream {
                    stream_type: "screen".to_string(),
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

            trace!("[AUTH] {:?}", ws_message);

            write.send(Message::Text(ws_message)).await?;

            Ok(())
        }

        #[tracing::instrument]
        pub async fn send_heartbeat(write: &mut WebSocketWrite) -> Result<u64, async_tungstenite::tungstenite::Error> {
            let nonce: u64 = rand::random();
            let ws_message = OutgoingWebsocketMessage::OpCode3(OpCode3_6 {
                d: Value::Number(Number::from(nonce)),
            }).to_json();

            trace!("[HEARTBEAT] {}", ws_message);

            write.send(Message::Text(ws_message)).await?;

            Ok(nonce)
        }

        #[tracing::instrument]
        async fn send_partial_stream_information(
            write: &mut WebSocketWrite,
            ssrcs: StreamSSRCs,
            max_resolution: GatewayResolution,
            max_framerate: u8,
        ) -> Result<(), async_tungstenite::tungstenite::Error> {
            let ws12 = OutgoingWebsocketMessage::OpCode12(OpCode12 {
                audio_ssrc: ssrcs.audio,
                rtx_ssrc: ssrcs.rtx,
                video_ssrc: ssrcs.video,
                streams: vec![
                    GatewayStream {
                        stream_type: "video".to_string(),
                        rid: "100".to_string(),
                        quality: 100,
                        active: Some(true),
                        ssrc: Some(ssrcs.video),
                        rtx_ssrc: Some(ssrcs.rtx),
                        max_bitrate: Some(MAX_BITRATE),
                        max_framerate: Some(max_framerate),
                        max_resolution: Some(max_resolution),
                    }
                ],
            }).to_json();

            trace!("[PARTIAL STREAM] {ws12}");

            write.send(Message::Text(ws12)).await?;
            Ok(())
        }

        #[tracing::instrument]
        async fn send_speaking_message(
            write: &mut WebSocketWrite,
        ) -> Result<(), async_tungstenite::tungstenite::Error> {
            let ws5 = OutgoingWebsocketMessage::OpCode5(OpCode5 {
                ssrc: 0,
                speaking: true,
                delay: 5,
            }).to_json();

            trace!("[SPEAKING] {ws5}");

            write.send(Message::Text(ws5)).await?;
            Ok(())
        }

        #[tracing::instrument]
        async fn send_stream_information(
            write: &mut WebSocketWrite,
            web_rtc_data: ToWs,
            max_resolution: GatewayResolution,
            max_framerate: u8,
            rtc_connection_id: String
        ) -> Result<(), async_tungstenite::tungstenite::Error> {
            let ws1 = OutgoingWebsocketMessage::OpCode1(OpCode1 {
                protocol: "webrtc".to_string(),
                rtc_connection_id,
                codecs: vec![
                    GatewayCodec {
                        name: "opus".to_string(),
                        codec_type: PayloadType::Audio,
                        priority: 1000,
                        payload_type: 111,
                        rtx_payload_type: None,
                    },
                    GatewayCodec {
                        name: "H264".to_string(),
                        codec_type: PayloadType::Video,
                        priority: 1000,
                        payload_type: web_rtc_data.video_payload_type,
                        rtx_payload_type: Some(web_rtc_data.rtx_payload_type),
                    }
                ],
                data: web_rtc_data.local_sdp.clone(),
                sdp: web_rtc_data.local_sdp,
            }).to_json();

            trace!("[STREAM] {ws1}");

            write.send(Message::Text(ws1)).await?;

            // TODO: clean this up
            Self::send_partial_stream_information(
                write,
                web_rtc_data.ssrcs,
                max_resolution,
                max_framerate
            ).await?;

            Self::send_speaking_message(write).await?;

            Ok(())
        }
    }
}