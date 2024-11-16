use bb_chipotle::{api_key, locations};
use clap::Parser;
use serde_json::json;

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
        short = 'l',
        long,
        help = "Endpoint to retrieve locations from. Defaults to the current Chipotle API."
    )]
    locations_endpoint: Option<String>,
    #[arg(short = 'o', long, help = "Output path to write locations to.")]
    output_path: Option<String>,
}

async fn get_api_key(
    client: &reqwest::Client,
    api_key: Option<String>,
    api_key_endpoint: Option<String>,
) -> String {
    let result = match api_key {
        Some(api_key) => Ok(api_key),
        None => {
            let endpoint = api_key_endpoint.as_deref();
            api_key::get(client, endpoint).await
        }
    };
    match result {
        Ok(api_key) => api_key,
        Err(api_key_err) => {
            println!(
                "{}",
                json!({"error": format!("Failed to get API key: {:?}", api_key_err)})
            );
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    let api_key = get_api_key(&client, args.api_key, args.api_key_endpoint).await;
    let locations = locations::get(&client, &api_key, args.locations_endpoint.as_deref()).await;
    if let Err(get_err) = locations {
        println!(
            "{}",
            json!({"error": format!("Failed to get locations: {:?}", get_err)})
        );
        std::process::exit(1);
    }
    let locations_json = locations
        .map(|val| serde_json::to_string::<Vec<locations::Location>>(val.as_ref()).unwrap())
        .unwrap();
    if let Some(output_path) = args.output_path {
        if let Ok(metadata) = tokio::fs::metadata(&output_path).await {
            if !metadata.is_file() {
                println!(
                    "{}",
                    json!({"error": format!("Output path exists and is not a file: {}", output_path)})
                );
                std::process::exit(1);
            }
        }
        if let Err(err) = tokio::fs::write(&output_path, &locations_json).await {
            println!(
                "{}",
                json!({"error": format!("Failed to write output: {:?}", err)})
            );
            std::process::exit(1);
        }
    } else {
        println!("{}", locations_json);
    }
}
