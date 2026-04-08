use log::{Level, Metadata, Record};

pub fn init(log_level: &str) {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();

    // Configure the logger
    match log_level {
        "Normal" => log::set_max_level(log::LevelFilter::Info),
        "Debug" => log::set_max_level(log::LevelFilter::Debug),
        "Verbose" => log::set_max_level(log::LevelFilter::Trace),
        _ => log::set_max_level(log::LevelFilter::Info),
    }
    info!("Logger Initialized.");
    info!("Kernel log mode: {}", log_level);
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        // Implement the logger with serial output
        let metadata = record.metadata();
        if self.enabled(metadata) {
            match metadata.level() {
                Level::Error => println!(
                    "[\x1b[1;31m{}\x1b[0m]: {}",
                    record.level().as_str(),
                    record.args()
                ),
                Level::Warn => {
                    println!(
                        "[\x1b[1;33m{}\x1b[0m ]: {}",
                        record.level().as_str(),
                        record.args()
                    )
                }
                Level::Info => println!(
                    "[\x1b[1;32m{}\x1b[0m ]: {}",
                    record.level().as_str(),
                    record.args()
                ),
                Level::Debug => println!(
                    "[\x1b[1;34m{}\x1b[0m]: {}",
                    record.level().as_str(),
                    record.args()
                ),
                Level::Trace => println!(
                    "[\x1b[2;37m{}\x1b[0m], {}",
                    record.level().as_str(),
                    record.args()
                ),
            }
        }
    }

    fn flush(&self) {}
}
