pub mod api{
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

pub mod websocket{
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
    use tracing::{debug, info};
    use tracing::field::debug;
    use crate::discord::api::{get_ws_endpoint, ApiError};

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct WebsocketMessage {
        pub op: u8,
        pub d: WebsocketMessageD,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum WebsocketMessageD {
        OpCode0(OpCode0),
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode0stream {
        #[serde(rename = "type")]
        pub stream_type: String,
        pub rid: u8,
        pub quantity: u8,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode0 {
        pub server_id: String,
        pub session_id: String,
        pub streams: Vec<OpCode0stream>,
        pub token: String,
        pub user_id: String,
        pub video: bool,
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

    pub struct WebsocketConnection {
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    }

    impl WebsocketConnection {
        #[tracing::instrument]
        pub async fn new() -> Result<Self, WebsocketInitError> {
            //v7 is going to be deprecated according to discord's docs (https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87) but is the one that discord client still use for video streams
            let endpoint = format!("{}/?v=7", get_ws_endpoint().await?.url);
            info!("Connecting to {}", endpoint);

            let (ws_stream, response) = connect_async(endpoint).await?;

            debug!("Connected with response code: {:?}", response.status());

            Ok(Self {
                ws_stream
            })
        }

        /// auth the websocket connection using opcode 0
        /// https://www.figma.com/file/AJoBnWrHIFxjeppBRVfqXP/Discord-stream-flow?node-id=48%3A87
        #[tracing::instrument]
        pub async fn auth(server_id: &str, session_id: &str, token: &str, user_id: &str) {

            "".to_string();
        }
    }
}