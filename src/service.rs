use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ping::PingReadingHistory;
use crate::server::{serve_query_server, ServerState};
use crate::{config::Config, ping::PingMonitor};

async fn run_ping_monitors(ping_monitors: Vec<PingMonitor>) {
    let ping_monitor_tasks: Vec<smol::Task<(String, anyhow::Result<()>)>> = ping_monitors
        .into_iter()
        .map(|mut ping_monitor| {
            smol::spawn(async move {
                let target = ping_monitor.target().to_string();
                if let Err(e) = ping_monitor.watch().await {
                    (target, Err(e))
                } else {
                    (target, Ok(()))
                }
            })
        })
        .collect();

    for ping_monitor_task in ping_monitor_tasks {
        let (target, maybe_error) = ping_monitor_task.await;
        match maybe_error {
            Ok(_) => log::error!("Ping monitor task for {target} stopped unexpectedly"),
            Err(e) => {
                log::error!("Ping monitor task for {target} stopped unexpectedly with error: {e}",)
            }
        }
    }
}

pub async fn run_service(config: Config) -> anyhow::Result<()> {
    let ping_monitors = config.ping_monitors();

    let reading_histories: HashMap<String, Arc<Mutex<PingReadingHistory>>> = ping_monitors
        .iter()
        .map(|monitor| (monitor.target().to_string(), monitor.reading_history()))
        .collect();

    let server_state = ServerState {
        ping_reading_histories: reading_histories,
    };

    let run_ping_monitors_task = smol::spawn(run_ping_monitors(ping_monitors));

    let serve_query_server_task = smol::spawn(serve_query_server(server_state));

    run_ping_monitors_task.await;
    serve_query_server_task.await?;

    Ok(())
}
