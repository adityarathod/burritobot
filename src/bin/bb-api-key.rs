use anyhow::Result;
use bb_chipotle::client::{Endpoint, EndpointConfig};
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short = 'e', long)]
    endpoint: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let http = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    let endpoints = EndpointConfig {
        api_key: args.endpoint.map(|val| Endpoint {
            url: val,
            replace_token: None,
        }),
        menu: None,
        restaurant: None,
    };
    let mut client = bb_chipotle::Client::new(http, Some(endpoints), None)?;
    let api_key = client.load_api_key(false).await?;
    println!("{}", api_key);
    Ok(())
}
