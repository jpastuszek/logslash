use std::fmt::{self, Display, Debug};
use std::error::Error;
use std::borrow::Cow;
use std::io::stdout;
use std::io::Write;
use futures::Sink;
use tokio_core::reactor::Handle;
use chrono::{DateTime, UTC};
use PipeError;
use output::write::write;

pub trait DebugPort {
    type SerializeError: Error;
    fn id(&self) -> Cow<str>;
    fn timestamp(&self) -> DateTime<UTC>;
    fn source(&self) -> Cow<str>;
    fn write_payload<W: Write>(&self, out: W) -> Result<W, Self::SerializeError>;
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

pub fn debug_print<T, IE>(handle: Handle) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + Debug + 'static, IE: 'static {
    write(handle, stdout(), |event: &T, buf: &mut Vec<u8>| {
        write!(buf, "{} {} [{}] -- ",  event.id().as_ref(), event.source().as_ref(), event.timestamp()).expect("header written to buf");

        event.write_payload(buf)
            .map_err(|error| DebugOuputError::Serialization(error))
            .map(|mut buf| buf.push(b'\n'))
            .map(|_| ())
    })
}
