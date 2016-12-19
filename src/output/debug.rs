/*
use futures::sync::mpsc;
use futures::Stream;
use std::fmt::Debug;

pub fn print_debug<T: Debug + 'static>(source: mpsc::Receiver<T>) -> Box<Future<Item=(),Error=()>> {
    Box::new(source.for_each(|message| {
        println!("{:#?}", &message);
        Ok(())
    }))
}

use std::io::Cursor;
use maybe_string::MaybeString;
use event::{LogstashEvent, SerializeEvent, SerdeFieldSerializer, FieldSerializer};
use serde_json::ser::Serializer;

pub fn print_logstash<T: LogstashEvent + 'static>(source: mpsc::Receiver<T>) -> Box<Future<Item=(),Error=()>> {
    Box::new(source.for_each(|message| {
        let data = Cursor::new(Vec::new());
        let mut ser = Serializer::new(data);

        {
            let field_ser = SerdeFieldSerializer::new(&mut ser).expect("field serializer")
                .rename("severity", "log_level")
                .map_str("severity", |l| l.to_lowercase())
                .map_str("message", |m| m.replace("#012", "\n"));
            message.serialize(field_ser).expect("serialized message");
        }

        let json = ser.into_inner().into_inner();
        // TODO error handling
        println!("{}", MaybeString(json));

        Ok(())
    }))
}
*/

use std::fmt::{self, Display};
use std::error::Error;
use std::io::Cursor;
use std::io::{self, Write};
use std::borrow::Cow;
use chrono::{DateTime, UTC};
use maybe_string::MaybeString;
use futures::{Future, Stream};
use event::{Payload, Event};
use serialize::Serialize;
use serde::Serializer;
use serde_json;
pub trait DebugPort {
    fn id(&self) -> Cow<str>;
    fn timestamp(&self) -> DateTime<UTC>;
    fn source(&self) -> Cow<str>;
}

/*
#[derive(Debug)]
pub enum DebugOuputError<SE: Error> {
    //TODO: chain errors?
    Serialization(SE),
    InputClosed
}

impl From<serde_json::Error> for DebugOuputError {
    fn from(error: serde_json::Error) -> DebugOuputError {
        DebugOuputError::Serialization(error)
    }
}

impl<SE: Error> Display for DebugOuputError<SE> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DebugOuputError::Serialization(ref error) => write!(f, "{}: {}", self.description(), error),
            DebugOuputError::InputClosed => write!(f, "{}", self.description()),
        }
    }
}

impl<SE: Error> Error for DebugOuputError<SE> {
    fn description(&self) -> &str {
        match *self {
            DebugOuputError::Serialization(_) => "Failed to serialise event",
            DebugOuputError::InputClosed => "Input closed",
        }
    }
}
*/

pub fn print_serde_json<F, S, SE: 'static, T: 'static>(source: F, serializer: S) -> Box<Stream<Item=T, Error=S::Error>>
    where F: Stream<Item=T, Error=()>, T: DebugPort, S: Serialize<T, (), Output=String, Error=SE>
{
    fn print<T, SE>(event: Result<T, ()>) -> Box<Future<Item=W, Error=Self::Error>> where T: DebugPort {
        match event {
            Ok(event) => {
                //TODO: lock once if possible?!? - this may block the event loop no?
                let stdout = io::stdout();
                let mut handle = stdout.lock();

                write!(&mut handle, "{} {}[{}] -- ",  event.id().as_ref(), event.timestamp(), event.source().as_ref());
                serializer.serialize(handle, &event)
            }
            Err(err) => Err(err)
        }
    }

    source.then(print)
}
