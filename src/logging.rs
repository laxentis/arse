#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

enum Destination {
    Stdout,
    File(Mutex<File>),
}

struct Logger {
    minimum_level: Level,
    destination: Destination,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_cli() {
    let _ = LOGGER.set(Logger {
        minimum_level: Level::Trace,
        destination: Destination::Stdout,
    });
}

pub fn init_gui() -> io::Result<()> {
    let path = std::env::var_os("ARSE_LOG_FILE").unwrap_or_else(|| "arse.log".into());
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let minimum_level = match std::env::var("ARSE_LOG_LEVEL").as_deref() {
        Ok("trace") => Level::Trace,
        Ok("info") => Level::Info,
        Ok("warn") | Ok("warning") | Err(_) => Level::Warn,
        Ok("error") => Level::Error,
        Ok(_) => Level::Warn,
    };
    let _ = LOGGER.set(Logger {
        minimum_level,
        destination: Destination::File(Mutex::new(file)),
    });
    Ok(())
}

pub fn trace(message: impl std::fmt::Display) {
    log(Level::Trace, message);
}

pub fn info(message: impl std::fmt::Display) {
    log(Level::Info, message);
}

pub fn warn(message: impl std::fmt::Display) {
    log(Level::Warn, message);
}

pub fn error(message: impl std::fmt::Display) {
    log(Level::Error, message);
}

fn log(level: Level, message: impl std::fmt::Display) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    if level < logger.minimum_level {
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let line = format!("[{timestamp}] [{}] {message}", level.as_str());
    match &logger.destination {
        Destination::Stdout => println!("{line}"),
        Destination::File(file) => {
            if let Ok(mut file) = file.lock() {
                let _ = writeln!(file, "{line}");
            }
        }
    }
}
