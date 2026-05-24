use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub seed: u64,
    pub message_count: u32,
    /// Delay between messages in milliseconds.
    pub interval_ms: u64,
    /// MIDI port index. If absent, the program prompts interactively.
    pub midi_port: Option<usize>,
    /// Receiver watchdog: exit if no message arrives within this many milliseconds.
    pub watchdog_ms: Option<u64>,
}
