use bb_chipotle::{
    client::{Endpoint, EndpointConfig},
    locations::Location,
    ApiKey,
};
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short = 'k', long)]
    api_key: Option<String>,
    #[arg(short = 'e', long)]
    api_key_endpoint: Option<String>,
    #[arg(short = 'l', long)]
    locations_endpoint: Option<String>,
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
        menu: None,
        restaurant: args.locations_endpoint.map(|val| Endpoint {
            url: val,
            replace_token: None,
        }),
    };
    endpoints.validate().unwrap();
    let api_key = if let Some(key) = args.api_key.as_deref() {
        ApiKey::from_raw(key)
    } else {
        ApiKey::from_custom(&http, None).await.unwrap()
    };
    let client = bb_chipotle::Client::new(http, Some(endpoints), api_key).unwrap();
    let locations = client.get_all_locations().await.unwrap();
    println!(
        "{}",
        serde_json::to_string::<Vec<Location>>(&locations).unwrap()
    );
}
