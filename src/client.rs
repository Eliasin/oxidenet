use serde::{Deserialize, Serialize};

use crate::{ping::PingReadingQuery, server::ServerResponse, Query};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ClientCommand {
    TargetAndPingReadingQuery(Option<String>, PingReadingQuery),
    Disconnect,
}

pub fn send_query(query: Query) {
    reqwest::get("http://localhost:30")
}
