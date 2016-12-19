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
use std::borrow::Cow;
use maybe_string::MaybeString;
use futures::Stream;
use futures::stream::Then;
use event::{Payload, Event};
use serde::Serializer;
use serde_json;

pub trait DebugPort: Event {
}

#[derive(Debug)]
pub enum DebugOuputError {
    SerdeJson(serde_json::Error),
    InputClosed
}

impl From<serde_json::Error> for DebugOuputError {
    fn from(error: serde_json::Error) -> DebugOuputError {
        DebugOuputError::SerdeJson(error)
    }
}

impl Display for DebugOuputError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DebugOuputError::SerdeJson(ref error) => write!(f, "{}: {}", self.description(), error),
            DebugOuputError::InputClosed => write!(f, "{}", self.description()),
        }
    }
}

impl Error for DebugOuputError {
    fn description(&self) -> &str {
        match *self {
            DebugOuputError::SerdeJson(_) => "Failed to serialise event into JSON",
            DebugOuputError::InputClosed => "Input closed",
        }
    }
}

//TODO: take Serializer type to do stuff to fileds before serialized
pub fn print_serde_json<F, T: 'static>(source: F) ->
    Then<F, fn(Result<T, ()>) -> Result<T, DebugOuputError>, Result<T, DebugOuputError>>
    where F: Stream<Item=T, Error=()>, T: DebugPort
{
    fn serialize_and_print<T>(event: Result<T, ()>) -> Result<T, DebugOuputError> where T: DebugPort {
        match event {
            Ok(event) => {
                let data = Cursor::new(Vec::new());
                let mut serializer = serde_json::ser::Serializer::new(data);

                let mut state = serializer.serialize_map(None)?;
                /*
                for (ref name, ref value) in event.fields() {
                    serializer.serialize_map_key(&mut state, *name)?;
                    match *value {
                        FieldValue::String(ref value) => serializer.serialize_map_value(&mut state, value)?,
                        FieldValue::U64(value) => serializer.serialize_map_value(&mut state, value)?,
                    }
                }
                */
                serializer.serialize_map_end(state)?;

                println!("{} - {}: {}", event.timestamp(), event.payload().unwrap_or(Payload::String(Cow::Borrowed("<no message>"))), MaybeString(serializer.into_inner().into_inner()));
                Ok(event)
            }
            Err(()) => Err(DebugOuputError::InputClosed)
        }
    }

    source.then(serialize_and_print)
}
