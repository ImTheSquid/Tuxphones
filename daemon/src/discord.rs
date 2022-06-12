pub mod api{
    #[derive(serde::Deserialize)]
    pub struct GetWsEndpointResponse {
        pub url: String,
    }

    #[derive(Debug)]
    pub enum GetWsEndpointResponseError {
        ReqwestError(reqwest::Error),
        ParseError(serde_json::Error),
    }

    impl From<reqwest::Error> for GetWsEndpointResponseError {
        fn from(error: reqwest::Error) -> GetWsEndpointResponseError {
            GetWsEndpointResponseError::ReqwestError(error)
        }
    }

    impl From<serde_json::Error> for GetWsEndpointResponseError {
        fn from(error: serde_json::Error) -> GetWsEndpointResponseError {
            GetWsEndpointResponseError::ParseError(error)
        }
    }

    pub async fn get_ws_endpoint() -> Result<GetWsEndpointResponse, GetWsEndpointResponseError> {
        let body = reqwest::get("https://discord.com/api/gateway").await?.text().await?;
        let deserialized: GetWsEndpointResponse = serde_json::from_str(&body)?;

        Ok(deserialized)
    }
}

pub mod websocket{
    use tokio_tungstenite::connect_async;
    use tracing::{debug};
    use crate::discord::api::get_ws_endpoint;

    pub struct websocket_connection {
    }

    impl websocket_connection {
        #[tracing::instrument]
        pub async fn new() -> Result<websocket_connection, tokio_tungstenite::tungstenite::Error> {
            debug!("Connecting");

            let (ws_stream, response) = connect_async(format!("{:?}/?v=7", get_ws_endpoint().await.unwrap().url)).await?;

            debug!("Connected with response code: {:?}", response.status());




            Ok(websocket_connection {
            })
        }
    }
}