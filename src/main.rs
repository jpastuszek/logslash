#[macro_use]
extern crate slog; extern crate logslash;
extern crate futures;
extern crate chrono;

use logslash::{terminal_logger, event_loop};
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
// * benchmar for debug_to_file output
// * use codec to serialize into buffer owned by Framed
// * support for arbitary fileds in messages
// * put events behind Rc to reduce copying?
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

    let logger = terminal_logger();
    info!(&logger, "Setting up pipline");

    let syslog = tcp_syslog_input(&logger, handle.clone(), &"127.0.0.1:5514".parse().unwrap());
    // syslog.rename() - need a future stream - Receiver is a Stream

    //let print = debug_print(&logger, JsonLogstashEventSerializer::default());
    let print = debug_to_file(&logger, File::create("/tmp/out").expect("falied to open out file"), JsonLogstashEventSerializer::default());

    //TODO: input and ouptut need to provide some printable error when they fail
    let pipe = syslog.map(SyslogDebugPortEvent).forward(print)
        .map_err(|e| error!(&logger, "Error while processing pipe: {:?}", e));

    info!(logger, "Running pipline");
    event_loop.run(pipe).expect("successful event loop run");
    info!(logger, "Pipline done");
}
