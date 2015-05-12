// adapted from simple_logger
// https://github.com/borntyping/rust-simple_logger

use log;
use log::{Log,LogLevel,LogLevelFilter,LogMetadata,LogRecord,SetLoggerError};
use time;

const LEVEL: LogLevel = LogLevel::Info;
const FILTER: LogLevelFilter = LogLevelFilter::Info;

struct SaladaLogger;

impl Log for SaladaLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LEVEL
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!(
                "{} {:<5} [{}] {}",
                time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).unwrap(),
                record.level().to_string(),
                record.location().module_path(),
                record.args());
        }
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(FILTER);
        Box::new(SaladaLogger)
    })
}
