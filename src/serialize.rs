use std::error::Error;

pub trait Serialize<T> {
    type Error: Error;
    fn serialize(&self, event: &T) -> Result<Vec<u8>, Self::Error>;
}

// By default we can serialize any Event to JSON with serde
use event::{Event, LogstashEvent, Payload, MetaValue};
use serde::ser::SerializeMap;
use serde::Serializer;
pub use serde_json::error::Error as JsonError;
use serde_json::ser::Serializer as JsonSerializer;
use std::io::Cursor;
use std::io::Write;

#[derive(Default)]
pub struct JsonEventSerializer;

/*
fn serialize_map_meta_value<'a, S>(serializer: &'a mut S, state: &mut <&'a mut S as Serializer>::SerializeMap, meta_value: MetaValue) -> Result<(), <&'a mut S as Serializer>::Error> where &'a mut S: Serializer {
    match meta_value {
        MetaValue::String(v) => state.serialize_value(v)?,
        MetaValue::U64(v) => state.serialize_value(v)?,
        MetaValue::Object(iter) => {
            let mut obj = serializer.serialize_map(None)?;

            for (key, value) in iter {
                obj.serialize_key(key)?;
                serialize_map_meta_value(serializer, &mut obj, value)?;
            }

            obj.end()?;
        }
    }
    Ok(())
}
*/

impl JsonEventSerializer {
    pub fn write<T: Event, W: Write>(event: &T, out: W) -> Result<W, JsonError> {
        let mut serializer = JsonSerializer::new(out);
        {
            let mut state = serializer.serialize_map(None)?;

            state.serialize_key("id")?;
            state.serialize_value(event.id())?;

            state.serialize_key("source")?;
            state.serialize_value(event.source())?;

            state.serialize_key("timestamp")?;
            state.serialize_value(event.timestamp().to_rfc3339())?;

            if let Some(payload) = event.payload() {
                match payload {
                    Payload::String(s) => {
                        state.serialize_key("message")?;
                        state.serialize_value(s)?;
                    }
                    Payload::Data(s) => {
                        state.serialize_key("data")?;
                        state.serialize_value(s.as_ref().as_bytes())?;
                    }
                }
            }

            /*
            for (key, value) in event.meta() {
                state.serialize_key(key)?;
                serialize_map_meta_value(&mut serializer, &mut state, value)?;
            }
            */

            state.end()?;
        }
        Ok(serializer.into_inner())
    }
}

impl<T: Event> Serialize<T> for JsonEventSerializer {
    type Error = JsonError;

    fn serialize(&self, event: &T) -> Result<Vec<u8>, Self::Error> {
        let mut serializer = JsonSerializer::new(Cursor::new(Vec::new()));
        {
            let mut state = serializer.serialize_map(None)?;

            state.serialize_key("id")?;
            state.serialize_value(event.id())?;

            state.serialize_key("source")?;
            state.serialize_value(event.source())?;

            state.serialize_key("timestamp")?;
            state.serialize_value(event.timestamp().to_rfc3339())?;

            if let Some(payload) = event.payload() {
                match payload {
                    Payload::String(s) => {
                        state.serialize_key("message")?;
                        state.serialize_value(s)?;
                    }
                    Payload::Data(s) => {
                        state.serialize_key("data")?;
                        state.serialize_value(s.as_ref().as_bytes())?;
                    }
                }
            }

            /*
            for (key, value) in event.meta() {
                state.serialize_key(key)?;
                serialize_map_meta_value(&mut serializer, &mut state, value)?;
            }
            */

            state.end()?;
        }
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
        {
            let mut state = serializer.serialize_map(None)?;

            state.serialize_key("@timestamp")?;
            state.serialize_value(event.timestamp().to_rfc3339())?;

            state.serialize_key("@version")?;
            state.serialize_value(event.version())?;

            if let Some(message) = event.message() {
                state.serialize_key("message")?;
                state.serialize_value(message)?;
            }

            state.serialize_key("type")?;
            state.serialize_value(event.event_type())?;

            state.serialize_key("tags")?;
            state.serialize_value(event.tags())?;

            state.serialize_key("@processed")?;
            state.serialize_value(event.processed().to_rfc3339())?;

            state.serialize_key("@id")?;
            state.serialize_value(event.id())?;

            state.end()?;
        }
        Ok(serializer.into_inner().into_inner())
    }
}
