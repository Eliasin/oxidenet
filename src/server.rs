use serde::{Deserialize, Serialize};

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use smol::net::unix::{UnixListener, UnixStream};
use smol::stream::StreamExt;

use crate::{
    client::ClientCommand,
    config::{Config, PingMonitorConfig},
    ping::{PingReading, PingReadingHistory, PingReadingQuery},
    util::{receive_length_prefixed_object_async, send_length_prefixed_object_async},
};

#[derive(Serialize, Deserialize)]
pub enum ServerResponse {
    UnknownTarget(String),
    PingQueryResult(HashMap<String, (Vec<PingReading>, PingMonitorConfig)>),
}

#[derive(Default, Debug)]
pub struct ServerState {
    pub ping_reading_histories: HashMap<String, Arc<Mutex<PingReadingHistory>>>,
    pub config: Config,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct TargetAndPingReadingQuery {
    pub target: Option<String>,
    pub query: PingReadingQuery,
}

fn query_ping_readings_for_targets<
    'a,
    I: Iterator<Item = (&'a String, &'a Arc<Mutex<PingReadingHistory>>)>,
>(
    target_readings: I,
    query: &PingReadingQuery,
    server_state: &ServerState,
) -> HashMap<String, (Vec<PingReading>, PingMonitorConfig)> {
    let mut results = HashMap::new();

    for (target, reading_history) in target_readings {
        let reading_history = reading_history.lock().unwrap();

        let target_result = query.query(reading_history.readings());

        results.insert(
            target.clone(),
            (
                target_result,
                *server_state
                    .config
                    .ping_monitor_configs()
                    .get(target)
                    .expect(""),
            ),
        );
    }

    results
}

async fn serve_client(mut stream: UnixStream, server_state: &ServerState) -> anyhow::Result<()> {
    loop {
        let command: anyhow::Result<ClientCommand> =
            receive_length_prefixed_object_async(&mut stream).await;
        match command {
            Ok(command) => match command {
                ClientCommand::TargetAndPingReadingQuery(TargetAndPingReadingQuery {
                    target,
                    query,
                }) => {
                    let result = if let Some(target) = target {
                        query_ping_readings_for_targets(
                            server_state
                                .ping_reading_histories
                                .get_key_value(&target)
                                .into_iter(),
                            &query,
                            server_state,
                        )
                    } else {
                        query_ping_readings_for_targets(
                            server_state.ping_reading_histories.iter(),
                            &query,
                            server_state,
                        )
                    };

                    send_length_prefixed_object_async(
                        &ServerResponse::PingQueryResult(result),
                        &mut stream,
                    )
                    .await?;
                }
                ClientCommand::Disconnect => {
                    return Ok(());
                }
            },
            Err(e) => anyhow::bail!(e),
        }
    }
}

pub async fn serve_query_server(server_state: ServerState) -> anyhow::Result<()> {
    if server_state.config.remove_existing_socket.unwrap_or(false) {
        let socket_path = Path::new(crate::UNIX_SOCKET_PATH);
        let _ = std::fs::remove_file(socket_path);
    }

    let listener = UnixListener::bind(crate::UNIX_SOCKET_PATH)?;

    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        if let Err(e) = serve_client(stream, &server_state).await {
            log::error!("Encountered error serving client: {e}");
        }
    }

    Ok(())
}
