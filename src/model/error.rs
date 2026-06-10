use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("URL parse error: {0}")]
    InvalidUrl(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Request timeout ({0}s)")]
    Timeout(u32),
}
