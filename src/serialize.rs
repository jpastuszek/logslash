use std::error::Error;

pub trait Serialize<T> {
    type Error: Error;
    fn serialize(&self, event: &T) -> Result<Vec<u8>, Self::Error>;
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
