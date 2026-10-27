use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub seed: u64,
    /// Delay between messages in milliseconds.
    pub interval_ms: u64,
    /// Receiver watchdog: exit if no message arrives within this many milliseconds.
    pub watchdog_ms: Option<u64>,
}
