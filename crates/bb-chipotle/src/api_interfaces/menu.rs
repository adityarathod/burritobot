use serde::Deserialize;

// Request structure is omitted since we use a single request structure for all requests.

/// Raw restaurant menu data from API.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub restaurant_id: i32,
    pub entrees: Vec<Item>,
    pub sides: Vec<Item>,
}

/// Raw item from API.
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
