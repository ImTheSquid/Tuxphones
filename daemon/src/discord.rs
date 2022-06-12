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