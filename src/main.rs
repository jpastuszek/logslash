extern crate logslash;
extern crate futures;
extern crate chrono;

use logslash::event_loop;
use logslash::event::{Payload, Event};
use logslash::input::syslog::{SyslogEvent, tcp_syslog_input};
use logslash::output::debug::{DebugPort, print_serde_json};
use futures::stream::Stream;

struct SyslogDebugPortEvent(SyslogEvent);

use std::borrow::Cow;
use chrono::{DateTime, UTC};

impl DebugPort for SyslogDebugPortEvent {
    fn id(&self) -> Cow<str> { self.0.id() }
    fn timestamp(&self) -> DateTime<UTC> { self.0.timestamp() }
    fn source(&self) -> Cow<str> { self.0.source() }
}

fn main() {
    println!("Hello, world!");

    let mut event_loop = event_loop();
    let handle = event_loop.handle();

    let syslog = tcp_syslog_input(handle.clone(), &"127.0.0.1:5514".parse().unwrap());
    // syslog.rename() - need a future stream - Receiver is a Stream

    //let output = print_debug(syslog);

    let print = print_serde_json(handle, "serializer");

    let pipe = syslog.map(SyslogDebugPortEvent).forward(print)
        .map_err(|e| println!("Error while processing output: {}", e))
        .for_each(|_| Ok(()));

    event_loop.run(pipe).expect("successful event loop run");
}
