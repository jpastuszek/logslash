use slog::Logger;
use std::net::SocketAddr;
use std::fmt::Debug;

use futures::stream::Stream;
use tokio_core::reactor::Handle;

use PipeError;
use codec::nom::{NomCodec, NomParser};

use input::tcp::tcp_input;

pub fn tcp_nom_input<T, OE>(logger: &Logger, name: &'static str, handle: Handle, addr: &SocketAddr, parser: NomParser<T>) -> Box<Stream<Item=T, Error=PipeError<(), OE>>> where T: Debug + 'static {
    tcp_input(logger, name, handle, addr, NomCodec::new(parser))
}
