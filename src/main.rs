extern crate logslash;
extern crate futures;
extern crate chrono;

use logslash::event_loop;
use logslash::event::{Payload, Event};
use logslash::input::syslog::{SyslogEvent, tcp_syslog_input};
use logslash::output::debug::{DebugPort, print_event};
use logslash::serialize::JsonEventSerializer;
use futures::{Future, Stream};

#[derive(Debug)]
struct SyslogDebugPortEvent(SyslogEvent);

use std::borrow::Cow;
use chrono::{DateTime, UTC};

impl Event for SyslogDebugPortEvent {
    fn id(&self) -> Cow<str> { self.0.id() }
    fn source(&self) -> Cow<str> { self.0.source() }
    fn timestamp(&self) -> DateTime<UTC> { self.0.timestamp() }
    fn payload(&self) -> Option<Payload> { self.0.payload() }
}

impl DebugPort for SyslogDebugPortEvent {
    fn id(&self) -> Cow<str> { self.0.id() }
    fn timestamp(&self) -> DateTime<UTC> { self.0.timestamp() }
    fn source(&self) -> Cow<str> { self.0.source() }
}

fn main() { println!("Hello, world!");
    let mut event_loop = event_loop();
    let handle = event_loop.handle();

    let syslog = tcp_syslog_input(handle.clone(), &"127.0.0.1:5514".parse().unwrap());
    // syslog.rename() - need a future stream - Receiver is a Stream

    //let output = print_debug(syslog);

    let print = print_event(handle, JsonEventSerializer::default());

    //TODO: input and ouptut need to provide some printable error when they fail
    let pipe = syslog.map(SyslogDebugPortEvent).forward(print)
        .map_err(|e| println!("Error while processing pipe: {:?}", e));

    event_loop.run(pipe).expect("successful event loop run");
}
