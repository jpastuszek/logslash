#[macro_use]
extern crate bencher;
extern crate logslash;

use bencher::Bencher;

use logslash::event_loop;

static syslog_rfc5424_newline_examples: &'static str = r#"
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo#012bar
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165?1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - 123 - [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"] foo
"#;

fn syslog_rfc5424_newline(bench: &mut Bencher) {
    let mut event_loop = event_loop();
    let handle = event_loop.handle();

    // I need IO here so wI can test the Codec on it
    // Only TcpStream implements it
    // But if I have PoolEvented<E> where E: Read + Write then I get it implemented as well on it
    // E has to be Evented (from mio) and is implemeted for EventedFd (unix file discriptiors)
    // and mio::channel::Receiver but you cannot Read + Write from it
    // There is a crate for Unix sockets as well but  nothing for plain files unless we use
    // EventedFd
    // I could also impement IO on a Cursor and use it as input and output

    bench.iter(|| {

    })
}

benchmark_group!(benches, syslog_rfc5424_newline);
benchmark_main!(benches);
