use std::borrow::Cow;
use chrono::{DateTime, UTC};
use serde::ser::Serializer;

//pub enum FieldValue<'f> {
    //String(&'f str),
    //U64(u64),
//}

pub trait Event {
    //type FieldsIter: Iterator<Item=(&'static str, FieldValue<'f>)>;

    fn id(&self) -> Cow<str>;
    fn source(&self) -> Cow<str>;
    fn timestamp(&self) -> DateTime<UTC>;
    fn message(&self) -> Option<Cow<str>>;

    //TODO: id, source, message
    //fn fields(&self) -> Self::FieldsIter;
}

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

pub trait LogstashEvent {
    fn timestamp(&self) -> DateTime<UTC>;
    fn version(&self) -> &str { "1" }
    fn message(&self) -> Option<Cow<str>>;
    fn event_type(&self) -> &str;
    fn tags(&self) -> Vec<&'static str>;
    fn processed(&self) -> DateTime<UTC>;
    fn id(&self) -> Cow<str>;
    fn fields<F: FieldSerializer>(&self, serializer: &mut F) -> Result<(), F::Error>;
}

pub trait FieldSerializer {
    type Error;
    fn serialize_field_str(&mut self, name: &str, value: &str) -> Result<(), Self::Error>;
    fn serialize_field_u64(&mut self, name: &str, value: u64) -> Result<(), Self::Error>;
    fn serialize_field_tags(&mut self, name: &str, tags: &[&'static str]) -> Result<(), Self::Error>;
    fn finish(self) -> Result<(), Self::Error>;

    fn rename(self, from: &'static str, to: &'static str) -> RenamingFieldSerializer<Self> where Self: Sized {
        RenamingFieldSerializer {
            from: from,
            to: to,
            inner: self
        }
    }

    fn map_str<FU>(self, field: &'static str, f: FU) -> MapStrFieldSerializer<Self, FU> where Self: Sized, FU: FnMut(&str) -> String {
        MapStrFieldSerializer {
            field: field,
            f: f,
            inner: self
        }
    }
}

pub struct RenamingFieldSerializer<F: FieldSerializer> {
    from: &'static str,
    to: &'static str,
    inner: F
}

impl<F: FieldSerializer> FieldSerializer for RenamingFieldSerializer<F> {
    type Error = F::Error;

    fn serialize_field_str(&mut self, name: &str, value: &str) -> Result<(), Self::Error> {
       let name = if name == self.from {
           self.to
       } else {
           name
       };
       self.inner.serialize_field_str(name, value)
    }

    fn serialize_field_u64(&mut self, name: &str, value: u64) -> Result<(), Self::Error> {
        let name = if name == self.from {
            self.to
        } else {
            name
        };
        self.inner.serialize_field_u64(name, value)
    }

    fn serialize_field_tags(&mut self, name: &str, tags: &[&'static str]) -> Result<(), Self::Error> {
        let name = if name == self.from {
            self.to
        } else {
            name
        };
        self.inner.serialize_field_tags(name, tags)
    }

    fn finish(self) -> Result<(), Self::Error> {
        self.inner.finish()
    }
}

pub struct MapStrFieldSerializer<F: FieldSerializer, FU> {
    field: &'static str,
    f: FU,
    inner: F
}

impl<F: FieldSerializer, FU> FieldSerializer for MapStrFieldSerializer<F, FU> where FU: FnMut(&str) -> String {
    type Error = F::Error;

    fn serialize_field_str(&mut self, name: &str, value: &str) -> Result<(), Self::Error> {
       let value = if name == self.field {
           Cow::Owned((self.f)(value))
       } else {
           Cow::Borrowed(value)
       };
       self.inner.serialize_field_str(name, value.as_ref())
    }

    fn serialize_field_u64(&mut self, name: &str, value: u64) -> Result<(), Self::Error> {
        self.inner.serialize_field_u64(name, value)
    }

    fn serialize_field_tags(&mut self, name: &str, tags: &[&'static str]) -> Result<(), Self::Error> {
        self.inner.serialize_field_tags(name, tags)
    }

    fn finish(self) -> Result<(), Self::Error> {
        self.inner.finish()
    }
}

pub struct SerdeFieldSerializer<'s, S> where S: Serializer + 's {
    serializer: &'s mut S,
    state: S::MapState,
}

impl<'s, S> SerdeFieldSerializer<'s, S> where S: Serializer + 's  {
    pub fn new(serializer: &'s mut S) -> Result<Self, S::Error> {
        let state = serializer.serialize_map(None)?;

        Ok(SerdeFieldSerializer {
            serializer: serializer,
            state: state
        })
    }
}

impl<'s, S> FieldSerializer for SerdeFieldSerializer<'s, S> where S: Serializer + 's {
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

    fn finish(self) -> Result<(), Self::Error> {
        self.serializer.serialize_map_end(self.state)
    }
}

pub trait SerializeEvent {
    fn serialize<F: FieldSerializer>(&self, serializer: F) -> Result<(), F::Error>;
}

impl<T> SerializeEvent for T where T: LogstashEvent {
    fn serialize<F: FieldSerializer>(&self, mut serializer: F) -> Result<(), F::Error> {
        serializer.serialize_field_str("@timestamp", &self.timestamp().to_rfc3339())?; //TODO: TZ should be 'Z'
        serializer.serialize_field_str("@processed", &self.processed().to_rfc3339())?; //TODO: TZ should be 'Z'
        serializer.serialize_field_str("@version", self.version())?;
        serializer.serialize_field_str("@id", self.id().as_ref())?;
        serializer.serialize_field_str("type", self.event_type())?;
        serializer.serialize_field_tags("tags", self.tags().as_slice())?;
        if let Some(message) = self.message() {
            serializer.serialize_field_str("message", message.as_ref())?;
        }

        self.fields(&mut serializer)?;

        serializer.finish()
    }
}
