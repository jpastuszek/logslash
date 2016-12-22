use std::error::Error;

pub trait Serialize<T> {
    type Error: Error;
    fn serialize(&self, event: &T) -> Result<Vec<u8>, Self::Error>;
}

// By default we can serialize any Event to JSON with serde
use event::{Event, Payload};
use serde_json::error::Error as JsonError;
use serde_json::ser::Serializer as JsonSerializer;
use serde::Serializer;
use std::io::Cursor;

#[derive(Default)]
pub struct JsonEventSerializer;

impl<T: Event> Serialize<T> for JsonEventSerializer {
    type Error = JsonError;

    fn serialize(&self, event: &T) -> Result<Vec<u8>, Self::Error> {
        let mut serializer = JsonSerializer::new(Cursor::new(Vec::new()));
        let mut state = serializer.serialize_map(None)?;

        serializer.serialize_map_key(&mut state, "id")?;
        serializer.serialize_map_value(&mut state, event.id())?;

        serializer.serialize_map_key(&mut state, "source")?;
        serializer.serialize_map_value(&mut state, event.source())?;

        serializer.serialize_map_key(&mut state, "timestamp")?;
        serializer.serialize_map_value(&mut state, event.timestamp().to_rfc3339())?;

        if let Some(payload) = event.payload() {
            match payload {
                Payload::String(s) => {
                    serializer.serialize_map_key(&mut state, "message")?;
                    serializer.serialize_map_value(&mut state, s)?;
                }
                Payload::Data(s) => {
                    serializer.serialize_map_key(&mut state, "data")?;
                    serializer.serialize_map_value(&mut state, s.as_ref().as_bytes())?;
                }
            }
        }

        serializer.serialize_map_end(state)?;
        Ok(serializer.into_inner().into_inner())
    }
}

/*
pub trait Serialize<T, E> {
    type Output;
    type Error;
    //TODO: moving T by value will be slow
    // ideally input would be a stream of Write targets and output would be a stream of WriteAll
    // futures
    fn serialize<I: Stream<Item=T, Error=E>>(&self, events: I) -> Box<Stream<Item=(T, Self::Output), Error=Self::Error>>;
}

#[derive(Debug)]
pub enum SerializeError<SE: Error> {
    Io(IoError),
    Serialization(SE)
}

impl<SE: Error> From<IoError> for SerializeError<SE> {
    fn from(error: IoError) -> SerializeError<SE> {
        SerializeError::Io(error)
    }
}

impl<SE: Error> Display for SerializeError<SE> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SerializeError::Io(ref error) => write!(f, "{}: {}", self.description(), error),
            SerializeError::Serialization(ref error) => write!(f, "{}: {}", self.description(), error),
        }
    }
}

impl<SE: Error> Error for SerializeError<SE> {
    fn description(&self) -> &str {
        match *self {
            SerializeError::Io(_) => "I/O error while writing serialized event",
            SerializeError::Serialization(_) => "Failed to serialize event",
        }
    }
}

*/

/*
 use futures::Future;
 use std::error::Error;
 use std::fmt::{self, Display};
 use std::io::Error as IoError;
 use std::io::Write;

// Given Write output and refrence to event T it will return future that will not return untill all
// data has be serialized into output
pub trait Serialize<T, W: Write> {
    type SeError: Error;
    fn serialize(&self, output: W, event: &T) -> Box<Future<Item=(usize, W), Error=(SerializeError<Self::SeError>, W)>>;
}
*/

/*
struct JSONEventSerializer {};
impl Serialize<T, E> for  JSONEventSerializer {
    type Output = String;
    type Error = ;

    fn serialize<I: Stream<Item=T, Error=E>>(&self, events: I) -> Box<Stream<Item=(T, Self::Output), Error=Self::Error>> {
        Box::new(events.then(|event| {
            match event {
                Ok(event) => {
                    let data = Cursor::new(Vec::new());
                    let mut serializer = serde_json::ser::Serializer::new(data);

                    serializer.serialize_map_end(state)?;
                    let mut state = serializer.serialize_map(None)?;
                }
            }
        })
    }
}
*/
