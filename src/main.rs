extern crate logslash;
extern crate futures;
extern crate chrono;

use logslash::event_loop;
use logslash::event::{Event, AsEvent, AsLogstashEvent};
use logslash::input::syslog::{SyslogEvent, tcp_syslog_input};
use logslash::output::debug::{DebugPort, print_event};
use logslash::serialize::JsonLogstashEventSerializer;
use futures::{Future, Stream};
use std::borrow::Cow;
use chrono::{DateTime, UTC};

//TODO:
// * use structured logging
// * reduce expect/unwrap for pipeline setup?
// * use CPU thread pools for processing of inputs and outputs

#[derive(Debug)]
struct SyslogDebugPortEvent(SyslogEvent);

impl AsEvent for SyslogDebugPortEvent {
    type Event = SyslogEvent;
    fn as_event(&self) -> &SyslogEvent {
        &self.0
    }
}

impl AsLogstashEvent for SyslogDebugPortEvent {
    type LogstashEvent = SyslogEvent;
    fn as_logstash_event(&self) -> &SyslogEvent {
        &self.0
    }
}

impl DebugPort for SyslogDebugPortEvent {
    fn id(&self) -> Cow<str> { self.0.id() }
    fn timestamp(&self) -> DateTime<UTC> { self.0.timestamp() }
    fn source(&self) -> Cow<str> { self.0.source() }
}

fn main() {
    let mut event_loop = event_loop();
    let handle = event_loop.handle();

    let syslog = tcp_syslog_input(handle.clone(), &"127.0.0.1:5514".parse().unwrap());
    // syslog.rename() - need a future stream - Receiver is a Stream

    //let output = print_debug(syslog);

    let print = print_event(handle, JsonLogstashEventSerializer::default());

    //TODO: input and ouptut need to provide some printable error when they fail
    let pipe = syslog.map(SyslogDebugPortEvent).forward(print)
        .map_err(|e| println!("Error while processing pipe: {:?}", e));

    event_loop.run(pipe).expect("successful event loop run");
}
