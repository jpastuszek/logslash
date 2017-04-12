#[macro_use]
extern crate bencher;
#[macro_use]
extern crate slog;
extern crate logslash;
extern crate futures;
extern crate tokio_core;
extern crate tokio_vec_io;
extern crate tempfile;

use bencher::Bencher;
use tokio_vec_io::BufStream;
use tokio_core::io::Io;
use futures::Future;
use futures::sync::mpsc::{channel, Sender, Receiver};
use futures::Sink;
use futures::future::ok;
use std::thread;

use logslash::{null_logger, event_loop};
use logslash::codec::syslog::{SyslogCodec, SyslogEvent};
use logslash::output::write::{write_blocking, write_threaded};
use logslash::serialize::JsonLogstashEventSerializer;
use logslash::serialize::Serializer;
use logslash::PipeError;
use futures::stream::Stream;
use tempfile::tempfile;

static SYSLOG_RFC5424_NEWLINE_EXAMPLES: &'static str =
r#"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo#012bar
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo#012bar
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum
"#;

static SYSLOG_RFC5424_NEWLINE_NO_META_EXAMPLES: &'static str =
r#"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo#012bar
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog 123 - - foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - - foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - - At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo#012bar
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog 123 - - foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - - foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - - At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum
"#;

fn syslog_rfc5424_newline_x10(bench: &mut Bencher) {
    let mut buf = Vec::from(SYSLOG_RFC5424_NEWLINE_EXAMPLES);
    bench.iter(|| {
        let input = BufStream::new(buf.as_mut_slice());
        let output = input.framed(SyslogCodec::rfc5424_in_newline_frame()).collect();
        assert_eq!(output.wait().expect("Ok result").len(), 10);
    })
}

fn syslog_rfc5424_newline_no_meta_x10(bench: &mut Bencher) {
    let mut buf = Vec::from(SYSLOG_RFC5424_NEWLINE_NO_META_EXAMPLES);
    bench.iter(|| {
        let input = BufStream::new(buf.as_mut_slice());
        let output = input.framed(SyslogCodec::rfc5424_in_newline_frame()).collect();
        assert_eq!(output.wait().expect("Ok result").len(), 10);
    })
}

fn syslog_rfc5424_newline_x10_to_logstash_json(bench: &mut Bencher) {
    let mut buf = Vec::from(SYSLOG_RFC5424_NEWLINE_EXAMPLES);
    let mut event_loop = event_loop();
    let handle = event_loop.handle();
    let logger = null_logger();

    bench.iter(move || {
        let input = BufStream::new(buf.as_mut_slice());

        let output = BufStream::default();
        let ser = JsonLogstashEventSerializer::default();

        let write = write_blocking(&logger, "syslog", handle.clone(), output, move |event, buf| {
            ser.serialize(event, buf).map(|_| ())
        });

        let pipe = input
            .framed(SyslogCodec::rfc5424_in_newline_frame())
            .map_err(|e| PipeError::Input(e))
            .forward(write);
        event_loop.run(pipe).expect("Ok result");
    })
}

fn syslog_rfc5424_newline_x10_to_logstash_json_to_file(bench: &mut Bencher) {
    let mut buf = Vec::from(SYSLOG_RFC5424_NEWLINE_EXAMPLES);
    let tempfile = tempfile().unwrap();

    let mut event_loop = event_loop();
    let (input, receiver): (Sender<SyslogEvent>, Receiver<SyslogEvent>) = channel(1);

    let logger = null_logger();
    let ser = JsonLogstashEventSerializer::default();

    let write = write_threaded(&logger, "syslog", tempfile, move |event, buf| {
        ser.serialize(event, buf).map(|_| ())
    });

    let pipe = receiver.forward(write);

    let handle = event_loop.handle();
    handle.spawn(pipe);

    bench.iter(move || {
        let data = BufStream::new(buf.as_mut_slice())
            .framed(SyslogCodec::rfc5424_in_newline_frame());
            //.map_err(|e| PipeError::Input(e));

        // send all test data and flush
        event_loop.run(data.forward(input)).expect("faile to spawn pipe");
    })
}

benchmark_group!(benches,
                 syslog_rfc5424_newline_x10,
                 syslog_rfc5424_newline_no_meta_x10,
                 syslog_rfc5424_newline_x10_to_logstash_json,
                 syslog_rfc5424_newline_x10_to_logstash_json_to_file);
benchmark_main!(benches);
