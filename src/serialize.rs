use std::error::Error;

// By default we can serialize any Event to JSON with serde
use event::{Event, LogstashEvent, Payload, MetaValue};
use serde::ser::{Serialize, SerializeMap};
use serde::Serializer as SerdeSerializer;
use serde_json::error::Error as JsonError;
use serde_json::ser::Serializer as JsonSerializer;
use std::io::Write;
use std::cell::RefCell;

pub trait Serializer<T> {
    type Error: Error;
    fn serialize<W: Write>(event: &T, out: W) -> Result<W, Self::Error>;
}

// Note:
// Need RefCell as this is using iterators inside that we need to mutate to iterate
// This means that it can be serialized only once (need to call Event::meta() to get fresh iterator
// every time it is serialized
struct MetaValueSerde<'i>(RefCell<MetaValue<'i>>);

impl<'i> Serialize for MetaValueSerde<'i> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: SerdeSerializer {
        match *self.0.borrow_mut() {
            MetaValue::String(string) => serializer.serialize_str(string),
            MetaValue::U64(num) => serializer.serialize_u64(num),
            MetaValue::Object(ref mut iter) => {
                let mut map = serializer.serialize_map(None)?;
                for (key, value) in iter {
                    map.serialize_key(key)?;
                    map.serialize_value(MetaValueSerde(RefCell::new(value)))?;
                }
                map.end()
            }
        }
    }
}

#[derive(Default)]
pub struct JsonEventSerializer;

/* impossible due to lack of access to serializer when buidling a map
fn serialize_meta_value_iter(iter: Box<Iterator<Item=(&'i str, MetaValue<'i>)> + 'i>, serializer: &'a mut S) -> Result<(), <&'a mut S as Serializer>::Error> where &'a mut S: Serializer {
    let mut map = serializer.serialize_map(None)?;

    for (key, value) in iter {
        map.serialize_key(key)?;
        match value {
            MetaValue::String(string) => map.serialize_value(string),
            MetaValue::U64(num) => map.serialize_value(num),
            MetaValue::Object(iter) => {
                serialize_meta_value_iter(iter, ?serializer?)?
            }
        }
    }

    map.end()
}
*/

impl<T: Event> Serializer<T> for JsonEventSerializer {
    type Error = JsonError;

    fn serialize<W: Write>(event: &T, out: W) -> Result<W, JsonError> {
        let mut serializer = JsonSerializer::new(out);
        {
            let mut map = serializer.serialize_map(None)?;

            map.serialize_key("id")?;
            map.serialize_value(event.id())?;

            map.serialize_key("source")?;
            map.serialize_value(event.source())?;

            map.serialize_key("timestamp")?;
            map.serialize_value(event.timestamp().to_rfc3339())?;

            if let Some(payload) = event.payload() {
                match payload {
                    Payload::String(s) => {
                        map.serialize_key("message")?;
                        map.serialize_value(s)?;
                    }
                    Payload::Data(s) => {
                        map.serialize_key("data")?;
                        map.serialize_value(s.as_ref().as_bytes())?;
                    }
                }
            }

            for (key, value) in event.meta() {
                map.serialize_key(key)?;
                map.serialize_value(MetaValueSerde(RefCell::new(value)))?;
            }

            map.end()?;
        }
        Ok(serializer.into_inner()) }
}

#[derive(Default)]
pub struct JsonLogstashEventSerializer;

impl<T: LogstashEvent> Serializer<T> for JsonLogstashEventSerializer {
    type Error = JsonError;

    fn serialize<W: Write>(event: &T, out: W) -> Result<W, JsonError> {
        let mut serializer = JsonSerializer::new(out);
        {
            let mut map = serializer.serialize_map(None)?;

            map.serialize_key("@timestamp")?;
            map.serialize_value(event.timestamp().to_rfc3339())?;

            map.serialize_key("@version")?;
            map.serialize_value(event.version())?;

            if let Some(message) = event.message() {
                map.serialize_key("message")?;
                map.serialize_value(message)?;
            }

            map.serialize_key("type")?;
            map.serialize_value(event.event_type())?;

            map.serialize_key("tags")?;
            map.serialize_value(event.tags())?;

            map.serialize_key("@processed")?;
            map.serialize_value(event.processed().to_rfc3339())?;

            map.serialize_key("@id")?;
            map.serialize_value(event.id())?;

            for (key, value) in event.fields() {
                map.serialize_key(key)?;
                map.serialize_value(MetaValueSerde(RefCell::new(value)))?;
            }

            map.end()?;
        }
        Ok(serializer.into_inner())
    }
}
