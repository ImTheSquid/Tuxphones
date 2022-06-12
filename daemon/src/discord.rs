pub mod api {
    #[derive(serde::Deserialize)]
    pub struct GetWsEndpointResponse {
        pub url: String,
    }

    #[derive(Debug)]
    pub enum ApiError {
        ReqwestError(reqwest::Error),
        ParseError(serde_json::Error),
    }

    impl From<reqwest::Error> for ApiError {
        fn from(error: reqwest::Error) -> ApiError {
            ApiError::ReqwestError(error)
        }
    }

    impl From<serde_json::Error> for ApiError {
        fn from(error: serde_json::Error) -> ApiError {
            ApiError::ParseError(error)
        }
    }

    pub async fn get_ws_endpoint() -> Result<GetWsEndpointResponse, ApiError> {
        let body = reqwest::get("https://discord.com/api/gateway").await?.text().await?;
        let deserialized: GetWsEndpointResponse = serde_json::from_str(&body)?;

        Ok(deserialized)
    }
}

pub mod websocket {
    use futures_util::SinkExt;
    use futures_util::stream::{SplitSink, StreamExt};
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
    use tokio_tungstenite::tungstenite::Message;
    use tracing::{debug, info, trace};

    use crate::discord::api::{ApiError, get_ws_endpoint};

    const API_VERSION: u8 = 7;

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct WebsocketMessage {
        pub op: u8,
        // TODO: find a way to tell serde to deserialize this based on the op
        pub d: WebsocketMessageD,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum WebsocketMessageD {
        OpCode0(OpCode0),
        OpCode2(OpCode2),
        OpCode8(OpCode8),
        OpCode12(OpCode12)
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct  GatewayResolution {
        #[serde(rename = "type")]
        resolution_type: String,
        width: u16,
        height: u16,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct GatewayStream {
        //Opcode 0 and 2 and 12 params
        #[serde(rename = "type")]
        pub stream_type: String,
        pub rid: u8,
        pub quality: u8,

        //Opcode 2 and 12 params
        #[serde(skip_serializing_if = "Option::is_none")]
        pub active: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ssrc: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub rtx_ssrc: Option<u32>,

        //Opcode 12 params
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_bitrate: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_framerate: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_resolution: Option<GatewayResolution>,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode0 {
        pub server_id: String,
        pub session_id: String,
        pub streams: Vec<GatewayStream>,
        /// session token
        pub token: String,
        pub user_id: String,
        pub video: bool,
    }

    /// Incoming message containing configuration options for websocket connection
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode2 {
        experiment: Vec<String>,
        /// Discord ip address to stream to
        ip: String,
        /// Discord port to stream to
        port: u16,
        /// Supported encrpytion modes by the server
        modes: Vec<String>,
        ssrc: u32,
        streams: Vec<GatewayStream>,
    }

    /// Initial heartbeat incoming message
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode8 {
        /// the interval (in milliseconds) the client should heartbeat with
        heartbeat_interval: u32,
        /// api version
        v: u8,
    }

    /// Outgoing message containing info about the stream
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode12 {
        audio_ssrc: u32,
        rtx_ssrc: u32,
        video_ssrc: u32,
        streams: Vec<GatewayStream>,
    }


    #[derive(Debug)]
    pub enum WebsocketInitError {
        ApiError(ApiError),
        TungsteniteError(tokio_tungstenite::tungstenite::Error),
    }

    impl From<ApiError> for WebsocketInitError {
        fn from(error: ApiError) -> WebsocketInitError {
            WebsocketInitError::ApiError(error)
        }
    }


    impl From<tokio_tungstenite::tungstenite::Error> for WebsocketInitError {
        fn from(error: tokio_tungstenite::tungstenite::Error) -> WebsocketInitError {
            WebsocketInitError::TungsteniteError(error)
        }
    }

    #[derive(Debug)]
    pub struct WebsocketConnection {
        ws_send_stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    }

    impl WebsocketConnection {
        #[tracing::instrument]
        pub async fn new() -> Result<Self, WebsocketInitError> {
            //v7 is going to be deprecated according to discord's docs (https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87) but is the one that discord client still use for video streams
            let endpoint = format!("{}/?v={}", get_ws_endpoint().await?.url, API_VERSION);
            info!("Connecting to {}", endpoint);

            let (ws_stream, response) = connect_async(endpoint).await?;

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
            let ws_message = WebsocketMessage {
                op: 0,
                d: WebsocketMessageD::OpCode0(OpCode0 {
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
                        max_resolution: None
                    }],
                    token,
                    user_id,
                    video: true,
                }),
            };

            let ws_message_string = serde_json::to_string(&ws_message).unwrap();

            trace!("Sending auth message: {}", ws_message_string);

            self.ws_send_stream.send(Message::Text(ws_message_string)).await?;

            Ok(())
        }
    }
}