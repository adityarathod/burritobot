use std::sync::LazyLock;

use crate::{constants::*, error::GetError};
use serde::{self, Deserialize};
use thiserror::Error;

use super::Summary;

static DEFAULT_ENDPOINT_CONFIG: LazyLock<Endpoint> = LazyLock::new(|| {
    Endpoint::try_new(
        DEFAULT_MENU_SERVICE_URL_FORMAT.to_string(),
        DEFAULT_MENU_SERVICE_URL_REPLACE_TOKEN.to_string(),
    )
    .expect("Invalid default endpoint config")
});

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub restaurant_id: i32,
    pub entrees: Vec<Item>,
    pub sides: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub item_category: String,
    pub item_type: String,
    pub item_id: String,
    pub item_name: String,
    pub unit_price: f32,
    pub unit_delivery_price: f32,
}

#[derive(Debug, PartialEq)]
pub struct Endpoint {
    pub url: String,
    pub replace_token: String,
}

#[derive(Debug, Error, PartialEq)]
pub enum EndpointConfigError {
    #[error("the endpoint format is missing")]
    MissingEndpoint,
    #[error("the replace token is missing")]
    MissingReplaceToken,
    #[error("the replace token provided is not in the endpoint format")]
    ReplaceTokenNotInEndpoint,
}

impl Endpoint {
    pub fn try_new(
        endpoint_format: String,
        replace_token: String,
    ) -> Result<Self, EndpointConfigError> {
        if replace_token.is_empty() {
            return Err(EndpointConfigError::MissingReplaceToken);
        }
        if endpoint_format.is_empty() {
            return Err(EndpointConfigError::MissingEndpoint);
        }
        if !endpoint_format.contains(&replace_token) {
            return Err(EndpointConfigError::ReplaceTokenNotInEndpoint);
        }
        Ok(Self {
            url: endpoint_format,
            replace_token,
        })
    }

    pub fn to_url(&self, store_id: &str) -> String {
        self.url.replace(&self.replace_token, store_id)
    }
}

/// Get the menu summary from the menu service.
pub async fn get(
    restaurant_id: &i32,
    client: &reqwest::Client,
    api_key: &str,
    endpoint_config: Option<Endpoint>,
) -> Result<Summary, GetError> {
    let url = match endpoint_config {
        Some(config) => config.to_url(&restaurant_id.to_string()),
        None => DEFAULT_ENDPOINT_CONFIG.to_url(&restaurant_id.to_string()),
    };
    let response = client
        .get(url)
        .header(API_KEY_HEADER, api_key)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(GetError::ResponseError(response.status()));
    }
    let body = response.text().await.map_err(GetError::ResponseBodyError)?;
    let parsed_body = serde_json::from_str::<Response>(&body)?;
    let summary = Summary::try_from(parsed_body)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use crate::menu::{BuildError, Price};

    use super::*;
    use httpmock::prelude::*;
    use serde_json::{json, Value};

    const FAKE_API_KEY: &str = "fake-api-key";
    const FAKE_RESTAURANT_ID: i32 = 1234;
    static COMPLETE_RESPONSE: LazyLock<Value> = LazyLock::new(|| {
        json!({
            "restaurantId": FAKE_RESTAURANT_ID,
            "entrees": [
                {
                    "itemCategory": "Entree",
                    "itemType": "Bowl",
                    "itemId": "1234",
                    "itemName": "Chicken Bowl",
                    "unitPrice": 7.99,
                    "unitDeliveryPrice": 8.99
                },
                {
                    "itemCategory": "Entree",
                    "itemType": "Bowl",
                    "itemId": "5678",
                    "itemName": "Steak Bowl",
                    "unitPrice": 8.99,
                    "unitDeliveryPrice": 9.99
                },
                {
                    "itemCategory": "Entree",
                    "itemType": "Bowl",
                    "itemId": "9012",
                    "itemName": "Veggie Bowl",
                    "unitPrice": 6.99,
                    "unitDeliveryPrice": 7.99
                }
            ],
            "sides": []
        })
    });

    static INCOMPLETE_RESPONSE: LazyLock<Value> = LazyLock::new(|| {
        json!({
            "restaurantId": FAKE_RESTAURANT_ID,
            "entrees": [
                {
                    "itemCategory": "Entree",
                    "itemType": "Bowl",
                    "itemId": "1234",
                    "itemName": "Chicken Bowl",
                    "unitPrice": 7.99,
                    "unitDeliveryPrice": 8.99
                },
                {
                    "itemCategory": "Entree",
                    "itemType": "Bowl",
                    "itemId": "5678",
                    "itemName": "Steak Bowl",
                    "unitPrice": 8.99,
                    "unitDeliveryPrice": 9.99
                }
            ],
            "sides": []
        })
    });

    #[test]
    fn endpoint_config_try_new_success() {
        let endpoint = "https://example.com/$store_id".to_string();
        let replace_token = "$store_id".to_string();
        let endpoint_config = Endpoint::try_new(endpoint, replace_token);
        assert!(endpoint_config.is_ok());
    }

    #[test]
    fn endpoint_config_try_new_missing_endpoint() {
        let endpoint = "".to_string();
        let replace_token = "$store_id".to_string();
        let endpoint_config = Endpoint::try_new(endpoint, replace_token);
        assert_eq!(endpoint_config, Err(EndpointConfigError::MissingEndpoint));
    }

    #[test]
    fn endpoint_config_try_new_missing_replace_token() {
        let endpoint = "https://example.com/$store_id".to_string();
        let replace_token = "".to_string();
        let endpoint_config = Endpoint::try_new(endpoint, replace_token);
        assert_eq!(
            endpoint_config,
            Err(EndpointConfigError::MissingReplaceToken)
        );
    }

    #[test]
    fn endpoint_config_try_new_replace_token_not_in_endpoint() {
        let endpoint = "https://example.com/store_id".to_string();
        let replace_token = "$store_id".to_string();
        let endpoint_config = Endpoint::try_new(endpoint, replace_token);
        assert_eq!(
            endpoint_config,
            Err(EndpointConfigError::ReplaceTokenNotInEndpoint)
        );
    }

    #[tokio::test]
    async fn get_success() {
        let server = MockServer::start_async().await;
        let menu_mock = server
            .mock_async(|when, then| {
                when.path(format!("/{}", &FAKE_RESTAURANT_ID));
                then.status(200).json_body((*COMPLETE_RESPONSE).clone());
            })
            .await;
        let endpoint_config = Endpoint {
            url: server.url(format!("/{}", DEFAULT_MENU_SERVICE_URL_REPLACE_TOKEN)),
            replace_token: DEFAULT_MENU_SERVICE_URL_REPLACE_TOKEN.to_string(),
        };
        let client = reqwest::Client::new();
        let summary = get(
            &FAKE_RESTAURANT_ID,
            &client,
            FAKE_API_KEY,
            Some(endpoint_config),
        )
        .await;
        assert!(
            summary.is_ok(),
            "Failed to get menu: {:?}",
            summary.unwrap_err()
        );
        let summary = summary.unwrap();
        assert_eq!(summary.restaurant_id, FAKE_RESTAURANT_ID);
        let expected_veggie_bowl_price = Price {
            normal_price: 6.99,
            delivery_price: 7.99,
        };
        let expected_steak_bowl_price = Price {
            normal_price: 8.99,
            delivery_price: 9.99,
        };
        let expected_chicken_bowl_price = Price {
            normal_price: 7.99,
            delivery_price: 8.99,
        };
        assert_eq!(summary.veggie_bowl_price, expected_veggie_bowl_price);
        assert_eq!(summary.chicken_bowl_price, expected_chicken_bowl_price);
        assert_eq!(summary.steak_bowl_price, expected_steak_bowl_price);
        menu_mock.assert();
    }

    #[tokio::test]
    async fn get_incomplete_response() {
        let server = MockServer::start_async().await;
        let menu_mock = server
            .mock_async(|when, then| {
                when.path(format!("/{}", &FAKE_RESTAURANT_ID));
                then.status(200).json_body((*INCOMPLETE_RESPONSE).clone());
            })
            .await;
        let endpoint_config = Endpoint {
            url: server.url(format!("/{}", DEFAULT_MENU_SERVICE_URL_REPLACE_TOKEN)),
            replace_token: DEFAULT_MENU_SERVICE_URL_REPLACE_TOKEN.to_string(),
        };
        let client = reqwest::Client::new();
        let summary = get(
            &FAKE_RESTAURANT_ID,
            &client,
            FAKE_API_KEY,
            Some(endpoint_config),
        )
        .await;
        assert!(summary.is_err());
        assert!(matches!(
            summary.unwrap_err(),
            GetError::TranslateError(BuildError::MissingFields(_))
        ));
        menu_mock.assert();
    }
}
