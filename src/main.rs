extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate assert_matches;

mod input;

use futures::Stream;
use tokio_core::reactor::Core;

use input::syslog::tcp_syslog_input;

fn main() {
    println!("Hello, world!");

    let mut event_loop = Core::new().unwrap();
    let handle = event_loop.handle();

    let input = tcp_syslog_input(handle, &"127.0.0.1:5514".parse().unwrap())
        .for_each(|message| {
            println!("got syslog message: {:?}", message);
            Ok(())
        });

    event_loop.run(input).expect("successful event loop run");
}
