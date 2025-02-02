use crate::{api_interfaces::locations, util::default_http_client, ApiKey};

use super::constants::API_KEY_HEADER;
use super::error::*;
use reqwest::Client;
use serde::{self, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path, sync::LazyLock};

const DEFAULT_LOCATION_INFO_ENDPOINT: &str =
    "https://services.chipotle.com/restaurant/v3/restaurant/";

/// Zip code overrides for specific location IDs.
static ZIP_CODE_OVERRIDES: LazyLock<HashMap<i32, &'static str>> =
    LazyLock::new(|| HashMap::from([(3065, "75235")]));

/// Default request body for getting all locations.
static DEFAULT_REQUEST_BODY: LazyLock<Value> = LazyLock::new(|| {
    json!({
        "latitude": 0,
        "longitude": 0,
        "radius": 999999999,
        "restaurantStatuses": ["OPEN", "LAB"],
        "conceptIds": ["CMG"],
        "orderBy": "distance",
        "orderByDescending": false,
        // 4000 is a good upper limit for the number of locations. Change when there are more.
        "pageSize": 4000,
        "pageIndex": 0,
        "embeds": {
            "addressTypes": ["MAIN"],
            "realHours": false,
            "directions": false,
            "catering": false,
            "onlineOrdering": true,
            "timezone": false,
            "marketing": false,
            "chipotlane": false,
            "sustainability": false,
            "experience": false,
        },
    })
});

/// Key identifying information for the location.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub id: i32,
    pub zip_code: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Locations(Vec<Location>);

impl Locations {
    /// Retrieve all US locations using the default HTTP client and endpoint.
    pub async fn get_all_default(key: &ApiKey) -> Result<Self, GetError> {
        let client = default_http_client();
        Self::get_all_us_custom(key, &client, None).await
    }

    /// Retrieve all US locations using a custom HTTP client and endpoint.
    pub async fn get_all_us_custom(
        key: &ApiKey,
        client: &Client,
        endpoint: Option<&str>,
    ) -> Result<Self, GetError> {
        let response = client
            .post(endpoint.unwrap_or(DEFAULT_LOCATION_INFO_ENDPOINT))
            .header("Content-Type", "application/json")
            .header(API_KEY_HEADER, key.get())
            .body(DEFAULT_REQUEST_BODY.to_string())
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(GetError::ResponseError(response.status()));
        }
        let response_body = response.text().await.map_err(GetError::ResponseBodyError)?;
        let parsed_body: locations::Response = serde_json::from_str(response_body.as_str())?;
        Ok(Locations(get_us_locations(parsed_body)))
    }

    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self, LoadError> {
        let file_contents = tokio::fs::read_to_string(path).await?;
        Ok(Self(serde_json::from_str(file_contents.as_str())?))
    }

    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveError> {
        let serialized = serde_json::to_string(&self.0)?;
        tokio::fs::write(path, serialized).await?;
        Ok(())
    }
}

fn get_zip_code(location_id: &i32, address: &locations::Address) -> String {
    ZIP_CODE_OVERRIDES
        .get(location_id)
        .copied()
        .or(address.postal_code.as_deref())
        .map(|x| {
            if x.len() > 5 {
                x[0..5].to_string()
            } else {
                x.to_string()
            }
        })
        .unwrap()
}

fn get_us_locations(data: locations::Response) -> Vec<Location> {
    data.data
        .iter()
        .filter_map(|location| match location.addresses.first() {
            Some(address) if address.country_code == "US" => Some(Location {
                id: location.id,
                zip_code: get_zip_code(&location.id, address),
            }),
            _ => None,
        })
        .collect()
}

impl IntoIterator for Locations {
    type Item = Location;
    type IntoIter = std::vec::IntoIter<Location>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const FAKE_API_KEY: &str = "fake-api-key";

    #[tokio::test]
    async fn get_success() {
        // Arrange
        let server = MockServer::start_async().await;
        let response_json = json!({
            "data": [
                {
                    "restaurantNumber": 1234,
                    "addresses": [
                        {
                            "postalCode": "12345",
                            "countryCode": "US"
                        }
                    ]
                }
            ]
        });
        let locations_mock = server
            .mock_async(|when, then| {
                let body_matcher = Regex::new(".+").unwrap();
                when.path("/")
                    .header(API_KEY_HEADER, FAKE_API_KEY)
                    .json_body(DEFAULT_REQUEST_BODY.clone())
                    .method(POST)
                    .body_matches(body_matcher);
                then.status(200).json_body(response_json);
            })
            .await;
        let url = server.url("/");
        let client = reqwest::Client::new();
        let api_key = ApiKey::from_raw(FAKE_API_KEY);

        // Act
        let locations = Locations::get_all_us_custom(&api_key, &client, Some(url.as_str())).await;

        // Assert
        assert!(
            locations.is_ok(),
            "Failed to get locations: {:?}",
            locations.unwrap_err()
        );
        let locations = locations.unwrap();
        assert_eq!(locations.0.len(), 1);
        assert_eq!(locations.0[0].id, 1234);
        assert_eq!(locations.0[0].zip_code, "12345");
        locations_mock.assert();
    }

    #[tokio::test]
    async fn get_invalid_url() {
        // Arrange
        let client = reqwest::Client::new();
        let api_key = ApiKey::from_raw(FAKE_API_KEY);

        // Act
        let locations =
            Locations::get_all_us_custom(&api_key, &client, Some("http://test.invalid")).await;

        // Assert
        assert!(locations.is_err());
        assert!(matches!(locations.unwrap_err(), GetError::RequestError(_)));
    }

    #[tokio::test]
    async fn get_bad_status() {
        // Arrange
        let server = MockServer::start_async().await;
        let locations_mock = server
            .mock_async(|when, then| {
                when.path("/");
                then.status(403);
            })
            .await;
        let url = server.url("/");
        let client = reqwest::Client::new();
        let api_key = ApiKey::from_raw(FAKE_API_KEY);

        // Act
        let locations = Locations::get_all_us_custom(&api_key, &client, Some(url.as_str())).await;

        // Assert
        assert!(locations.is_err());
        assert!(matches!(locations.unwrap_err(), GetError::ResponseError(_)));
        locations_mock.assert();
    }

    #[tokio::test]
    async fn get_bad_json() {
        // Arrange
        let server = MockServer::start_async().await;
        let locations_mock = server
            .mock_async(|when, then| {
                when.path("/");
                then.status(200)
                    .header("Content-Type", "application/json")
                    .body(r#"{"error": "something is amiss" }"#);
            })
            .await;
        let url = server.url("/");
        let client = reqwest::Client::new();
        let api_key = ApiKey::from_raw(FAKE_API_KEY);

        // Act
        let locations = Locations::get_all_us_custom(&api_key, &client, Some(url.as_str())).await;

        // Assert
        assert!(locations.is_err());
        assert!(matches!(locations.unwrap_err(), GetError::ParseError(_)));
        locations_mock.assert();
    }

    #[tokio::test]
    async fn get_non_us_filtered() {
        // Arrange
        let server = MockServer::start_async().await;
        let response_json = json!({
            "data": [
                {
                    "restaurantNumber": 1234,
                    "addresses": [
                        {
                            "postalCode": "12345",
                            "countryCode": "CA"
                        }
                    ]
                }
            ]
        });
        let locations_mock = server
            .mock_async(|when, then| {
                when.path("/");
                then.status(200).json_body(response_json);
            })
            .await;
        let url = server.url("/");
        let client = reqwest::Client::new();
        let api_key = ApiKey::from_raw(FAKE_API_KEY);

        // Act
        let locations = Locations::get_all_us_custom(&api_key, &client, Some(url.as_str())).await;

        // Assert
        assert!(locations.is_ok());
        assert!(locations.unwrap().0.is_empty());
        locations_mock.assert();
    }

    #[tokio::test]
    async fn load_success() {
        // Arrange
        let fake_location = Location {
            id: 12345,
            zip_code: "54321".to_string(),
        };
        let file_json = json!([fake_location]).to_string();
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", file_json).unwrap();

        // Act
        let locations = Locations::load(temp_file.path()).await;

        // Assert
        assert!(
            locations.is_ok(),
            "Failed to load locations: {:?}",
            locations.unwrap_err()
        );
        let locations = locations.unwrap();
        assert_eq!(locations.0.len(), 1);
        assert_eq!(locations.0[0], fake_location);
    }

    #[tokio::test]
    async fn load_invalid_file() {
        // Act
        let locations = Locations::load("totally_nonexistent.json").await;

        // Assert
        assert!(locations.is_err());
        assert!(matches!(locations.unwrap_err(), LoadError::ReadError(_)));
    }

    #[tokio::test]
    async fn load_bad_json() {
        // Arrange
        let mut temp_file = NamedTempFile::new().unwrap();
        let json = json!({"not": "a location"}).to_string();
        write!(temp_file, "{}", json).unwrap();

        // Act
        let locations = Locations::load(temp_file.path()).await;

        // Assert
        assert!(locations.is_err());
        assert!(matches!(locations.unwrap_err(), LoadError::ParseError(_)));
    }

    #[tokio::test]
    async fn save_and_load_successful() {
        // Arrange
        let fake_location = Location {
            id: 12345,
            zip_code: "54321".to_string(),
        };
        let locations = Locations(vec![fake_location]);
        let temp_file = NamedTempFile::new().unwrap();

        // Act
        let save_result = locations.save(temp_file.path()).await;

        // Assert
        assert!(
            save_result.is_ok(),
            "Failed to save locations: {:?}",
            save_result.unwrap_err()
        );
        let loaded_locations = Locations::load(temp_file.path()).await.unwrap();
        assert_eq!(locations.0.len(), 1);
        assert_eq!(&loaded_locations.0[0], &locations.0[0]);
    }
}
