//! Standard Rust log crate adapter to slog-rs
//!
//! Note: `slog-stdlog` for slog v2, unlike previous releases uses logging
//! scopes provided by `slog-scope` crate instead of providing it's own ones.
//!
//! This crate provides two way compatibility with legacy `log` crate logging.
//!
//! ### `log` -> `slog`
//!
//! After calling `init` legacy `log` crate logging statements (eg.
//! `debug!(...)`) will be redirected just like they originated from the logger
//! returned by `slog_scope::logger()`.  See documentation of `slog-scope` for
//! examples of logging scope usage.
//!
//! ### `slog` -> `log`
//!
//! `StdLog` is a `slog::Drain` implementation that will log logging `Record`s
//! just like they were created using legacy `log` statements.
//!
//! ### Warning
//!
//! Be careful when using both methods at the same time, as a loop can be easily
//! created: `log` -> `slog` -> `log` -> ...
//!
//! ## Compile-time log level filtering
//!
//! For filtering `debug!` and other `log` statements at compile-time, configure
//! the features on the `log` crate in your `Cargo.toml`:
//!
//! ```norust
//! log = { version = "*", features = ["max_level_trace", "release_max_level_warn"] }
//! ```
#![warn(missing_docs)]

#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_scope;
extern crate log;

use log::LogMetadata;
use std::{io, fmt};

use slog::Level;
use slog::KV;

struct Logger;

fn log_to_slog_level(level: log::LogLevel) -> Level {
    match level {
        log::LogLevel::Trace => Level::Trace,
        log::LogLevel::Debug => Level::Debug,
        log::LogLevel::Info => Level::Info,
        log::LogLevel::Warn => Level::Warning,
        log::LogLevel::Error => Level::Error,
    }
}

impl log::Log for Logger {
    fn enabled(&self, _: &LogMetadata) -> bool {
        true
    }

    fn log(&self, r: &log::LogRecord) {
        let level = log_to_slog_level(r.metadata().level());

        let args = r.args();
        let target = r.target();
        let module = r.location().__module_path;
        let file = r.location().__file;
        let line = r.location().line();

        let s = slog::RecordStatic {
            location: &slog::RecordLocation {
                file: file,
                line: line,
                column: 0,
                function: "",
                module: module,
            },
            level: level,
            tag: target,
        };
        slog_scope::logger().log(&slog::Record::new(&s, args, b!()))
    }
}

/// Minimal initialization with default drain
///
/// The exact default drain is unspecified and will
/// change in future versions! Use `set_logger` instead
/// to build customized drain.
///
/// ```
/// #[macro_use]
/// extern crate log;
/// extern crate slog_stdlog;
///
/// fn main() {
///     slog_stdlog::init().unwrap();
///     // Note: this `info!(...)` macro comes from `log` crate
///     info!("standard logging redirected to slog");
/// }
/// ```
pub fn init() -> Result<(), log::SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(log::LogLevelFilter::max());
        Box::new(Logger)
    })
}

/// Drain logging `Record`s into `log` crate
///
/// Using `StdLog` is effectively the same as using `log::info!(...)` and
/// other standard logging statements.
///
/// Caution needs to be taken to prevent circular loop where `Logger`
/// installed via `slog-stdlog::set_logger` would log things to a `StdLog`
/// drain, which would again log things to the global `Logger` and so on
/// leading to an infinite recursion.
pub struct StdLog;

struct LazyLogString<'a> {
    info: &'a slog::Record<'a>,
    logger_values: &'a slog::OwnedKVList,
}

impl<'a> LazyLogString<'a> {
    fn new(info: &'a slog::Record, logger_values: &'a slog::OwnedKVList) -> Self {

        LazyLogString {
            info: info,
            logger_values: logger_values,
        }
    }
}

impl<'a> fmt::Display for LazyLogString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        try!(write!(f, "{}", self.info.msg()));

        let io = io::Cursor::new(Vec::new());
        let mut ser = KSV::new(io);

        let res = {
                || -> io::Result<()> {
                    try!(self.logger_values.serialize(self.info, &mut ser));
                    try!(self.info.kv().serialize(self.info, &mut ser));
                    Ok(())
                }
            }()
            .map_err(|_| fmt::Error);

        try!(res);

        let values = ser.into_inner().into_inner();


        write!(f, "{}", String::from_utf8_lossy(&values))

    }
}

impl slog::Drain for StdLog {
    type Err = io::Error;
    type Ok = ();
    fn log(&self, info: &slog::Record, logger_values: &slog::OwnedKVList) -> io::Result<()> {

        let level = match info.level() {
            slog::Level::Critical | slog::Level::Error => log::LogLevel::Error,
            slog::Level::Warning => log::LogLevel::Warn,
            slog::Level::Info => log::LogLevel::Info,
            slog::Level::Debug => log::LogLevel::Debug,
            slog::Level::Trace => log::LogLevel::Trace,
        };

        let target = info.tag();

        let location = log::LogLocation {
            __module_path: info.module(),
            __file: info.file(),
            __line: info.line(),
        };

        let lazy = LazyLogString::new(info, logger_values);
        // Please don't yell at me for this! :D
        // https://github.com/rust-lang-nursery/log/issues/95
        log::__log(level, target, &location, format_args!("{}", lazy));

        Ok(())
    }
}

/// Key-Separator-Value serializer
struct KSV<W: io::Write> {
    io: W,
}

impl<W: io::Write> KSV<W> {
    fn new(io: W) -> Self {
        KSV {
            io: io,
        }
    }

    fn into_inner(self) -> W {
        self.io
    }
}

impl<W: io::Write> slog::Serializer for KSV<W> {
    fn emit_arguments(&mut self, key: &str, val: &fmt::Arguments) -> slog::Result {
        try!(write!(self.io, ", {}: {}", key, val));
        Ok(())
    }
}
