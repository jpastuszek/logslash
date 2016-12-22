use std::borrow::Cow;
use std::fmt;
use maybe_string::MaybeStr;
use chrono::{DateTime, UTC};

//pub enum FieldValue<'f> {
    //String(&'f str),
    //U64(u64),
//}

#[derive(Debug, PartialEq)]
pub enum Payload<'e> {
    String(Cow<'e, str>),
    Data(Cow<'e, MaybeStr>)
}

impl<'e> fmt::Display for Payload<'e> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Payload::String(Cow::Borrowed(ref s)) => write!(f, "{}", s),
            Payload::String(Cow::Owned(ref s)) => write!(f, "{}", s),
            Payload::Data(Cow::Borrowed(ref s)) => write!(f, "<DATA>{}", s),
            Payload::Data(Cow::Owned(ref s)) => write!(f, "<DATA>{}", s),
        }
    }
}

pub trait Event {
    //type FieldsIter: Iterator<Item=(&'static str, FieldValue<'f>)>;

    fn id(&self) -> Cow<str>;
    fn source(&self) -> Cow<str>;
    fn timestamp(&self) -> DateTime<UTC>;
    fn payload(&self) -> Option<Payload>;

    //TODO: id, source, message
    //fn fields(&self) -> Self::FieldsIter;
}

pub trait AsEvent {
    type Event: Event;
    fn as_event(&self) -> &Self::Event;
}

impl<T> Event for T where T: AsEvent {
    fn id(&self) -> Cow<str> { self.as_event().id() }
    fn source(&self) -> Cow<str> { self.as_event().source() }
    fn timestamp(&self) -> DateTime<UTC> { self.as_event().timestamp() }
    fn payload(&self) -> Option<Payload> { self.as_event().payload() }
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
    //fn fields<F: FieldSerializer>(&self, serializer: &mut F) -> Result<(), F::Error>;
}

pub trait AsLogstashEvent {
    type LogstashEvent: LogstashEvent;
    fn as_logstash_event(&self) -> &Self::LogstashEvent;
}

impl<T> LogstashEvent for T where T: AsLogstashEvent {
    fn timestamp(&self) -> DateTime<UTC> { self.as_logstash_event().timestamp() }
    fn version(&self) -> &str { self.as_logstash_event().version() }
    fn message(&self) -> Option<Cow<str>> { self.as_logstash_event().message() }
    fn event_type(&self) -> &str { self.as_logstash_event().event_type() }
    fn tags(&self) -> Vec<&'static str> { self.as_logstash_event().tags() }
    fn processed(&self) -> DateTime<UTC> { self.as_logstash_event().processed() }
    fn id(&self) -> Cow<str> { self.as_logstash_event().id() }
}
