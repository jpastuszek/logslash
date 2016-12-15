/*
use futures::sync::mpsc;
use futures::future::Future;
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

use std::io::Cursor;
use maybe_string::MaybeString;
use futures::future::Future;
use event::{SerdeJsonEvent, FieldValue};
use serde::Serializer;
use serde_json;

//TODO: take Serializer type to do stuff to fileds before serialized
pub fn print_serde_json<'f, T: 'f + 'static, F: 'static, E>(source: F) -> Box<Future<Item=T, Error=serde_json::Error>> where T: SerdeJsonEvent<'f>, F: Future<Item=T, Error=serde_json::Error> {
    fn serialize_and_print<'f, T: 'f + 'static>(event: T) -> Result<T, serde_json::Error> where T: SerdeJsonEvent<'f> {
        let data = Cursor::new(Vec::new());
        let mut serializer = serde_json::ser::Serializer::new(data);

        let mut state = serializer.serialize_map(None)?;
        for (ref name, ref value) in event.fields() {
            serializer.serialize_map_key(&mut state, *name)?;
            match *value {
                FieldValue::String(ref value) => serializer.serialize_map_value(&mut state, value)?,
                FieldValue::U64(value) => serializer.serialize_map_value(&mut state, value)?,
            }
        }
        serializer.serialize_map_end(state)?;

        println!("{}", MaybeString(serializer.into_inner().into_inner()));
        Ok(event)
    }

    Box::new(source.and_then(serialize_and_print))
}
