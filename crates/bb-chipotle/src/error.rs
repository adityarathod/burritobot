use crate::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GetError {
    #[error("unable to build the request: {0}")]
    BuildError(String),
    #[error("the request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("the request failed with status code: {0}")]
    ResponseError(reqwest::StatusCode),
    #[error("the response body could not be read: {0}")]
    ResponseBodyError(#[source] reqwest::Error),
    #[error("unable to parse the response body: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("unable to translate response object: {0}")]
    TranslateError(#[from] menu::MenuBuilderError),
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("unable to read the file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("unable to parse the file: {0}")]
    ParseError(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("unable to write the file: {0}")]
    WriteError(#[from] std::io::Error),
    #[error("unable to serialize the data: {0}")]
    SerializeError(#[from] serde_json::Error),
}
