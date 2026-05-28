use log::{Level, LevelFilter, Log, Metadata, Record};
use rtt_target::{rprintln, rtt_init_print};
const fn parse_log_level(s: &[u8]) -> LevelFilter {
    match s {
        b"error" => LevelFilter::Error,
        b"warn" => LevelFilter::Warn,
        b"info" => LevelFilter::Info,
        b"debug" => LevelFilter::Debug,
        b"trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    }
}
pub const LOG_LEVEL: LevelFilter = match option_env!("LOG_LEVEL") {
    Some(s) => parse_log_level(s.as_bytes()),
    None => LevelFilter::Info,
};

pub struct RttLogger {
    level: LevelFilter,
}

impl RttLogger {
    pub const fn new(level: LevelFilter) -> Self {
        RttLogger { level }
    }
    pub fn init(&'static self) {
        rtt_init_print!();
        log::set_logger(self).expect("Failed to set logger");
        log::set_max_level(self.level);
    }

    fn color_for(level: Level) -> &'static str {
        match level {
            Level::Error => "\x1B[31m", // Red
            Level::Warn => "\x1B[33m",  // Yellow
            Level::Info => "\x1B[32m",  // Green
            Level::Debug => "\x1B[36m", // Cyan
            Level::Trace => "\x1B[90m", // Bright Black (gray)
        }
    }
}

impl Log for RttLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let color = Self::color_for(record.level());
            let reset = "\x1B[0m";
            rprintln!(
                "{}[{}]{} {} - {}",
                color,
                record.level(),
                reset,
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
