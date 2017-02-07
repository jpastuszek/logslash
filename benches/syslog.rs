#[macro_use]
extern crate bencher;
extern crate logslash;
extern crate futures;
extern crate tokio_core;
extern crate tokio_vec_io;

use bencher::Bencher;
use tokio_vec_io::BufStream;
use tokio_core::io::Io;
use futures::Future;

use logslash::event_loop;
use logslash::codec::syslog::SyslogCodec;
use logslash::output::write::write;
use logslash::serialize::JsonLogstashEventSerializer;
use logslash::serialize::Serializer;
use logslash::PipeError;
use futures::stream::Stream;

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

    bench.iter(move || {
        let input = BufStream::new(buf.as_mut_slice());

        let output = BufStream::default();
        let ser = JsonLogstashEventSerializer::default();

        let write = write(handle.clone(), output, move |event, buf| {
            ser.serialize(event, buf).map(|_| ())
        });

        let pipe = input
            .framed(SyslogCodec::rfc5424_in_newline_frame())
            .map_err(|e| PipeError::Input(e))
            .forward(write);
        event_loop.run(pipe).expect("Ok result");
    })
}

benchmark_group!(benches,
                 syslog_rfc5424_newline_x10,
                 syslog_rfc5424_newline_no_meta_x10,
                 syslog_rfc5424_newline_x10_to_logstash_json);
benchmark_main!(benches);
