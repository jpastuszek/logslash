use std::fmt::{self, Display, Debug};
use std::error::Error;
use std::borrow::Cow;
use std::io::Write;
use std::fs::File;
use std::io::stdout;

use slog::Logger;

use futures::Sink;
use chrono::{DateTime, UTC};
use PipeError;
use serialize::Serializer;
use output::write::write_threaded;

pub trait DebugPort {
    type Payload;

    fn id(&self) -> Cow<str>;
    fn timestamp(&self) -> DateTime<UTC>;
    fn source(&self) -> Cow<str>;
    fn write_payload<W: Write, S: Serializer<Self::Payload>>(&self, out: W, serializer: &S) -> Result<W, S::Error>;
}

#[derive(Debug)]
pub enum DebugOuputError<SE> {
    Serialization(SE),
}

impl<SE: Debug + Display> Display for DebugOuputError<SE> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DebugOuputError::Serialization(ref error) => write!(f, "{}: {}", self.description(), error),
        }
    }
}

impl<SE: Debug + Display> Error for DebugOuputError<SE> {
    fn description(&self) -> &str {
        match *self {
            DebugOuputError::Serialization(_) => "Failed to serialize debug event",
        }
    }
}

fn serialize_event<T, S>(event: &T, buf: &mut Vec<u8>, serializer: &S) -> Result<(), DebugOuputError<<S as Serializer<<T as DebugPort>::Payload>>::Error>> where T: DebugPort + 'static, S: Serializer<T::Payload> + 'static {
    write!(buf, "{} {} [{}] -- ",  event.id().as_ref(), event.source().as_ref(), event.timestamp()).expect("header written to buf");

    event.write_payload(buf, serializer)
        .map_err(|error| DebugOuputError::Serialization(error))
        .map(|mut buf| buf.push(b'\n'))
        .map(|_| ())
}

pub fn debug_to_file<T, S, IE>(logger: &Logger, file: File, serializer: S) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + Send + 'static, S: Serializer<T::Payload> + Send + 'static, IE: 'static {
    write_threaded(logger, "debug_to_file", file, move |event: &T, buf: &mut Vec<u8>| {
        serialize_event(event, buf, &serializer)
    })
}

pub fn debug_print<T, S, IE>(logger: &Logger, serializer: S) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + Send + 'static, S: Serializer<T::Payload> + Send + 'static, IE: 'static {
    write_threaded(logger, "debug_print", stdout(), move |event: &T, buf: &mut Vec<u8>| {
        serialize_event(event, buf, &serializer)
    })
}
