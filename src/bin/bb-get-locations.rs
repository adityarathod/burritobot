use bb_chipotle::{
    client::{Endpoint, EndpointConfig},
    locations::Location,
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
    let http_client = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    let endpoints = EndpointConfig {
        api_key: args.api_key_endpoint.map(|val| Endpoint {
            url: val,
            replace_token: None,
        }),
        menu: None,
        restaurant: args.locations_endpoint.map(|val| Endpoint {
            url: val,
            replace_token: None,
        }),
    };
    endpoints.validate().unwrap();
    let mut client = bb_chipotle::Client::new(http_client, Some(endpoints), None).unwrap();
    client.load_api_key(true).await.unwrap();
    let locations = client.get_all_locations().await.unwrap();
    println!(
        "{}",
        serde_json::to_string::<Vec<Location>>(&locations).unwrap()
    );
}
