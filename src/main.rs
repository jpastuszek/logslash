extern crate logslash;
extern crate futures;
extern crate chrono;

use logslash::event_loop;
use logslash::event::Event;
use logslash::input::syslog::{SyslogEvent, tcp_syslog_input};
use logslash::output::debug::DebugPort;
use logslash::output::debug::*;
use logslash::serialize::Serializer;
//use logslash::serialize::JsonEventSerializer;
use logslash::serialize::JsonLogstashEventSerializer;

use futures::{Future, Stream};
use std::borrow::Cow;
use std::io::Write;
use std::fs::File;
use chrono::{DateTime, UTC};

//TODO:
// * support for arbitary fileds in messages
// * renames
// * put events behind Rc to reduce copying?
// * use structured logging
// * proper nom errors with dumps etc
// * reduce expect/unwrap for pipeline setup?
// * parse common syslog messages
// * auto-select syslog parser based on message
// * benches
// * use CPU thread pools for processing of inputs and outputs
// * dead letters and parsing error logging
// * Kafka output
// * prelude with common input/output/codecs

#[derive(Debug)]
struct SyslogDebugPortEvent(SyslogEvent);

impl DebugPort for SyslogDebugPortEvent {
    type Payload = SyslogEvent;

    fn id(&self) -> Cow<str> { self.0.id() }
    fn timestamp(&self) -> DateTime<UTC> { self.0.timestamp() }
    fn source(&self) -> Cow<str> { self.0.source() }
    fn write_payload<W: Write, S: Serializer<Self::Payload>>(&self, out: W, serializer: &S) -> Result<W, S::Error> {
        serializer.serialize(&self.0, out)
    }
}

fn main() {
    let mut event_loop = event_loop();
    let handle = event_loop.handle();

    let syslog = tcp_syslog_input(handle.clone(), &"127.0.0.1:5514".parse().unwrap());
    // syslog.rename() - need a future stream - Receiver is a Stream

    //let print = debug_print(JsonLogstashEventSerializer::default());
    let print = debug_to_file(File::create("/tmp/out").expect("falied to open out file"), JsonLogstashEventSerializer::default());

    //TODO: input and ouptut need to provide some printable error when they fail
    let pipe = syslog.map(SyslogDebugPortEvent).forward(print)
        .map_err(|e| println!("Error while processing pipe: {:?}", e));

    event_loop.run(pipe).expect("successful event loop run");
}
