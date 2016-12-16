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

use std::fmt::Debug;
use std::error::Error;
use std::io::Cursor;
use std::borrow::Cow;
use maybe_string::MaybeString;
use futures::Stream;
use futures::stream::Then;
use event::Event;
use serde::Serializer;
use serde_json;
use chrono::{DateTime, UTC};

pub trait DebugPort {
}

#[derive(Debug)]
pub enum DebugOuputError<E> {
    SerdeJsonError(serde_json::Error),
    InputError(E)
}

//TODO: impl Error

impl<E> From<serde_json::Error> for DebugOuputError<E> {
    fn from(error: serde_json::Error) -> DebugOuputError<E> {
        DebugOuputError::SerdeJsonError(error)
    }
}

//TODO: take Serializer type to do stuff to fileds before serialized
pub fn print_serde_json<T: 'static, F: 'static, E: 'static>(source: F) ->
    Then<F, fn(Result<T, E>) -> Result<T, DebugOuputError<E>>, Result<T, DebugOuputError<E>>>
    where T: Event, F: Stream<Item=T, Error=E>, T: DebugPort
{
    fn serialize_and_print<T: 'static, E: 'static>(event: Result<T, E>) -> Result<T, DebugOuputError<E>> where T: Event + DebugPort {
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

                println!("{} - {}: {}", event.timestamp(), event.message().unwrap_or(Cow::Borrowed("<no message>")), MaybeString(serializer.into_inner().into_inner()));
                Ok(event)
            }
            Err(error) => Err(DebugOuputError::InputError(error))
        }
    }

    source.then(serialize_and_print)
}
