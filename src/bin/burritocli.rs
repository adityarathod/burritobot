use std::time::Duration;

use anyhow::Result;
use bb_chipotle::{menu::Menu, ApiKey};
use clap::{Args, Parser, Subcommand};
use futures::{stream, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::json;
use tokio::time;

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

    #[clap(name = "get-all-menus", about = "Get menu for all locations")]
    AllMenus {
        #[command(flatten)]
        location_opts: LocationOpts,

        #[arg(short = 'm', long, help = "Menu endpoint")]
        menu_endpoint: Option<String>,

        #[arg(short = 'o', long, help = "Output file")]
        output_path: Option<String>,
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
        // i've only ran this once lol
        Command::AllMenus {
            location_opts,
            menu_endpoint,
            output_path,
        } => {
            let locations = bb_chipotle::locations::Locations::get_all_us_custom(
                &api_key,
                &http,
                location_opts.locations_endpoint.as_deref(),
            )
            .await?
            // TODO: figure out how to not do this
            .into_iter()
            .collect::<Vec<_>>();

            // Get menus in batches of 5
            let progress = ProgressBar::new(locations.len() as u64);
            progress.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap(),
            );
            let mut menus = Vec::new();
            let delay_between_batches = Duration::from_secs(1);
            for location_batch in locations.chunks(5) {
                let menu_batch = stream::iter(location_batch)
                    .map(|location| {
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
                    .buffer_unordered(5)
                    .collect::<Vec<_>>()
                    .await;
                menus.extend(menu_batch);
                progress.inc(location_batch.len() as u64);
                time::sleep(delay_between_batches).await;
            }
            progress.finish();
            let json_output = serde_json::to_string_pretty(&menus)?;
            if let Some(output_path) = output_path {
                std::fs::write(output_path, json_output)?;
            } else {
                println!("{}", json_output);
            }
        }
    }

    Ok(())
}
