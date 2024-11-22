use std::sync::LazyLock;

use super::constants::DEFAULT_API_KEY_SOURCE_URL;
use regex::Regex;
use reqwest::Client;
use thiserror::Error;

const API_KEY_PATTERN: &str = r#"gatewaySubscriptionKey:Q\("([a-zA-Z0-9-]+)"\)"#;
static API_KEY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(API_KEY_PATTERN).expect("Invalid regex pattern"));

#[derive(Debug, Error)]
pub enum ApiKeyError {
    #[error("the client bundle request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("the client bundle request failed with status code: {0}")]
    ResponseError(reqwest::StatusCode),
    #[error("the client bundle response body could not be read: {0}")]
    ResponseBodyError(#[source] reqwest::Error),
    #[error("the API key could not be found in the client bundle")]
    ApiKeyNotFound,
}

/// Retrieve the API key from the Chipotle client bundle.
///
/// * `client` - The reqwest HTTP client to use for the request.
/// * `bundle_url` - The URL to retrieve the client bundle from. If not provided, the default URL will be used.
pub async fn get(client: &Client, bundle_url: Option<&str>) -> Result<String, ApiKeyError> {
    let response = client
        .get(bundle_url.unwrap_or(DEFAULT_API_KEY_SOURCE_URL))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(ApiKeyError::ResponseError(response.status()));
    }
    let body = response
        .text()
        .await
        .map_err(ApiKeyError::ResponseBodyError)?;
    let captures = API_KEY_REGEX
        .captures(&body)
        .ok_or(ApiKeyError::ApiKeyNotFound)?;

    captures
        .get(1)
        .map(|m| m.as_str().to_string())
        .ok_or(ApiKeyError::ApiKeyNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    const FAKE_API_KEY: &str = "fake-api-key";

    #[tokio::test]
    async fn get_success() {
        // Arrange
        let server = MockServer::start_async().await;
        let api_key_mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(200).body(format!(
                    r#"thingthing;gatewaySubscriptionKey:Q("{}");3fjhkasfd78r3"#,
                    FAKE_API_KEY
                ));
            })
            .await;
        let url = server.url("/");
        let client = reqwest::Client::new();

        // Act
        let api_key = get(&client, Some(&url)).await;

        // Assert
        assert!(
            api_key.is_ok(),
            "Failed to get API key: {:?}",
            api_key.unwrap_err()
        );
        assert_eq!(api_key.unwrap(), FAKE_API_KEY);
        api_key_mock.assert();
    }

    #[tokio::test]
    async fn get_bad_status() {
        // Arrange
        let server = MockServer::start_async().await;
        let api_key_mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(403);
            })
            .await;
        let url = server.url("/");
        let client = reqwest::Client::new();

        // Act
        let api_key = get(&client, Some(&url)).await;

        // Assert
        assert!(api_key.is_err());
        assert!(matches!(
            api_key.unwrap_err(),
            ApiKeyError::ResponseError(_)
        ));
        api_key_mock.assert();
    }

    #[tokio::test]
    async fn get_not_found() {
        // Arrange
        let server = MockServer::start_async().await;
        let api_key_mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/");
                then.status(200).body("thingthing;3fjhkasfd78r3");
            })
            .await;
        let url = server.url("/");
        let client = reqwest::Client::new();

        // Act
        let api_key = get(&client, Some(&url)).await;

        // Assert
        assert!(api_key.is_err());
        assert!(matches!(api_key.unwrap_err(), ApiKeyError::ApiKeyNotFound));
        api_key_mock.assert();
    }
}
