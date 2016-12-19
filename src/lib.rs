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

pub mod input;
pub mod output;
pub mod event;
pub mod serialize;

use tokio_core::reactor::Core;

pub fn event_loop() -> Core {
    Core::new().expect("Tokio Core event loop")
}
