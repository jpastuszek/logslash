use std::borrow::Cow;
use chrono::{DateTime, UTC};
use serde::ser::Serializer;

/* Logstash Event
 * * serializes as some kind of hash map with fields:
 * ** @timestamp (iso8601 e.g. 2013-02-09T20:39:26.234Z)
 * ** @version (1)
 * ** message (String)
 * ** type (String)
 * ** tags ([String])
 * * extra custom fields
 * ** @processed (iso8601)
 * ** @id (String)
 */

pub trait FieldSerializer {
    type Error;
    fn serialize_field_str(&mut self, name: &str, value: &str) -> Result<(), Self::Error>;
    fn serialize_field_u64(&mut self, name: &str, value: u64) -> Result<(), Self::Error>;
    fn serialize_field_tags(&mut self, name: &str, tags: &[&'static str]) -> Result<(), Self::Error>;
}

struct SerdeFieldSerializer<'ss, 's: 'ss, S> where S: Serializer + 's {
    serializer: &'s mut S,
    state: &'ss mut S::MapState,
}

impl<'s, 'ss, S> FieldSerializer for SerdeFieldSerializer<'s, 'ss, S> where S: Serializer + 's {
    type Error = S::Error;

    fn serialize_field_str(&mut self, name: &str, value: &str) -> Result<(), Self::Error> {
        self.serializer.serialize_map_key(&mut self.state, name)?;
        self.serializer.serialize_map_value(&mut self.state, value)
    }

    fn serialize_field_u64(&mut self, name: &str, value: u64) -> Result<(), Self::Error> {
        self.serializer.serialize_map_key(&mut self.state, name)?;
        self.serializer.serialize_map_value(&mut self.state, value)
    }

    fn serialize_field_tags(&mut self, name: &str, tags: &[&'static str]) -> Result<(), Self::Error> {
        self.serializer.serialize_map_key(&mut self.state, name)?;
        self.serializer.serialize_map_value(&mut self.state, tags)
    }
}

pub trait LogstashEvent {
    fn timestamp(&self) -> DateTime<UTC>;
    fn version(&self) -> &str { "1" }
    fn message(&self) -> Cow<str>;
    fn event_type(&self) -> &str;
    fn tags(&self) -> Vec<&'static str>;
    fn processed(&self) -> DateTime<UTC>;
    fn id(&self) -> Cow<str>;
    fn fields<F: FieldSerializer>(&self, serializer: &mut F) -> Result<(), F::Error>;
}

pub trait SerializeEvent {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer;
}

impl<T> SerializeEvent for T where T: LogstashEvent {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        let mut state = serializer.serialize_map(None)?;
        {
            let mut filed_serializer = SerdeFieldSerializer { serializer: serializer, state: &mut state };

            filed_serializer.serialize_field_str("@timestamp", &self.timestamp().to_rfc3339())?; //TODO: TZ should be 'Z'
            filed_serializer.serialize_field_str("@processed", &self.processed().to_rfc3339())?; //TODO: TZ should be 'Z'
            filed_serializer.serialize_field_str("@version", self.version())?;
            filed_serializer.serialize_field_str("@id", self.id().as_ref())?;
            filed_serializer.serialize_field_str("type", self.event_type())?;
            filed_serializer.serialize_field_tags("tags", self.tags().as_slice())?;
            filed_serializer.serialize_field_str("message", self.message().as_ref())?;

            self.fields(&mut filed_serializer)?;
        }
        serializer.serialize_map_end(state)
    }
}
