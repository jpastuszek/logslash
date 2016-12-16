extern crate logslash;
extern crate futures;

use logslash::event_loop;
use logslash::input::syslog::tcp_syslog_input;
use logslash::output::debug::print_serde_json;
use futures::stream::Stream;
use logslash::event::LogstashEvent;

fn main() {
    println!("Hello, world!");

    let mut event_loop = event_loop();
    let handle = event_loop.handle();

    let syslog = tcp_syslog_input(handle, &"127.0.0.1:5514".parse().unwrap());
    // syslog.rename() - need a future stream - Receiver is a Stream

    //let output = print_debug(syslog);
    let output = print_serde_json(syslog).map(|s| s.to_event());
    let outputs = output.map_err(|e| println!("Error while processing output: {}", e)).for_each(|_| Ok(()));
    event_loop.run(outputs).expect("successful event loop run");
}
