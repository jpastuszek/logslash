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

use logslash::codec::syslog::SyslogCodec;
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

benchmark_group!(benches,
                 syslog_rfc5424_newline_x10,
                 syslog_rfc5424_newline_no_meta_x10);
benchmark_main!(benches);
