use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

pub fn init(log_level: LevelFilter) {
    let encoder = PatternEncoder::new("{h({l})} - {m}{n}");
    let console = ConsoleAppender::builder()
        .encoder(Box::new(encoder))
        .target(Target::Stderr)
        .build();
    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .build(Root::builder().appender("console").build(log_level))
        .unwrap();

    log4rs::init_config(config).unwrap();
}
