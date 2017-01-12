use std::net::SocketAddr;

use futures::stream::Stream;
use tokio_core::reactor::Handle;

use PipeError;
use codec::syslog::SyslogCodec;
pub use codec::syslog::SyslogEvent;

use input::tcp::tcp_input;

pub fn tcp_syslog_input<OE>(handle: Handle, addr: &SocketAddr) -> Box<Stream<Item=SyslogEvent, Error=PipeError<(), OE>>> {
    tcp_input("syslog", handle, addr, SyslogCodec::rfc5424_in_newline_frame())
}
