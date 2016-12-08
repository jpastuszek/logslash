use super::nom::tcp_nom_input;
use super::parse;
pub use super::parse::Timestamp;
use std::net::SocketAddr;
use std::collections::HashMap;
use tokio_core::reactor::Handle;
use futures::sync::mpsc;
use nom::{ErrorKind, rest};
use maybe_string::{MaybeStr, MaybeString};
use std::mem;

// TODO: use &str instead of String; make OwnedSyslogMessage variant that is Send

#[derive(Debug, Clone)]
pub struct StructuredElement {
    pub id: String,
    pub params: HashMap<String, String>
}

#[derive(Debug, Clone, Default)]
pub struct StructuredData {
    pub elements: Vec<StructuredElement>
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    String(String),
    MaybeString(MaybeString)
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u8)]
#[derive(Debug)]
pub enum Facility {
	KernelMessages = 0,
	UserLevelMessages = 1,
	MailSystem = 2,
	SystemDaemons = 3,
	SecurityMessages = 4,
	Internal = 5,
	LinePrinterSubsystem = 6,
	NetworkNewsSubsystem = 7,
	UucpSubsystem = 8,
	ClockDaemon = 9,
	SecurityMessages2 = 10,
	FtpDaemon = 11,
	NtpSubsystem = 12,
	LogAudit = 13,
	LogAlert = 14,
	ClockDaemon2 = 15,
	Local0 = 16,
	Local1 = 17,
	Local2 = 18,
	Local3 = 19,
	Local4 = 20,
	Local5 = 21,
	Local6 = 22,
	Local7 = 23,
}

impl Facility {
    fn from_priority(priority: u8) -> Result<Facility, &'static str> {
        let facility = priority >> 3;
        if facility > 23 {
            return Err("facility values MUST be in the range of 0 to 23 inclusive")
        }
        Ok(unsafe { mem::transmute(facility) })
    }
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u8)]
#[derive(Debug)]
pub enum Severity {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Informational = 6,
    Debug = 7
}

impl Severity {
    fn from_priority(priority: u8) -> Severity {
        let severity = priority - (priority >> 3 << 3);
        unsafe { mem::transmute(severity) }
    }
}

#[derive(Debug, Clone)]
pub struct SyslogEvent {
    pub facility: Facility,
    pub severity: Severity,
    pub timestamp: Timestamp,
    pub hostname: String,
    pub app_name: Option<String>,
    pub proc_id: Option<String>,
    pub msg_id: Option<String>,
    pub structured_data: StructuredData,
    pub message: Option<Message>
}

named!(priority<&[u8], u8>, return_error!(ErrorKind::Custom(1),
    delimited!(tag!(b"<"), map_res!(take_until!(">1"), parse::int_u8), tag!(b">"))));

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

// Messages to be interpreted as UTF-8 strings need to be preceded with UTF-8 BOM
const BOM: &'static[u8] = &[0xEF,0xBB,0xBF];
named!(message<&[u8], Option<Message> >, return_error!(ErrorKind::Custom(20), do_parse!(
        sp: call!(sp_or_eof) >>
        ret: cond_with_error!(sp == b" ", do_parse!(
            bom: opt!(tag!(BOM)) >>
            ret: map_res!(rest,
                |bytes| if bom.is_some() {
                    parse::string(bytes).map(|s| Message::String(s.to_owned()))
                } else {
                    Ok(Message::MaybeString(MaybeStr::from_bytes(bytes).to_maybe_string()))
                }) >>
            (ret)
        )) >>
        (ret)
    )));

named!(pub syslog_rfc5424<&[u8], SyslogEvent>, do_parse!(
    facility: map_res!(peek!(priority), |p| Facility::from_priority(p)) >>
    severity: map!(priority, |p| Severity::from_priority(p)) >>
    tag!(b"1 ") >> // Fromat version 1
    timestamp: timestamp >>
    hostname: hostname >>
    app_name: app_name >>
    proc_id: proc_id >>
    msg_id: msg_id >>
    structured_data: structured_data >>
    message: message >>
    (SyslogEvent {
        facility: facility,
        severity: severity,
        timestamp: timestamp,
        hostname: hostname.to_owned(),
        app_name: app_name.map(|s| s.to_owned()),
        proc_id: proc_id.map(|s| s.to_owned()),
        msg_id: msg_id.map(|s| s.to_owned()),
        structured_data: structured_data,
        message: message
    })));

named!(pub syslog_rfc5425_frame<&[u8], &[u8]>, do_parse!(
        msg_len: return_error!(ErrorKind::Custom(1),
           terminated!(map_res!(take_until!(" "), parse::int_u8), tag!(" "))) >>
        syslog_msg: take!(msg_len) >>
        (syslog_msg)
    ));

named!(pub syslog_rfc5424_in_rfc5425_frame<&[u8], SyslogEvent>,
       flat_map!(call!(syslog_rfc5425_frame), call!(syslog_rfc5424)));

pub mod simple_errors {
    use super::SyslogEvent;
    use nom::{IResult, ErrorKind};

    pub fn syslog_rfc5424(input: &[u8]) -> IResult<&[u8], SyslogEvent, &'static str> {
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
            ErrorKind::Custom(20) => "Bad syslog message payload encoding",
            _ => "Syslog parser did not match"
        }))
    }

    pub fn syslog_rfc5425_frame(input: &[u8]) -> IResult<&[u8], &[u8], &'static str> {
        super::syslog_rfc5425_frame(input).map_err(|err| ErrorKind::Custom(match err {
            ErrorKind::Custom(1) => "Expected syslog RFC5425 frame message length",
            _ => "Syslog RFC5425 frame parser did not match"
        }))
    }

    named!(pub syslog_rfc5424_in_rfc5425_frame<&[u8], SyslogEvent, &'static str>,
           flat_map!(call!(syslog_rfc5425_frame), call!(syslog_rfc5424)));
}

pub fn tcp_syslog_input(handle: Handle, addr: &SocketAddr) -> mpsc::Receiver<SyslogEvent> {
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
    use super::{Timestamp, Message, Facility, Severity};
    use maybe_string::MaybeString;
    use nom::ErrorKind;

    #[test]
    fn priority() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.facility, Facility::Local4);
        assert_eq!(o.severity, Severity::Notice);
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
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Application\" eventID=\"1011\"] \xEF\xBB\xBFfoo\nbar").unwrap();
        assert!(i.is_empty());

        assert_eq!(o.structured_data.elements.len(), 1);

        let e = o.structured_data.elements.get(0).unwrap();
        assert_eq!(e.id, "exampleSDID@32473");
        assert_eq!(e.params.len(), 3);
        assert_eq!(e.params["iut"], "3".to_owned());
        assert_eq!(e.params["eventSource"], "Application".to_owned());
        assert_eq!(e.params["eventID"], "1011".to_owned());

        assert_eq!(o.message, Some(Message::String("foo\nbar".to_owned())));
    }

    #[test]
    fn structured_data_multi() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Application\" eventID=\"1011\"][examplePriority@32473 class=\"high\"] \xEF\xBB\xBFfoo\nbar").unwrap();
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

        assert_eq!(o.message, Some(Message::String("foo\nbar".to_owned())));
    }

    #[test]
    fn structured_data_multi_escapes() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Appli\\\\catio\\]n\" eventID=\"1011\"][examplePriority@32473 class=\"hi\\\"gh\"] \xEF\xBB\xBFfoo\nbar").unwrap();

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

        assert_eq!(o.message, Some(Message::String("foo\nbar".to_owned())));
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
    fn message_maybe_string() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo\nbar").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.message, Some(Message::MaybeString(MaybeString::from_bytes(b"foo\nbar".as_ref().to_owned()))));
    }

    #[test]
    fn message_bom() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - \xEF\xBB\xBFfoo\nbar").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.message, Some(Message::String("foo\nbar".to_owned())));
    }

    #[test]
    fn message_bom_bad_utf8() {
        let err = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - \xEF\xBB\xBFbaz\xc3\x28").unwrap_err();
        assert_matches!(err, ErrorKind::Custom("Bad syslog message payload encoding"));
    }
}
