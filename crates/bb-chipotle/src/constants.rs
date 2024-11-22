/// The default URL to retrieve the API key from
pub const DEFAULT_API_KEY_SOURCE_URL: &str =
    "https://orderweb-prd-centralus-cdne.azureedge.net/js/app.js";

/// The default endpoint for the Chipotle restaurant info service
pub const DEFAULT_RESTAURANT_SERVICE_URL: &str =
    "https://services.chipotle.com/restaurant/v3/restaurant/";

/// The default endpoint format for the Chipotle menu service
pub const DEFAULT_MENU_SERVICE_URL_FORMAT: &str = "https://services.chipotle.com/menuinnovation/v1/restaurants/$store_id/onlinemenu?channelId=web&includeUnavailableItems=true";
pub const DEFAULT_MENU_SERVICE_URL_REPLACE_TOKEN: &str = "$store_id";

/// The header to use to send API keys in requests
pub const API_KEY_HEADER: &str = "Ocp-Apim-Subscription-Key";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_service_menu_url_format_has_token() {
        assert!(DEFAULT_MENU_SERVICE_URL_FORMAT.contains(DEFAULT_MENU_SERVICE_URL_REPLACE_TOKEN));
    }
}
