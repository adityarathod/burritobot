use regex::Regex;
use reqwest::Client;

pub const DEFAULT_API_SOURCE_URL: &str =
    "https://orderweb-prd-centralus-cdne.azureedge.net/js/app.js";
const API_KEY_PATTERN: &str = r#"gatewaySubscriptionKey:Q\("([a-zA-Z0-9-]+)"\)"#;

#[derive(Debug)]
pub enum ApiKeyError {
    /// The client bundle failed to load from the API source.
    ClientBundleLoadError(reqwest::Error),
    /// The client bundle was unable to be retrieved.
    ClientBundleRetrievalError,
    /// The API key was not found in the client bundle.
    ApiKeyNotFound,
}

/// Retrieve the API key from the Chipotle client bundle.
///
/// * `client` - The reqwest HTTP client to use for the request.
/// * `bundle_url` - The URL to retrieve the client bundle from. If not provided, the default URL will be used.
pub async fn get(client: &Client, bundle_url: Option<&str>) -> Result<String, ApiKeyError> {
    let pattern = Regex::new(API_KEY_PATTERN).expect("Invalid regex pattern");
    match client
        .get(bundle_url.unwrap_or(DEFAULT_API_SOURCE_URL))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                let body = response
                    .text()
                    .await
                    .map_err(|_| ApiKeyError::ClientBundleRetrievalError)?;
                return match pattern.captures(body.as_str()) {
                    Some(captures) => Ok(captures[1].to_string()),
                    None => Err(ApiKeyError::ApiKeyNotFound),
                };
            } else {
                Err(ApiKeyError::ClientBundleRetrievalError)
            }
        }
        Err(e) => Err(ApiKeyError::ClientBundleLoadError(e)),
    }
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
            ApiKeyError::ClientBundleRetrievalError
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
