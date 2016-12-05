use super::nom::tcp_nom_input;
use super::parse;
pub use super::parse::Timestamp;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;
use futures::sync::mpsc;
use nom::{IResult, ErrorKind};

#[derive(Debug)]
pub struct SyslogMessage {
    pub facility: u8,
    pub severity: u8,
    pub timestamp: Timestamp,
    pub hostname: String,
    pub message: String
}

// "(?m)<%{POSINT:priority}>(?:%{SYSLOGTIMESTAMP:timestamp}|%{TIMESTAMP_ISO8601:timestamp8601}) (?:%{SYSLOGFACILITY} )?(:?%{SYSLOGHOST:logsource} )?(?<program>[^ \[]+)(?:\[%{POSINT:pid}\])?: %{GREEDYDATA:message}"

named!(priority<&[u8], u8>, return_error!(ErrorKind::Custom(1),
    delimited!(tag!(b"<"), map_res!(take_until!(&b">1"[..]), parse::int_u8), tag!(b">1 "))));

named!(timestamp<&[u8], Timestamp>, return_error!(ErrorKind::Custom(2),
    terminated!(map_res!(take_until!(&b" "[..]), parse::timestamp), tag!(b" "))));

/* optional hostname
named!(hostname<&[u8], &str>, return_error!(ErrorKind::Custom(3),
    switch!(peek!(take!(2)),
        b"- " => map!(take!(2), |_| None) |
        _ => map!(terminated!(map_res!(take_until!(&b" "[..]), parse::string), tag!(b" ")), |s| Some(s))
    )));
*/

named!(hostname<&[u8], &str>, return_error!(ErrorKind::Custom(3),
    do_parse!(
        not!(tag!("- ")) >>
        h: terminated!(map_res!(take_until!(&b" "[..]), parse::string), tag!(b" ")) >>
        (h)
    )));

named!(message<&[u8], &str>, return_error!(ErrorKind::Custom(99),
    terminated!(map_res!(take_until!(&b"\n"[..]), parse::string), tag!(b"\n"))));

named!(syslog_parser_full<&[u8], SyslogMessage>, chain!(
    priority: priority ~
    timestamp: timestamp ~
    hostname: hostname ~
    message: message,
    || SyslogMessage {
        facility: priority >> 3,
        severity: priority - (priority >> 3 << 3),
        timestamp: timestamp,
        hostname: hostname.to_owned(),
        message: message.to_owned()
    }));

pub fn syslog_parser(input: &[u8]) -> IResult<&[u8], SyslogMessage, &'static str> {
    syslog_parser_full(input).map_err(|err| ErrorKind::Custom(match err {
            ErrorKind::Custom(1) => "Bad syslog priority tag format",
            ErrorKind::Custom(2) => "Unrecognized syslog timestamp format",
            ErrorKind::Custom(3) => "Expected syslog hostname",
            ErrorKind::Custom(99) => "Bad syslog message payload",
            _ => "Syslog parser did not match"
    }))
}

pub fn tcp_syslog_input(handle: Handle, addr: &SocketAddr) -> mpsc::Receiver<SyslogMessage> {
    tcp_nom_input("syslog", handle, addr, syslog_parser)
}

#[cfg(test)]
mod parser_tests {
    use super::syslog_parser;
    use super::Timestamp;
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
        assert_matches!(err, ErrorKind::Custom("Bad syslog priority tag format"));
    }

    #[test]
    fn timestamp() {
        let (i, o) = syslog_parser(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - foobar\n").unwrap();
        assert_eq!(i, b"");
        assert_eq!(o.timestamp, Timestamp::parse_from_rfc3339("2003-10-11T22:14:15.003Z").unwrap());
    }

    #[test]
    fn timestamp_error() {
        let err = syslog_parser(b"<165>1 XXXX-10-11T22:14:15.003Z mymachine.example.com evntslog - foobar\n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Unrecognized syslog timestamp format"));
    }

    #[test]
    fn hostname() {
        let (i, o) = syslog_parser(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - foobar\n").unwrap();
        assert_eq!(i, b"");
        assert_eq!(o.hostname, "mymachine.example.com");
    }

    #[test]
    fn hostname_error() {
        let err = syslog_parser(b"<165>1 2003-10-11T22:14:15.003Z \n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Expected syslog hostname"));
    }

    #[test]
    fn hostname_nil_error() {
        let err = syslog_parser(b"<165>1 2003-10-11T22:14:15.003Z - \n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Expected syslog hostname"));
    }

    #[test]
    fn message_error() {
        let err = syslog_parser(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - \xc3\x28\n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Bad syslog message payload"));
    }
}
