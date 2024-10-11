use std::{path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};
use client::{display_ping_query_results, send_client_command, ClientCommand};
use ping::PingReadingQuery;
use server::{ServerResponse, TargetAndPingReadingQuery};
use smol::io::AsyncReadExt;
use smol_macros::main;

mod anomaly;
mod client;
mod command_watcher;
mod config;
mod monitor;
mod ping;
mod server;
mod service;
mod util;

pub const UNIX_SOCKET_PATH: &str = "/tmp/oxidenet";

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Query {
    #[clap(about = "query ping logs")]
    Ping {
        #[arg(long, short, help = "filter by target, comma separated list, optional")]
        target: Option<String>,
        #[arg(long, short, help = "latency threshold to filter with, in ms")]
        latency_higher_than: u32,
        #[arg(
            long,
            short = 'i',
            help = "minimum instances of ping higher than threshold that is added to query results"
        )]
        min_intensity: u32,
        #[arg(
            long,
            short = 'w',
            help = "maximum size of time window that is remembered when traversing ping logs for ping over the threshold, in seconds"
        )]
        max_window: u32,
    },
}

#[derive(Subcommand, Debug)]
enum Command {
    #[clap(about = "run monitor service")]
    Service {
        #[arg(long)]
        config: PathBuf,
    },
    #[clap(about = "query monitor service")]
    Query {
        #[command(subcommand)]
        query: Query,
    },
}

fn run_query(query: Query) -> anyhow::Result<()> {
    match query {
        Query::Ping {
            target,
            latency_higher_than,
            min_intensity,
            max_window,
        } => {
            let query = PingReadingQuery::new(
                Duration::from_millis(latency_higher_than.into()),
                min_intensity,
                Duration::from_secs(max_window.into()),
            );

            let server_response = send_client_command(ClientCommand::TargetAndPingReadingQuery(
                TargetAndPingReadingQuery { target, query },
            ))?;

            match server_response {
                ServerResponse::PingQueryResult(results) => {
                    display_ping_query_results(&results);
                }
                ServerResponse::UnknownTarget(target) => {
                    println!("Server reply: Unknown target {target}");
                }
            }
        }
    }
    Ok(())
}

main! {
    async fn main() -> anyhow::Result<()> {
        env_logger::init();

        let args = Args::parse();

        match args.command {
            Command::Service { config } => {

                let mut buf = String::new();
                let mut config_file = smol::fs::File::open(config).await?;
                config_file.read_to_string(&mut buf).await?;

                let config: config::Config = toml::from_str(&buf)?;
                service::run_service(config).await
            },
            Command::Query { query } => {
                run_query(query)
            },
        }
    }
}
