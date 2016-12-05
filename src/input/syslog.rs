use super::nom::tcp_nom_input;
use super::parse;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;
use futures::sync::mpsc;
use nom::{IResult, ErrorKind};

#[derive(Debug)]
pub struct SyslogMessage {
    pub facility: u8,
    pub severity: u8,
    pub message: String
}

// "(?m)<%{POSINT:priority}>(?:%{SYSLOGTIMESTAMP:timestamp}|%{TIMESTAMP_ISO8601:timestamp8601}) (?:%{SYSLOGFACILITY} )?(:?%{SYSLOGHOST:logsource} )?(?<program>[^ \[]+)(?:\[%{POSINT:pid}\])?: %{GREEDYDATA:message}"

named!(priority<&[u8], u8>, return_error!(ErrorKind::Custom(1),
    delimited!(tag!(b"<"), map_res!(take_until!(&b">"[..]), parse::int_u8), tag!(b">"))));

named!(message<&[u8], &str>, return_error!(ErrorKind::Custom(99),
    terminated!(map_res!(take_until!(&b"\n"[..]), parse::string), tag!(b"\n"))));

named!(syslog_parser_full<&[u8], SyslogMessage>, chain!(
    priority: priority ~
    message: message,
    || SyslogMessage {
        facility: priority >> 3,
        severity: priority - (priority >> 3 << 3),
        message: message.to_owned()
    }));

pub fn syslog_parser(input: &[u8]) -> IResult<&[u8], SyslogMessage, &'static str> {
    syslog_parser_full(input).map_err(|err| ErrorKind::Custom(match err {
            ErrorKind::Custom(1) => "bad syslog priority tag format",
            ErrorKind::Custom(99) => "bad syslog message payload",
            _ => "syslog parser did not match"
    }))
}

pub fn tcp_syslog_input(handle: Handle, addr: &SocketAddr) -> mpsc::Receiver<SyslogMessage> {
    tcp_nom_input("syslog", handle, addr, syslog_parser)
}

#[cfg(test)]
mod parser_tests {
    use super::syslog_parser;
    use nom::ErrorKind;

    #[test]
    fn priority() {
        let (i, o) = syslog_parser(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - foobar\n").unwrap();
        assert_eq!(i, b"");
        assert_eq!(o.facility, 20);
        assert_eq!(o.severity, 5);
    }

    #[test]
    fn priority_error() {
        let err = syslog_parser(b"<16x5>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - foobar\n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("bad syslog priority tag format"));
    }

    #[test]
    fn message_error() {
        let err = syslog_parser(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - \xc3\x28\n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("bad syslog message payload"));
    }
}
