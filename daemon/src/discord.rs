pub mod websocket {
    use futures_util::SinkExt;
    use futures_util::stream::{SplitSink, StreamExt};
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
    use tokio_tungstenite::tungstenite::Message;
    use tracing::{debug, info, trace};

    use crate::discord_op::opcodes::*;

    const API_VERSION: u8 = 7;

    #[derive(Debug)]
    pub struct WebsocketConnection {
        ws_send_stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    }

    impl WebsocketConnection {
        #[tracing::instrument]
        pub async fn new(endpoint: String) -> Result<Self, tokio_tungstenite::tungstenite::Error> {
            //v7 is going to be deprecated according to discord's docs (https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87) but is the one that discord client still use for video streams
            let (ws_stream, response) = connect_async(format!("{}/?v={}", endpoint, API_VERSION)).await?;

            let (write, read) = ws_stream.split();

            debug!("Connected with response code: {:?}", response.status());

            Ok(Self {
                ws_send_stream: write,
            })
        }

        /// auth the websocket connection using opcode 0
        /// https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87
        #[tracing::instrument]
        pub async fn auth(&mut self, server_id: String, session_id: String, token: String, user_id: String) -> Result<(), tokio_tungstenite::tungstenite::Error> {
            let ws_message = WebsocketMessage::OpCode0(OpCode0 {
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
            });

            let ws_message_string = serde_json::to_string(&ws_message).unwrap();

            trace!("Sending auth message: {}", ws_message_string);

            self.ws_send_stream.send(Message::Text(ws_message_string)).await?;

            Ok(())
        }
    }
}