use thiserror::Error;

use crate::{error::GetError, locations, menu, ApiKey};

#[derive(Clone, Debug)]
pub struct Client {
    http_client: reqwest::Client,
    endpoints: Option<EndpointConfig>,
    api_key: ApiKey,
}

#[derive(Clone, Debug)]
pub struct EndpointConfig {
    pub menu: Option<Endpoint>,
    pub restaurant: Option<Endpoint>,
}

#[derive(Debug, Error)]
pub enum EndpointConfigError {
    #[error("missing replace token for endpoint {0} (url: {1})")]
    MissingReplaceToken(String, String),
    #[error("unnecessary replace token `${1}` provided in endpoint {0}")]
    UnnecessaryReplaceToken(String, String),
}

impl EndpointConfig {
    pub fn validate(&self) -> Result<(), EndpointConfigError> {
        if let Some(menu) = &self.menu {
            if menu.replace_token.is_none() {
                return Err(EndpointConfigError::MissingReplaceToken(
                    "menu".to_string(),
                    menu.url.clone(),
                ));
            }
        }
        if let Some(restaurant) = &self.restaurant {
            if restaurant.replace_token.is_some() {
                return Err(EndpointConfigError::UnnecessaryReplaceToken(
                    "restaurant".to_string(),
                    restaurant.replace_token.clone().unwrap(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub url: String,
    pub replace_token: Option<String>,
}

#[derive(Debug, Error)]
pub enum ClientInitError {
    #[error("invalid endpoint configuration: {0}")]
    InvalidEndpointConfig(#[from] EndpointConfigError),
}

impl Client {
    pub fn new(
        http_client: reqwest::Client,
        endpoints: Option<EndpointConfig>,
        api_key: ApiKey,
    ) -> Result<Self, ClientInitError> {
        if let Some(endpoints) = &endpoints {
            endpoints.validate()?
        }
        Ok(Self {
            http_client,
            endpoints,
            api_key,
        })
    }

    pub async fn get_all_locations(&self) -> Result<Vec<locations::Location>, GetError> {
        let url = self
            .endpoints
            .as_ref()
            .and_then(|endpoints| endpoints.restaurant.as_ref())
            .map(|endpoint| endpoint.url.clone());
        locations::get(&self.http_client, self.api_key.get(), url.as_deref()).await
    }

    pub async fn get_menu_summary(&self, restaurant_id: i32) -> Result<menu::Summary, GetError> {
        let url = self
            .endpoints
            .as_ref()
            .and_then(|endpoints| endpoints.menu.as_ref())
            .and_then(|endpoint| {
                endpoint.replace_token.as_ref().map(|token| menu::Endpoint {
                    url: endpoint.url.clone(),
                    replace_token: token.clone(),
                })
            });
        menu::get(&restaurant_id, &self.http_client, self.api_key.get(), url).await
    }
}
