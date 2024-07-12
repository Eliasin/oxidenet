use std::collections::HashSet;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use smol::process::Command;
use smol::Timer;

use crate::command_watcher::{watch, InputConsumptionResult};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PingReadingQuery {
    pub latency_higher_than: Duration,
    pub min_intensity: u32,
    pub max_window: Duration,
}

impl PingReadingQuery {
    pub fn new(
        latency_higher_than: Duration,
        min_intensity: u32,
        max_window: Duration,
    ) -> PingReadingQuery {
        PingReadingQuery {
            latency_higher_than,
            min_intensity,
            max_window,
        }
    }

    pub fn query(&self, readings: &[PingReading]) -> Vec<PingReading> {
        let mut reading_history: Vec<(bool, usize, &PingReading)> = vec![];
        let mut intensity = 0;
        let mut included_readings: HashSet<usize> = HashSet::new();

        for (i, reading) in readings.iter().enumerate() {
            if reading.latency > self.latency_higher_than {
                reading_history.push((true, i, reading));
                intensity += 1;
            } else {
                reading_history.push((false, i, reading));
            }

            while let Some(first_reading) = reading_history.first() {
                let time_since = reading.timestamp.duration_since(first_reading.2.timestamp);
                match time_since {
                    Ok(time_since) => {
                        if time_since > self.max_window {
                            let (over_threshold, _, _) = reading_history.remove(0);

                            if over_threshold {
                                intensity -= 1;
                            }
                        } else {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Time skip anomaly in ping reading, skipping: {e}",);
                        let (over_threshold, _, _) = reading_history.remove(0);

                        if over_threshold {
                            intensity -= 1;
                        }
                    }
                }
            }

            if intensity > self.min_intensity as usize {
                included_readings.extend(reading_history.iter().map(|(_, index, _)| index))
            }
        }

        included_readings
            .into_iter()
            .map(|index| readings.get(index).unwrap())
            .copied()
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct PingReading {
    latency: Duration,
    timestamp: SystemTime,
}

#[derive(Debug)]
pub struct PingReadingHistory {
    readings: Vec<PingReading>,
    max_readings: usize,
}

impl PingReadingHistory {
    fn new(interval_seconds: f32, history_length: Duration) -> Self {
        PingReadingHistory {
            readings: Default::default(),
            max_readings: Self::calculate_max_readings(interval_seconds, history_length),
        }
    }

    fn calculate_max_readings(interval_seconds: f32, history_length: Duration) -> usize {
        let history_length_ms = history_length.as_millis();
        let interval_millis = interval_seconds * 1000.0;

        usize::max((history_length_ms as f32 / interval_millis) as usize, 1)
    }

    fn parse_line_into_reading(line: &str) -> Option<PingReading> {
        /* We expect any ping reading lines to be in the form
         * ... time=NUMBER ms ...
         */
        if let Some(time_with_ms) = line.split("time=").nth(1) {
            if let Some(time) = time_with_ms.split("ms").nth(0) {
                if let Ok(time) = time.trim().parse::<f32>() {
                    return Some(PingReading {
                        latency: Duration::from_millis(time as u64),
                        timestamp: SystemTime::now(),
                    });
                }
            }
        }
        None
    }

    fn add_reading(&mut self, ping_reading: PingReading) {
        self.readings.push(ping_reading);

        if self.readings.len() > self.max_readings {
            self.readings.remove(0);
        }
    }

    pub fn add_output_line(&mut self, line: &str) {
        if let Some(reading) = PingReadingHistory::parse_line_into_reading(line) {
            self.add_reading(reading);
        }
    }

    pub fn readings(&self) -> &[PingReading] {
        &self.readings
    }
}

#[derive(Debug)]
pub struct PingMonitor {
    target_host: String,
    interval_seconds: f32,
    ping_reading_history: Arc<Mutex<PingReadingHistory>>,
}

impl PingMonitor {
    pub fn new(
        target_host: String,
        interval_seconds: f32,
        history_length: Duration,
    ) -> PingMonitor {
        PingMonitor {
            target_host,
            interval_seconds,
            ping_reading_history: Arc::new(Mutex::new(PingReadingHistory::new(
                interval_seconds,
                history_length,
            ))),
        }
    }

    pub fn target(&self) -> &str {
        &self.target_host
    }

    fn create_command(interval_seconds: f32, target_host: &str) -> Command {
        let mut c = Command::new("ping");
        c.arg("-i")
            .arg(format!("{:.5}", interval_seconds))
            .arg(target_host)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        c
    }

    async fn _watch(&mut self) {
        let ping_command = PingMonitor::create_command(self.interval_seconds, &self.target_host);

        watch(
            format!("ping to {}", self.target_host),
            ping_command,
            |line| {
                self.ping_reading_history
                    .lock()
                    .unwrap()
                    .add_output_line(line);
                InputConsumptionResult::Continue
            },
            |_| InputConsumptionResult::TerminateCommand {
                reason: String::from("ping failed"),
            },
        )
        .await;
    }

    pub async fn watch(&mut self) -> anyhow::Result<()> {
        loop {
            self._watch().await;
            Timer::after(Duration::from_secs(5)).await;
        }
    }

    pub fn readings(&self) -> Vec<PingReading> {
        let ping_reading_history = self.ping_reading_history.lock().unwrap();

        ping_reading_history.readings().to_vec().clone()
    }

    pub fn reading_history(&self) -> Arc<Mutex<PingReadingHistory>> {
        self.ping_reading_history.clone()
    }
}
