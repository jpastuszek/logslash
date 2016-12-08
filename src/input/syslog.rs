use super::nom::tcp_nom_input;
use super::parse;
pub use super::parse::Timestamp;
use std::net::SocketAddr;
use std::collections::HashMap;
use tokio_core::reactor::Handle;
use futures::sync::mpsc;
use nom::{ErrorKind, rest};

// TODO: use &str instead of String; make OwnedSyslogMessage variant that is Send

#[derive(Debug)]
pub struct StructuredElement {
    pub id: String,
    pub params: HashMap<String, String>
}

#[derive(Debug, Default)]
pub struct StructuredData {
    pub elements: Vec<StructuredElement>
}

#[derive(Debug)]
pub struct SyslogMessage {
    // TODO: enum facility and severity
    pub facility: u8,
    pub severity: u8,
    pub timestamp: Timestamp,
    pub hostname: String,
    pub app_name: Option<String>,
    pub proc_id: Option<String>,
    pub msg_id: Option<String>,
    pub structured_data: StructuredData,
    pub message: Option<String>
}

// "(?m)<%{POSINT:priority}>(?:%{SYSLOGTIMESTAMP:timestamp}|%{TIMESTAMP_ISO8601:timestamp8601}) (?:%{SYSLOGFACILITY} )?(:?%{SYSLOGHOST:logsource} )?(?<program>[^ \[]+)(?:\[%{POSINT:pid}\])?: %{GREEDYDATA:message}"

named!(priority<&[u8], u8>, return_error!(ErrorKind::Custom(1),
    delimited!(tag!(b"<"), map_res!(take_until!(">1"), parse::int_u8), tag!(b">1 "))));

named!(timestamp<&[u8], Timestamp>, return_error!(ErrorKind::Custom(2),
    terminated!(map_res!(take_until!(" "), parse::timestamp), tag!(b" "))));

named!(hostname<&[u8], &str>, return_error!(ErrorKind::Custom(3),
    do_parse!(
        not!(tag!("- ")) >>
        h: terminated!(map_res!(take_until!(" "), parse::string), tag!(b" ")) >>
        (h)
    )));

named!(opt_str<&[u8], Option<&str> >, map!(
        terminated!(map_res!(take_until!(" "), parse::string), tag!(b" ")),
        |s| if s == "-" { None } else { Some(s) }
    ));

named!(app_name<&[u8], Option<&str> >, return_error!(ErrorKind::Custom(4), opt_str));
named!(proc_id<&[u8], Option<&str> >, return_error!(ErrorKind::Custom(5), opt_str));
named!(msg_id<&[u8], Option<&str> >, return_error!(ErrorKind::Custom(6), opt_str));

named!(structured_data_param<&[u8], (&str, String)>, return_error!(ErrorKind::Custom(12), do_parse!(
        name: map_res!(take_until!("="), parse::string) >>
        tag!(b"=\"") >>
        value: map!(
            map_res!(escaped!(is_not!("\"\\"), '\\', is_a!("\"\\]")), parse::string),
            |s: &str|
                s.replace("\\\"", "\"")
                .replace("\\\\", "\\")
                .replace("\\]", "]")) >>
        tag!(b"\"") >>
        (name, value)
    )));

named!(structured_data_element<&[u8], StructuredElement>, return_error!(ErrorKind::Custom(11), do_parse!(
        tag!(b"[") >>
        id: map_res!(take_until!(" "), parse::string) >>
        tag!(b" ") >>
        params: separated_list!(tag!(b" "), call!(structured_data_param)) >>
        tag!(b"]") >>
        (StructuredElement {
            id: id.to_owned(),
            params: params.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
        })
    )));

named!(sp_or_eof<&[u8], &[u8]>, alt!(eof!() | tag!(b" ")));

named!(structured_data<&[u8], StructuredData>, do_parse!(
        minus: opt!(tag!(b"-")) >>
        res: cond_with_error!(minus.is_none(), many_till!(
            call!(structured_data_element),
            peek!(call!(sp_or_eof))
        )) >>
        (StructuredData {
            elements: res.map(|r| r.0).unwrap_or(Vec::new())
        })
    ));

// TODO: BOM support and octets
named!(message<&[u8], Option<&str> >, return_error!(ErrorKind::Custom(99), do_parse!(
        sp: call!(sp_or_eof) >>
        ret: cond_with_error!(sp == b" ", map_res!(rest, parse::string)) >>
        (ret)
    )));

named!(pub syslog_rfc5424<&[u8], SyslogMessage>, do_parse!(
    priority: priority >>
    timestamp: timestamp >>
    hostname: hostname >>
    app_name: app_name >>
    proc_id: proc_id >>
    msg_id: msg_id >>
    structured_data: structured_data >>
    message: message >>
    (SyslogMessage {
        facility: priority >> 3,
        severity: priority - (priority >> 3 << 3),
        timestamp: timestamp,
        hostname: hostname.to_owned(),
        app_name: app_name.map(|s| s.to_owned()),
        proc_id: proc_id.map(|s| s.to_owned()),
        msg_id: msg_id.map(|s| s.to_owned()),
        structured_data: structured_data,
        message: message.map(|s| s.to_owned())
    })));

named!(pub syslog_rfc5425_frame<&[u8], &[u8]>, do_parse!(
        msg_len: return_error!(ErrorKind::Custom(1),
           terminated!(map_res!(take_until!(" "), parse::int_u8), tag!(" "))) >>
        syslog_msg: take!(msg_len) >>
        (syslog_msg)
    ));

named!(pub syslog_rfc5424_in_rfc5425_frame<&[u8], SyslogMessage>,
       flat_map!(call!(syslog_rfc5425_frame), call!(syslog_rfc5424)));

pub mod simple_errors {
    use super::SyslogMessage;
    use nom::{IResult, ErrorKind};

    pub fn syslog_rfc5424(input: &[u8]) -> IResult<&[u8], SyslogMessage, &'static str> {
        super::syslog_rfc5424(input).map_err(|err| ErrorKind::Custom(match err {
            ErrorKind::Custom(1) => "Bad syslog priority tag format",
            ErrorKind::Custom(2) => "Unrecognized syslog timestamp format",
            ErrorKind::Custom(3) => "Expected syslog hostname",
            ErrorKind::Custom(4) => "Expected syslog application name",
            ErrorKind::Custom(5) => "Expected syslog process ID",
            ErrorKind::Custom(6) => "Expected syslog message ID",
            ErrorKind::Custom(7) => "Expected valid syslog structured data",
            ErrorKind::Custom(11) => "Failed to parse structured data element",
            ErrorKind::Custom(12) => "Failed to parse structured data parameter",
            ErrorKind::Custom(99) => "Bad syslog message payload",
            //_ => "Syslog parser did not match"
            e => panic!("{:?}", e)
        }))
    }

    pub fn syslog_rfc5425_frame(input: &[u8]) -> IResult<&[u8], &[u8], &'static str> {
        super::syslog_rfc5425_frame(input).map_err(|err| ErrorKind::Custom(match err {
            ErrorKind::Custom(1) => "Expected syslog RFC5425 frame message length",
            _ => "Syslog RFC5425 frame parser did not match"
        }))
    }

    named!(pub syslog_rfc5424_in_rfc5425_frame<&[u8], SyslogMessage, &'static str>,
           flat_map!(call!(syslog_rfc5425_frame), call!(syslog_rfc5424)));
}

pub fn tcp_syslog_input(handle: Handle, addr: &SocketAddr) -> mpsc::Receiver<SyslogMessage> {
    tcp_nom_input("syslog", handle, addr, simple_errors::syslog_rfc5424_in_rfc5425_frame)
}

#[cfg(test)]
mod syslog_rfc5425_frame_tests {
    use super::simple_errors::syslog_rfc5425_frame;

    #[test]
    fn length() {
        let (i, o) = syslog_rfc5425_frame(b"79 <165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo\nbarEOF").unwrap();
        assert_eq!(o, &b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo\nbar"[..]);
        assert_eq!(i, &b"EOF"[..]);
    }
}

#[cfg(test)]
mod syslog_rfc5424_tests {
    use super::simple_errors::syslog_rfc5424;
    use super::Timestamp;
    use nom::ErrorKind;

    #[test]
    fn priority() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.facility, 20);
        assert_eq!(o.severity, 5);
    }

    #[test]
    fn priority_error() {
        let err = syslog_rfc5424(b"<16x5>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Bad syslog priority tag format"));
    }

    #[test]
    fn timestamp() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.timestamp, Timestamp::parse_from_rfc3339("2003-10-11T22:14:15.003Z").unwrap());
    }

    #[test]
    fn timestamp_error() {
        let err = syslog_rfc5424(b"<165>1 XXXX-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Unrecognized syslog timestamp format"));
    }

    #[test]
    fn hostname() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.hostname, "mymachine.example.com");
    }

    #[test]
    fn hostname_error() {
        let err = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z \n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Expected syslog hostname"));
    }

    #[test]
    fn hostname_nil_error() {
        let err = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z - \n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Expected syslog hostname"));
    }

    #[test]
    fn app_name() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.app_name, Some("evntslog".to_owned()));
    }

    #[test]
    fn app_name_none() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.app_name, None);
    }

    #[test]
    fn app_name_error() {
        let err = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com \n").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Expected syslog application name"));
    }

    #[test]
    fn structured_data_single() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Application\" eventID=\"1011\"] foo\nbar").unwrap();
        assert!(i.is_empty());

        assert_eq!(o.structured_data.elements.len(), 1);

        let e = o.structured_data.elements.get(0).unwrap();
        assert_eq!(e.id, "exampleSDID@32473");
        assert_eq!(e.params.len(), 3);
        assert_eq!(e.params["iut"], "3".to_owned());
        assert_eq!(e.params["eventSource"], "Application".to_owned());
        assert_eq!(e.params["eventID"], "1011".to_owned());

        assert_eq!(o.message, Some("foo\nbar".to_owned()));
    }

    #[test]
    fn structured_data_multi() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Application\" eventID=\"1011\"][examplePriority@32473 class=\"high\"] foo\nbar").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.structured_data.elements.len(), 2);

        let e = o.structured_data.elements.get(0).unwrap();
        assert_eq!(e.id, "exampleSDID@32473");
        assert_eq!(e.params.len(), 3);
        assert_eq!(e.params["iut"], "3".to_owned());
        assert_eq!(e.params["eventSource"], "Application".to_owned());
        assert_eq!(e.params["eventID"], "1011".to_owned());

        let e = o.structured_data.elements.get(1).unwrap();
        assert_eq!(e.id, "examplePriority@32473");
        assert_eq!(e.params.len(), 1);
        assert_eq!(e.params["class"], "high".to_owned());

        assert_eq!(o.message, Some("foo\nbar".to_owned()));
    }

    #[test]
    fn structured_data_multi_escapes() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Appli\\\\catio\\]n\" eventID=\"1011\"][examplePriority@32473 class=\"hi\\\"gh\"] foo\nbar").unwrap();

        assert!(i.is_empty());
        assert_eq!(o.structured_data.elements.len(), 2);

        let e = o.structured_data.elements.get(0).unwrap();
        assert_eq!(e.id, "exampleSDID@32473");
        assert_eq!(e.params.len(), 3);
        assert_eq!(e.params["iut"], "3".to_owned());
        assert_eq!(e.params["eventSource"], "Appli\\catio]n".to_owned());
        assert_eq!(e.params["eventID"], "1011".to_owned());

        let e = o.structured_data.elements.get(1).unwrap();
        assert_eq!(e.id, "examplePriority@32473");
        assert_eq!(e.params.len(), 1);
        assert_eq!(e.params["class"], "hi\"gh".to_owned());

        assert_eq!(o.message, Some("foo\nbar".to_owned()));
    }

    #[test]
    fn structured_data_multi_no_message() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Application\" eventID=\"1011\"][examplePriority@32473 class=\"high\"]").unwrap();
        assert!(i.is_empty());

        assert_eq!(o.structured_data.elements.len(), 2);

        let e = o.structured_data.elements.get(0).unwrap();
        assert_eq!(e.id, "exampleSDID@32473");
        assert_eq!(e.params.len(), 3);
        assert_eq!(e.params["iut"], "3".to_owned());
        assert_eq!(e.params["eventSource"], "Application".to_owned());
        assert_eq!(e.params["eventID"], "1011".to_owned());

        let e = o.structured_data.elements.get(1).unwrap();
        assert_eq!(e.id, "examplePriority@32473");
        assert_eq!(e.params.len(), 1);
        assert_eq!(e.params["class"], "high".to_owned());

        assert_eq!(o.message, None);
    }

    #[test]
    fn message() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo\nbar").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.message, Some("foo\nbar".to_owned()));
    }

    #[test]
    fn message_error() {
        let err = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - baz\xc3\x28").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Bad syslog message payload"));
    }
}
