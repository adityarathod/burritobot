use bb_chipotle::{
    client::{Client, Endpoint, EndpointConfig},
    ApiKey,
};
use clap::Parser;
use serde_json::json;
use tokio_stream::{self, StreamExt};

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short = 'k',
        long,
        help = "API key to use. If not provided, one will be fetched from the API key endpoint."
    )]
    api_key: Option<String>,
    #[arg(
        short = 'a',
        long,
        help = "Endpoint to extract API key from. Defaults to the current Chipotle API."
    )]
    api_key_endpoint: Option<String>,
    #[arg(
        short = 'e',
        help = "Endpoint to retrieve locations from. Defaults to the current Chipotle API."
    )]
    menu_endpoint: Option<String>,
    #[arg(
        short = 'c',
        long = "zip-code",
        help = "Zip code to search for locations near."
    )]
    zip_code: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let http = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    let endpoints = EndpointConfig {
        menu: args.menu_endpoint.map(|val| Endpoint {
            url: val,
            replace_token: None,
        }),
        restaurant: None,
    };
    endpoints.validate().unwrap();
    let api_key = if let Some(key) = args.api_key.as_deref() {
        ApiKey::from_raw(key)
    } else {
        ApiKey::from_custom(&http, None).await.unwrap()
    };
    let client = Client::new(http, Some(endpoints), api_key).unwrap();
    let retrieved_locations = client
        .get_all_locations()
        .await
        .unwrap()
        .into_iter()
        .filter(|location| location.zip_code == args.zip_code);
    let locations = tokio_stream::iter(retrieved_locations)
        .then(|location| {
            let client = client.clone();
            async move {
                let menu = client.get_menu_summary(location.id).await.unwrap();
                json!({"location": location, "menu": menu})
            }
        })
        .collect::<Vec<_>>()
        .await;

    println!("{}", serde_json::to_string_pretty(&locations).unwrap());
}
