use anyhow::Result;
use bb_chipotle::{menu::Menu, ApiKey};
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use tokio_stream::StreamExt;

#[derive(Parser, Debug)]
struct CliArgs {
    #[command(subcommand)]
    pub subcommand: Command,

    #[command(flatten)]
    pub global_opts: GlobalOpts,
}

#[derive(Args, Debug)]
struct GlobalOpts {
    #[arg(short = 'a', long, global = true)]
    pub api_key_endpoint: Option<String>,

    #[arg(short = 'k', long, conflicts_with = "api_key_endpoint", global = true)]
    pub api_key: Option<String>,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Command {
    #[clap(name = "get-api-key")]
    ApiKey,

    #[clap(name = "get-all-locations", about = "Get all US locations")]
    AllLocations {
        #[command(flatten)]
        location_opts: LocationOpts,
    },

    #[clap(name = "get-menu", about = "Get menu for locations by ZIP code")]
    Menu {
        #[command(flatten)]
        location_opts: LocationOpts,

        #[arg(short = 'm', long, help = "Menu endpoint")]
        menu_endpoint: Option<String>,

        #[arg(short = 'z', long, help = "ZIP code")]
        zip_code: String,
    },
}

#[derive(Args, Debug, PartialEq)]
struct LocationOpts {
    #[arg(short = 'l', long, help = "Endpoint for retrieving locations")]
    pub locations_endpoint: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();
    let http = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    let api_key = if let Some(key) = args.global_opts.api_key.as_deref() {
        ApiKey::from_raw(key)
    } else {
        ApiKey::get_custom(&http, args.global_opts.api_key_endpoint.as_deref()).await?
    };

    match args.subcommand {
        Command::ApiKey => {
            println!("{}", api_key.get());
        }
        Command::AllLocations { location_opts } => {
            let locations = bb_chipotle::locations::Locations::get_all_us_custom(
                &api_key,
                &http,
                location_opts.locations_endpoint.as_deref(),
            )
            .await?;
            println!(
                "{}",
                serde_json::to_string::<bb_chipotle::locations::Locations>(&locations)?
            );
        }
        Command::Menu {
            location_opts,
            menu_endpoint,
            zip_code,
        } => {
            let locations = bb_chipotle::locations::Locations::get_all_us_custom(
                &api_key,
                &http,
                location_opts.locations_endpoint.as_deref(),
            )
            .await?
            .into_iter()
            .filter(|location| location.zip_code == zip_code.as_str());
            let menus = tokio_stream::iter(locations)
                .then(|location| {
                    // i have no idea what i'm doing with this :(
                    let api_key = api_key.clone();
                    let http = http.clone();
                    let menu_endpoint = menu_endpoint.clone();
                    async move {
                        let menu = Menu::get_custom(
                            &location.id,
                            &api_key,
                            &http,
                            menu_endpoint.as_deref(),
                        )
                        .await
                        .unwrap();
                        json!({"location": location, "menu": menu})
                    }
                })
                .collect::<Vec<_>>()
                .await;
            println!("{}", serde_json::to_string_pretty(&menus)?);
        }
    }

    Ok(())
}
