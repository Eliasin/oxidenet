use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::os::unix::net::UnixStream;

use crate::ping::PingReading;
use crate::server::{ServerResponse, TargetAndPingReadingQuery};
use crate::util::{receive_length_prefixed_object, send_length_prefixed_object};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ClientCommand {
    TargetAndPingReadingQuery(TargetAndPingReadingQuery),
    Disconnect,
}

pub fn send_client_command(command: ClientCommand) -> anyhow::Result<ServerResponse> {
    let mut stream = UnixStream::connect(crate::UNIX_SOCKET_PATH)?;

    send_length_prefixed_object(&command, &mut stream)?;
    let response = receive_length_prefixed_object(&mut stream)?;

    send_length_prefixed_object(&ClientCommand::Disconnect, &mut stream)?;

    Ok(response)
}

pub fn display_ping_query_results(results: &HashMap<String, Vec<PingReading>>) {
    for (target, readings) in results {
        println!("{target}");

        for reading in readings {
            println!("{reading:?}");
        }
    }

    println!("========== {} targets found ==========", results.len());
}
