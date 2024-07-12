use std::{path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};
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

async fn test() {
    let mut ping_monitor =
        ping::PingMonitor::new("8.8.8.8".to_string(), 1.0, Duration::from_secs(60 * 60));

    let reading_history = ping_monitor.reading_history();

    smol::spawn(async move { ping_monitor.watch().await }).detach();

    let query = ping::PingReadingQuery::new(Duration::from_millis(20), 5, Duration::from_secs(20));

    loop {
        smol::Timer::after(std::time::Duration::from_secs(5)).await;
        let results = query.query(reading_history.lock().unwrap().readings());

        println!("{:?}", results);
    }
}

async fn run_query(query: Query) -> anyhow::Result<()> {
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
                run_query(query).await
            },
        }
    }
}
