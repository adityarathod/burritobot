use reqwest::Client;
use serde::{self, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path, sync::LazyLock};

/// The default URL for the Chipotle restaurant service.
const DEFAULT_RESTAURANT_SERVICE_URL: &str =
    "https://services.chipotle.com/restaurant/v3/restaurant/";

/// The header to use to send the API key.
const API_KEY_HEADER: &str = "Ocp-Apim-Subscription-Key";

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

/// Response from the restaurant service.
#[derive(Deserialize)]
struct LocationDataResponse {
    data: Vec<LocationData>,
}

/// Information about a single location.
#[derive(Deserialize)]
struct LocationData {
    #[serde(alias = "restaurantNumber")]
    id: i32,
    addresses: Vec<Address>,
}

/// Address information
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Address {
    postal_code: Option<String>,
    country_code: String,
}

#[derive(Debug, Serialize, Deserialize, Eq)]
pub struct Location {
    pub id: i32,
    pub zip_code: String,
}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.zip_code == other.zip_code
    }
}

fn get_zip_code(location_id: &i32, address: &Address) -> String {
    ZIP_CODE_OVERRIDES
        .get(location_id)
        .map(|x| (*x).to_string())
        .or(address.postal_code.clone())
        .map(|x| if x.len() > 5 { x[0..5].to_string() } else { x })
        .unwrap()
}

fn get_us_locations(data: LocationDataResponse) -> Vec<Location> {
    data.data
        .iter()
        .filter_map(|location| {
            if let Some(address) = location.addresses.get(0) {
                if address.country_code == "US" {
                    return Some(Location {
                        id: location.id,
                        zip_code: get_zip_code(&location.id, address),
                    });
                }
            } else {
                eprintln!("Location {} has no address", location.id);
            }
            None
        })
        .collect()
}

#[derive(Debug)]
pub enum GetError {
    RequestError(reqwest::Error),
    ResponseError(reqwest::StatusCode),
    ResponseBodyError(reqwest::Error),
    ParseError(serde_json::Error),
}

pub async fn get(
    client: &Client,
    api_key: &str,
    restaurant_service_url: Option<&str>,
) -> Result<Vec<Location>, GetError> {
    match client
        .post(restaurant_service_url.unwrap_or(DEFAULT_RESTAURANT_SERVICE_URL))
        .header("Content-Type", "application/json")
        .header(API_KEY_HEADER, api_key)
        .body(DEFAULT_REQUEST_BODY.to_string())
        .send()
        .await
    {
        Ok(response) => {
            if !response.status().is_success() {
                return Err(GetError::ResponseError(response.status()));
            }
            let response_body = response
                .text()
                .await
                .map_err(|e| GetError::ResponseBodyError(e))?;
            let parsed_body: LocationDataResponse = serde_json::from_str(response_body.as_str())
                .map_err(|e| GetError::ParseError(e))?;
            Ok(get_us_locations(parsed_body))
        }
        Err(e) => Err(GetError::RequestError(e)),
    }
}

#[derive(Debug)]
pub enum LoadError {
    ReadError(std::io::Error),
    ParseError(serde_json::Error),
}

pub async fn load<P: AsRef<Path>>(path: P) -> Result<Vec<Location>, LoadError> {
    let file_contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| LoadError::ReadError(e))?;
    let parsed_body: Vec<Location> =
        serde_json::from_str(file_contents.as_str()).map_err(|e| LoadError::ParseError(e))?;
    Ok(parsed_body)
}

#[derive(Debug)]
pub enum SaveError {
    WriteError(std::io::Error),
    SerializeError(serde_json::Error),
}

pub async fn save<P: AsRef<Path>>(path: P, locations: &[Location]) -> Result<(), SaveError> {
    let serialized = serde_json::to_string(locations).map_err(|e| SaveError::SerializeError(e))?;
    tokio::fs::write(path, serialized)
        .await
        .map_err(|e| SaveError::WriteError(e))?;
    Ok(())
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

        // Act
        let locations = get(&client, FAKE_API_KEY, Some(url.as_str())).await;

        // Assert
        assert!(
            locations.is_ok(),
            "Failed to get locations: {:?}",
            locations.unwrap_err()
        );
        let locations = locations.unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].id, 1234);
        assert_eq!(locations[0].zip_code, "12345");
        locations_mock.assert();
    }

    #[tokio::test]
    async fn get_invalid_url() {
        // Arrange
        let client = reqwest::Client::new();

        // Act
        let locations = get(&client, FAKE_API_KEY, Some("http://test.invalid")).await;

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

        // Act
        let locations = get(&client, FAKE_API_KEY, Some(url.as_str())).await;

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

        // Act
        let locations = get(&client, FAKE_API_KEY, Some(url.as_str())).await;

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

        // Act
        let locations = get(&client, FAKE_API_KEY, Some(url.as_str())).await;

        // Assert
        assert!(locations.is_ok());
        assert!(locations.unwrap().is_empty());
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
        let locations = load(temp_file.path()).await;

        // Assert
        assert!(
            locations.is_ok(),
            "Failed to load locations: {:?}",
            locations.unwrap_err()
        );
        let locations = locations.unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0], fake_location);
    }

    #[tokio::test]
    async fn load_invalid_file() {
        // Act
        let locations = load("totally_nonexistent.json").await;

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
        let locations = load(temp_file.path()).await;

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
        let locations = vec![fake_location];
        let temp_file = NamedTempFile::new().unwrap();

        // Act
        let save_result = save(temp_file.path(), &locations).await;

        // Assert
        assert!(
            save_result.is_ok(),
            "Failed to save locations: {:?}",
            save_result.unwrap_err()
        );
        let loaded_locations = load(temp_file.path()).await.unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(&loaded_locations[0], &locations[0]);
    }
}
