use tokio_tungstenite::connect_async;
use tracing::field::debug;
use tracing::{debug, info, trace};
use tracing_subscriber::fmt::format;
use crate::discord::get_ws_endpoint;

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