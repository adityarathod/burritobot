use derive_builder::Builder;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{api_interfaces::menu, error::GetError, util::default_http_client, ApiKey};
use super::constants::API_KEY_HEADER;

const DEFAULT_MENU_SERVICE_URL_FORMAT: &str = 
"https://services.chipotle.com/menuinnovation/v1/restaurants/$store/onlinemenu?channelId=web&includeUnavailableItems=true";

pub const MENU_SERVICE_URL_REPLACE_TOKEN : &str = "$store";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Price {
    pub normal_price: f32,
    pub delivery_price: f32,
}

impl PartialEq for Price {
    fn eq(&self, other: &Self) -> bool {
        let normal_price_within_epsilon =
            (self.normal_price - other.normal_price).abs() < f32::EPSILON;
        let delivery_price_within_epsilon =
            (self.delivery_price - other.delivery_price).abs() < f32::EPSILON;

        normal_price_within_epsilon && delivery_price_within_epsilon
    }
}

impl Eq for Price {}

impl TryFrom<menu::Item> for Price {
    type Error = ();

    fn try_from(item: menu::Item) -> Result<Self, ()> {
        Ok(Self {
            normal_price: item.unit_price,
            delivery_price: item.unit_delivery_price,
        })
    }
}

// TODO: Add more fields as needed
#[derive(Builder, Debug, Serialize, Deserialize, PartialEq)]
pub struct Menu {
    pub veggie_bowl_price: Price,
    pub chicken_bowl_price: Price,
    pub steak_bowl_price: Price,
}

impl Menu {
    /// Get the summarized menu from the menu service.
    pub async fn get(restaurant_id: &i32, key: &ApiKey) -> Result<Self, GetError> {
        let client = default_http_client();
        Self::get_custom(restaurant_id, key, &client, None).await
    }

    /// Get the summarized menu from the menu service with a custom HTTP client and endpoint.
    pub async fn get_custom(restaurant_id: &i32, key: &ApiKey, client: &Client, endpoint: Option<&str>) -> Result<Self, GetError> {
        let complete_endpoint = endpoint.unwrap_or(DEFAULT_MENU_SERVICE_URL_FORMAT)
            .replace(MENU_SERVICE_URL_REPLACE_TOKEN, &restaurant_id.to_string());
        let response = client.get(&complete_endpoint)
            .header(API_KEY_HEADER, key.get())
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(GetError::ResponseError(response.status()));
        }
        let body = response.text().await.map_err(GetError::ResponseBodyError)?;
        let parsed_body: menu::Response = serde_json::from_str(&body)?;
        Menu::try_from(parsed_body)
    }

}

impl TryFrom<menu::Response> for Menu {
    type Error = GetError;

    fn try_from(response: menu::Response) -> Result<Self, GetError> {
        let mut builder = MenuBuilder::default();

        for entree in response.entrees {
            if builder.chicken_bowl_price.is_some()
                && builder.veggie_bowl_price.is_some()
                && builder.steak_bowl_price.is_some()
            {
                break;
            }
            if entree.item_type.to_lowercase() != "bowl" {
                continue;
            }
            let item_name = entree.item_name.clone();
            let price = Price::try_from(entree).expect("Failed to convert entree to price");
            match item_name.to_lowercase().replace("bowl", "").trim() {
                "veggie" => {
                    builder.veggie_bowl_price(price);
                }
                "chicken" => {
                    builder.chicken_bowl_price(price);
                }
                "steak" => {
                    builder.steak_bowl_price(price);
                }
                _ => {}
            }
        }
        Ok(builder.build()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    const FAKE_API_KEY: &str = "fake_api_key";

    #[tokio::test]
    async fn get_success() {
        // Arrange
        let server = MockServer::start_async().await;
        let response_json = json!({
            "restaurantId": 1234, 
            "entrees": [
                {
                    "itemCategory": "Entree",
                    "itemType": "Bowl",
                    "itemId": "1",
                    "itemName": "Veggie Bowl",
                    "unitPrice": 7.99,
                    "unitDeliveryPrice": 8.99
                },
                {
                    "itemCategory": "Entree",
                    "itemType": "Bowl",
                    "itemId": "2",
                    "itemName": "Chicken Bowl",
                    "unitPrice": 8.99,
                    "unitDeliveryPrice": 9.99
                },
                {
                    "itemCategory": "Entree",
                    "itemType": "Bowl",
                    "itemId": "3",
                    "itemName": "Steak Bowl",
                    "unitPrice": 9.99,
                    "unitDeliveryPrice": 10.99
                }
            ],
            "sides": []
        });

        let menu_mock = server
            .mock_async(|when, then| {
                when.path("/").header(API_KEY_HEADER, FAKE_API_KEY);
                then.status(200).json_body(response_json);
            })
            .await;
        let url = server.url("/");
        let client = reqwest::Client::new();
        let api_key = ApiKey::from_raw(FAKE_API_KEY);

        // Act
        let menu = Menu::get_custom(&1234, &api_key, &client, Some(url.as_str())).await;

        // Assert
        assert!(menu.is_ok(), "Failed to get menu: {:?}", menu.unwrap_err());
        let menu = menu.unwrap();
        assert_eq!(menu.veggie_bowl_price, Price {
            normal_price: 7.99,
            delivery_price: 8.99,
        });
        assert_eq!(menu.chicken_bowl_price, Price {
            normal_price: 8.99,
            delivery_price: 9.99,
        });
        assert_eq!(menu.steak_bowl_price, Price {
            normal_price: 9.99,
            delivery_price: 10.99,
        });
        menu_mock.assert();
    }
    
}