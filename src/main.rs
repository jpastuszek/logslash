extern crate logslash;

use logslash::event_loop;
use logslash::input::syslog::tcp_syslog_input;
use logslash::output::debug::print_logstash;

fn main() {
    println!("Hello, world!");

    let mut event_loop = event_loop();
    let handle = event_loop.handle();

    let syslog = tcp_syslog_input(handle, &"127.0.0.1:5514".parse().unwrap());
    //let output = print_debug(syslog);
    let output = print_logstash(syslog);
    event_loop.run(output).expect("successful event loop run");
}
