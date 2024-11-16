use bb_chipotle::api_key;
use clap::Parser;
use serde_json::json;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short = 'e', long, default_value = api_key::DEFAULT_API_SOURCE_URL)]
    endpoint: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    let api_key = api_key::get(&client, args.endpoint.as_deref()).await;
    if api_key.is_err() {
        println!(
            "{}",
            json!({
                "error": format!("Failed to get API key: {:?}", api_key.err().unwrap()),
            })
        );
        return;
    }
    let api_key = api_key.unwrap();
    println!("{}", json!({ "api_key": api_key }));
}
