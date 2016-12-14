extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate assert_matches;
extern crate chrono;
extern crate maybe_string;
extern crate uuid;
extern crate serde;
extern crate serde_json;

mod input;
mod output;
mod event;

use tokio_core::reactor::Core;

use input::syslog::tcp_syslog_input;
use output::debug::print_logstash;

fn main() {
    println!("Hello, world!");

    let mut event_loop = Core::new().unwrap();
    let handle = event_loop.handle();

    let syslog = tcp_syslog_input(handle, &"127.0.0.1:5514".parse().unwrap());
    //let output = print_debug(syslog);
    let output = print_logstash(syslog);
    event_loop.run(output).expect("successful event loop run");
}
