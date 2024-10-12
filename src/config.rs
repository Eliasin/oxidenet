use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Serialize};

use crate::ping::PingMonitor;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct PingMonitorConfig {
    #[serde(rename = "interval-seconds")]
    pub interval_seconds: f32,
    #[serde(rename = "history-length-hours")]
    pub history_length_hours: f32,
}

type Target = String;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    #[serde(rename = "ping-monitors")]
    ping_monitors: HashMap<Target, PingMonitorConfig>,
    #[serde(rename = "remove-existing-socket")]
    pub remove_existing_socket: Option<bool>,
}

impl Config {
    pub fn ping_monitor_configs(&self) -> &HashMap<Target, PingMonitorConfig> {
        &self.ping_monitors
    }

    pub fn ping_monitors(&self) -> Vec<PingMonitor> {
        self.ping_monitors
            .iter()
            .map(
                |(
                    target,
                    PingMonitorConfig {
                        interval_seconds,
                        history_length_hours,
                    },
                )| {
                    let history_length =
                        Duration::from_secs_f32(history_length_hours * 60_f32 * 60_f32);
                    PingMonitor::new(target.clone(), *interval_seconds, history_length)
                },
            )
            .collect()
    }
}
