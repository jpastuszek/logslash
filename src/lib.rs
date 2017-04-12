#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate assert_matches;
extern crate chrono;
extern crate maybe_string;
extern crate uuid;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod input;
pub mod output;
pub mod codec;
pub mod event;
pub mod serialize;

use tokio_core::reactor::Core;
use futures::sync::mpsc::SendError;
use std::fmt::{self, Display, Debug};
use std::error::Error;
use slog::{DrainExt, Logger};

#[derive(Debug)]
pub enum PipeError<IE, OE> {
    Input(IE),
    Output(OE)
}

impl<IE: Debug + Display, OE: Debug + Display> Display for PipeError<IE, OE> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PipeError::Input(ref error) => write!(f, "{}: {}", self.description(), error),
            PipeError::Output(ref error) => write!(f, "{}: {}", self.description(), error),
        }
    }
}

impl<IE: Debug + Display, OE: Debug + Display> Error for PipeError<IE, OE> {
    fn description(&self) -> &str {
        match *self {
            PipeError::Input(_) => "Pipe input failed",
            PipeError::Output(_) => "Pipe output failed",
        }
    }
}

impl<IE, OE, T> From<SendError<T>> for PipeError<IE, OE> {
    fn from(_send_error: SendError<T>) -> PipeError<IE, OE> {
        panic!("Output receiver died")
    }
}

pub fn event_loop() -> Core {
    Core::new().expect("Tokio Core event loop")
}

pub fn terminal_logger() -> Logger {
    let drain = slog_term::streamer().build().fuse();
    let root_logger = slog::Logger::root(drain, o!());
    info!(root_logger, "Logging started");
    root_logger
}

pub fn null_logger() -> Logger {
    let drain = slog::Discard;
    let root_logger = slog::Logger::root(drain, o!());
    info!(root_logger, "Logging started");
    root_logger
}
