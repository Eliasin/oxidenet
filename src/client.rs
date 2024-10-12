use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::time::{Duration, SystemTime};

use crate::config::PingMonitorConfig;
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

pub struct PingQueryResultDisplayOptions {
    pub display_skip_warning_threshold: Option<Duration>,
    pub time_format: Option<String>,
    pub show_original_line: bool,
}

fn display_ping_reading(reading: &PingReading, options: &PingQueryResultDisplayOptions) {
    if options.show_original_line {
        println!("{}", reading.original_line.trim_end());
    } else {
        println!("{} ms", reading.latency.as_millis());
    }
}

fn display_current_time(time: SystemTime, options: &PingQueryResultDisplayOptions) {
    let chrono_time = DateTime::<Local>::from(time);

    let format_string = options.time_format.as_ref().map_or("%F %X", |s| s.as_str());

    println!("===== At {} =====", chrono_time.format(format_string));
}

fn display_readings_for_target(
    target: &str,
    readings: &[PingReading],
    monitor_config: &PingMonitorConfig,
    options: &PingQueryResultDisplayOptions,
) {
    println!("Target: {target}");

    let mut errors: Vec<String> = vec![];
    let display_skip_warning_threshold =
        options
            .display_skip_warning_threshold
            .unwrap_or(Duration::from_secs_f32(
                monitor_config.interval_seconds * 1.1,
            ));

    let mut last_reading_time = None;
    for reading in readings {
        if let Some(last_reading_time) = last_reading_time {
            match reading.timestamp.duration_since(last_reading_time) {
                Ok(time_since_last_reading) => {
                    if time_since_last_reading > display_skip_warning_threshold {
                        display_current_time(reading.timestamp, options);
                    }
                }
                Err(e) => {
                    errors.push(format!("Error calculating time between timestamps for {last_reading_time:?} and {:?}, {}", reading.timestamp, e));
                }
            }
        } else {
            display_current_time(reading.timestamp, options);
        }

        display_ping_reading(reading, options);
        last_reading_time = Some(reading.timestamp);
    }
}

pub fn display_ping_query_results(
    results: &HashMap<String, (Vec<PingReading>, PingMonitorConfig)>,
    options: &PingQueryResultDisplayOptions,
) {
    for (target, (readings, monitor_config)) in results {
        display_readings_for_target(target, readings, monitor_config, options);
    }

    println!(">>>>>>>>>> {} target(s) found <<<<<<<<<<", results.len());
}
