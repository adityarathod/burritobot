use anyhow::Result;
use bb_chipotle::ApiKey;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short = 'e', long)]
    pub endpoint: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let http = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    let api_key = ApiKey::from_custom(&http, args.endpoint.as_deref()).await?;
    println!("{}", api_key.get());
    Ok(())
}
