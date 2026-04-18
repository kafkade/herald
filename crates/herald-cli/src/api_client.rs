use herald_common::ErrorResponse;
use reqwest::StatusCode;

/// User-friendly errors for API operations.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Connection refused — is the Herald server running at {0}?")]
    ConnectionRefused(String),
    #[error("Authentication failed — check your --token value")]
    Unauthorized,
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Server error: {0}")]
    ServerError(String),
    #[error("{0}")]
    Other(String),
}

/// HTTP client for the Herald REST API.
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl ApiClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
        }
    }

    /// Send a POST request with a JSON body and return the deserialized response.
    pub async fn post<Req, Res>(&self, path: &str, body: &Req) -> Result<Res, ApiError>
    where
        Req: serde::Serialize,
        Res: serde::de::DeserializeOwned,
    {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .map_err(|e| classify_request_error(e, &self.base_url))?;

        handle_response(response).await
    }

    /// Send a GET request and return the deserialized response.
    pub async fn get<Res>(&self, path: &str) -> Result<Res, ApiError>
    where
        Res: serde::de::DeserializeOwned,
    {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| classify_request_error(e, &self.base_url))?;

        handle_response(response).await
    }

    /// Send a DELETE request. Returns Ok(()) on 204 No Content.
    pub async fn delete(&self, path: &str) -> Result<(), ApiError> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .client
            .delete(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| classify_request_error(e, &self.base_url))?;

        let status = response.status();
        if status == StatusCode::NO_CONTENT || status.is_success() {
            return Ok(());
        }

        Err(classify_status_error(status, response).await)
    }
}

/// Classify a reqwest transport error into a user-friendly ApiError.
fn classify_request_error(err: reqwest::Error, base_url: &str) -> ApiError {
    if err.is_connect() {
        ApiError::ConnectionRefused(base_url.to_string())
    } else {
        ApiError::Other(err.to_string())
    }
}

/// Extract the error body and classify by HTTP status code.
async fn classify_status_error(status: StatusCode, response: reqwest::Response) -> ApiError {
    let body_msg = response
        .json::<ErrorResponse>()
        .await
        .map(|e| e.message)
        .unwrap_or_else(|_| status.to_string());

    match status {
        StatusCode::UNAUTHORIZED => ApiError::Unauthorized,
        StatusCode::NOT_FOUND => ApiError::NotFound(body_msg),
        StatusCode::BAD_REQUEST => ApiError::BadRequest(body_msg),
        s if s.is_server_error() => ApiError::ServerError(body_msg),
        _ => ApiError::Other(format!("{status}: {body_msg}")),
    }
}

/// Handle a successful HTTP response, deserializing the JSON body.
async fn handle_response<Res: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<Res, ApiError> {
    let status = response.status();
    if status.is_success() {
        response
            .json::<Res>()
            .await
            .map_err(|e| ApiError::Other(format!("Failed to parse response: {e}")))
    } else {
        Err(classify_status_error(status, response).await)
    }
}
