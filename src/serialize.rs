use std::error::Error;

pub trait Serialize<T> {
    type Error: Error;
    fn serialize(&self, event: &T) -> Result<Vec<u8>, Self::Error>;
}

// By default we can serialize any Event to JSON with serde
use event::{Event, LogstashEvent, Payload, MetaValue};
use serde_json::error::Error as JsonError;
use serde_json::ser::Serializer as JsonSerializer;
use serde::Serializer;
use std::io::Cursor;

#[derive(Default)]
pub struct JsonEventSerializer;

fn serialize_map_meta_value<S>(serializer: &mut S, state: &mut S::MapState, meta_value: MetaValue) -> Result<(), <S as Serializer>::Error> where S: Serializer {
    match meta_value {
        MetaValue::String(v) => serializer.serialize_map_value(state, v)?,
        MetaValue::U64(v) => serializer.serialize_map_value(state, v)?,
        MetaValue::Object(iter) => {
            let mut obj = serializer.serialize_map(None)?;

            for (key, value) in iter {
                serializer.serialize_map_key(&mut obj, key)?;
                serialize_map_meta_value(serializer, &mut obj, value)?;
            }

            serializer.serialize_map_end(obj)?;
        }
    }
    Ok(())
}

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

        for (key, value) in event.meta() {
            serializer.serialize_map_key(&mut state, key)?;
            serialize_map_meta_value(&mut serializer, &mut state, value)?;
        }

        serializer.serialize_map_end(state)?;
        Ok(serializer.into_inner().into_inner())
    }
}

#[derive(Default)]
pub struct JsonLogstashEventSerializer;

impl<T: LogstashEvent> Serialize<T> for JsonLogstashEventSerializer {
    type Error = JsonError;
    // type Mapper

    fn serialize(&self, event: &T) -> Result<Vec<u8>, Self::Error> {
        let mut serializer = JsonSerializer::new(Cursor::new(Vec::new()));
        let mut state = serializer.serialize_map(None)?;

        serializer.serialize_map_key(&mut state, "@timestamp")?;
        serializer.serialize_map_value(&mut state, event.timestamp().to_rfc3339())?;

        serializer.serialize_map_key(&mut state, "@version")?;
        serializer.serialize_map_value(&mut state, event.version())?;

        if let Some(message) = event.message() {
            serializer.serialize_map_key(&mut state, "message")?;
            serializer.serialize_map_value(&mut state, message)?;
        }

        serializer.serialize_map_key(&mut state, "type")?;
        serializer.serialize_map_value(&mut state, event.event_type())?;

        serializer.serialize_map_key(&mut state, "tags")?;
        serializer.serialize_map_value(&mut state, event.tags())?;

        serializer.serialize_map_key(&mut state, "@processed")?;
        serializer.serialize_map_value(&mut state, event.processed().to_rfc3339())?;

        serializer.serialize_map_key(&mut state, "@id")?;
        serializer.serialize_map_value(&mut state, event.id())?;

        serializer.serialize_map_end(state)?;
        Ok(serializer.into_inner().into_inner())
    }
}
