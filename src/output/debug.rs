use std::fmt::{self, Display, Debug};
use std::error::Error;
use std::borrow::Cow;
use std::io::Write;
use std::fs::File;
use std::io::stdout;
use futures::Sink;
use tokio_core::reactor::Handle;
use chrono::{DateTime, UTC};
use PipeError;
use serialize::Serializer;
use output::write::write;

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

fn write_event<T, S>(event: &T, buf: &mut Vec<u8>, serializer: &S) -> Result<(), DebugOuputError<<S as Serializer<<T as DebugPort>::Payload>>::Error>> where T: DebugPort + 'static, S: Serializer<T::Payload> + 'static {
    write!(buf, "{} {} [{}] -- ",  event.id().as_ref(), event.source().as_ref(), event.timestamp()).expect("header written to buf");

    event.write_payload(buf, serializer)
        .map_err(|error| DebugOuputError::Serialization(error))
        .map(|mut buf| buf.push(b'\n'))
        .map(|_| ())
}

pub fn debug_to_file<T, S, IE>(handle: Handle, file: File, serializer: S) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + 'static, S: Serializer<T::Payload> + 'static, IE: 'static {
    write(handle, file, move |event: &T, buf: &mut Vec<u8>| {
        write_event(event, buf, &serializer)
    })
}

pub fn debug_print<T, S, IE>(handle: Handle, serializer: S) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + 'static, S: Serializer<T::Payload> + 'static, IE: 'static {
    write(handle, stdout(), move |event: &T, buf: &mut Vec<u8>| {
        write_event(event, buf, &serializer)
    })
}

#[cfg(unix)]
pub mod unix {
    use super::*;
    use std::os::unix::io::FromRawFd;
    use output::write::unix::write_evented;

    pub fn debug_to_file<T, S, IE>(handle: Handle, file: File, serializer: S) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + Debug + 'static, S: Serializer<T::Payload> + 'static, IE: 'static {
        write_evented(handle, file, move |event: &T, buf: &mut Vec<u8>| {
            write_event(event, buf, &serializer)
        })
    }

    pub fn debug_print<T, S, IE>(handle: Handle, serializer: S) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + Debug + 'static, S: Serializer<T::Payload> + 'static, IE: 'static {
        unsafe {
            let stdout = File::from_raw_fd(1);
            debug_to_file(handle, stdout, serializer)
        }
    }
}
