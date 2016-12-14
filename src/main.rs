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
mod event;

use futures::Stream;
use tokio_core::reactor::Core;

use input::syslog::tcp_syslog_input;

use std::io::Cursor;
use maybe_string::MaybeString;
use event::{SerializeEvent, SerdeFieldSerializer, FieldSerializer};

fn main() {
    println!("Hello, world!");

    let mut event_loop = Core::new().unwrap();
    let handle = event_loop.handle();

    let input = tcp_syslog_input(handle, &"127.0.0.1:5514".parse().unwrap())
        .for_each(|message| {
            println!("Got syslog message: {:#?}", &message);

            let data = Cursor::new(Vec::new());
            let mut ser = serde_json::ser::Serializer::new(data);

            {
                let field_ser = SerdeFieldSerializer::new(&mut ser).expect("field serializer")
                    .rename("severity", "log_level");
                message.serialize(field_ser).expect("serialized message");
            }

            let json = ser.into_inner().into_inner();
            println!("JSON: {}", MaybeString(json));

            Ok(())
        });

    event_loop.run(input).expect("successful event loop run");
}
