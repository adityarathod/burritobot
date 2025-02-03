use serde::Deserialize;

// Request structure is omitted since we use a single request structure for all requests.

/// Raw response from API.
#[derive(Deserialize)]
pub struct Response {
    pub data: Vec<Location>,
}

/// Raw location data from API.
#[derive(Deserialize)]
pub struct Location {
    #[serde(alias = "restaurantNumber")]
    pub id: i32,
    pub addresses: Vec<Address>,
}

/// Raw address data from API.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub postal_code: Option<String>,
    pub country_code: String,
}
