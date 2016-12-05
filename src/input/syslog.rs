use std::str::{from_utf8, Utf8Error};
use std::num::ParseIntError;

use super::nom::tcp_nom_input;

use std::net::SocketAddr;
use tokio_core::reactor::Handle;
use futures::sync::mpsc;

#[derive(Debug)]
pub struct SyslogMessage {
    pub facility: u8,
    pub severity: u8,
}

fn str(bytes: &[u8]) -> Result<&str, Utf8Error> {
    from_utf8(bytes)
}

enum InputIntError {
    Utf8Error(Utf8Error),
    ParseIntError(ParseIntError)
}

fn number_u8(bytes: &[u8]) -> Result<u8, InputIntError> {
    str(bytes).map_err(InputIntError::Utf8Error)
        .and_then(|s| s.parse().map_err(InputIntError::ParseIntError))
}

// "(?m)<%{POSINT:priority}>(?:%{SYSLOGTIMESTAMP:timestamp}|%{TIMESTAMP_ISO8601:timestamp8601}) (?:%{SYSLOGFACILITY} )?(:?%{SYSLOGHOST:logsource} )?(?<program>[^ \[]+)(?:\[%{POSINT:pid}\])?: %{GREEDYDATA:message}"

named!(syslog_parser<&[u8], SyslogMessage>, chain!(
    priority: delimited!(tag!(b"<"),
                         map_res!(take_until!(&b">"[..]), number_u8),
                         tag!(b">")),
    || SyslogMessage {
        facility: priority >> 3,
        severity: priority - (priority >> 3 << 3)
    }));

pub fn tcp_syslog_input(handle: Handle, addr: &SocketAddr) -> mpsc::Receiver<SyslogMessage> {
    tcp_nom_input("syslog", handle, addr, syslog_parser)
}


