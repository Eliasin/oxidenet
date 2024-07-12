use serde::{Deserialize, Serialize};
use serde_json::json;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::ping::{PingReading, PingReadingHistory, PingReadingQuery};

#[derive(Serialize, Deserialize)]
pub enum ServerResponse {
    UnknownTarget(String),
    PingQueryResult(HashMap<String, Vec<PingReading>>),
}

#[derive(Default, Debug)]
pub struct ServerState {
    pub ping_reading_histories: HashMap<String, Arc<Mutex<PingReadingHistory>>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TargetAndPingReadingQuery {
    target: Option<String>,
    query: PingReadingQuery,
}

pub async fn serve_query_server(server_state: ServerState) {
    use async_compat::Compat;
    use warp::Filter;

    let server_state = Box::leak(Box::new(server_state));

    let app =
        warp::path("pings")
            .and(warp::body::json())
            .map(|ping_query: TargetAndPingReadingQuery| {
                let mut reading_results: HashMap<String, Vec<PingReading>> = HashMap::new();

                let queried_histories: Vec<(String, Vec<PingReading>)> = match ping_query.target {
                    Some(target) => match server_state.ping_reading_histories.get(&target) {
                        Some(ping_reading_history) => {
                            vec![(
                                target,
                                ping_reading_history.lock().unwrap().readings().to_vec(),
                            )]
                        }
                        None => {
                            return warp::reply::json(&json!({
                                "error": format!("Unknown target: {target}")
                            }))
                        }
                    },
                    None => server_state
                        .ping_reading_histories
                        .iter()
                        .map(|(target, history)| {
                            (target.clone(), history.lock().unwrap().readings().to_vec())
                        })
                        .collect(),
                };

                for (target, queried_history) in queried_histories {
                    reading_results.insert(target, ping_query.query.query(&queried_history));
                }

                warp::reply::json(&reading_results)
            });

    Compat::new(warp::serve(app).run(([127, 0, 0, 1], 3031))).await;
}
