use std::str::FromStr;

use fern::colors::Color;
use fern::colors::ColoredLevelConfig;

pub enum Level {
    Warn,
    Info,
    Debug,
    Trace,
}

impl FromStr for Level {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "warn" => Ok(Level::Warn),
            "info" => Ok(Level::Info),
            "debug" => Ok(Level::Debug),
            "trace" => Ok(Level::Trace),
            _ => unreachable!(),
        }
    }
}

pub fn init(verbosity: &Level) {
    // std::env::set_var("RUST_LOG", "debug");

    let levels = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Blue)
        .debug(Color::Magenta)
        .trace(Color::White);

    let mut logger = fern::Dispatch::new();

    logger = logger.format(move |out, message, record| {
        out.finish(format_args!(
            "{b}{time}{r} {l}{kind:<5}{r} {c}{name}{r} {l}{message}{r}",
            l = format_args!("\x1B[{}m", levels.get_color(&record.level()).to_fg_str()),
            b = format_args!("\x1B[{}m", Color::BrightBlack.to_fg_str()),
            c = format_args!("\x1B[{}m", Color::Cyan.to_fg_str()),
            r = "\x1B[0m",
            time = chrono::Local::now().format("[%Y-%m-%d %H:%M:%S.%3f]"),
            kind = record.level(),
            name = record.target(),
            message = message,
        ))
    });

    logger = match verbosity {
        Level::Warn => logger.level_for("tt2", log::LevelFilter::Warn),
        Level::Info => logger.level_for("tt2", log::LevelFilter::Info),
        Level::Debug => logger.level_for("tt2", log::LevelFilter::Debug),
        _ => logger.level_for("tt2", log::LevelFilter::Trace),
    };

    logger = match verbosity {
        Level::Warn => logger.level(log::LevelFilter::Warn),
        Level::Info => logger.level(log::LevelFilter::Info),
        Level::Debug => logger.level(log::LevelFilter::Debug),
        Level::Trace => logger.level(log::LevelFilter::Trace),
    };

    logger = logger.chain(std::io::stderr());

    logger.apply().unwrap();
}
