use super::nom::tcp_nom_input;
use super::parse;
use event::{Payload, Event, LogstashEvent, FieldSerializer};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::mem;
use std::borrow::Cow;
use tokio_core::reactor::Handle;
use futures::sync::mpsc;
use nom::{ErrorKind, rest};
use maybe_string::{MaybeStr, MaybeString};
use chrono::{DateTime, UTC, FixedOffset};
use uuid::Uuid;

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
#[allow(dead_code)]
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
	AuthPrivMessage = 10,
	FtpDaemon = 11,
	NtpSubsystem = 12,
	LogAudit = 13,
	LogAlert = 14,
	SchedulingDaemon = 15,
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
#[allow(dead_code)]
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
    pub timestamp: DateTime<FixedOffset>,
    pub hostname: String,
    pub program: Option<String>,
    pub proc_id: Option<String>,
    pub msg_id: Option<String>,
    pub structured_data: Option<StructuredData>,
    pub message: Option<Message>,
    pub processed: DateTime<UTC>,
}

impl SyslogEvent {
    //TODO: in \n separated TCP stream the ctrl chars are be escaped using # + octal encoding by
    //popular log agents
    fn decode_newlines(self) -> SyslogEvent {
        if let Some(Message::String(s)) = self.message {
            SyslogEvent { message: Some(Message::String(s.replace("#012", "\n"))), .. self }
        } else {
            self
        }
    }
}

/*
struct FieldsIterator<'f> {
    fields: [(&'static name, fn(&SyslogEvent) -> FieldValue<'f>)]
}

impl<'f> Iterator for FieldsIterator<'f> {
    type Item = FieldValue<'f>;

     fn next(&mut self) -> Option<Self::Item> {
         if let Some((ref (name, value), rest)) = fields.split_first() {
             self.fields = rest;
             Some((name, value))
         } else {
             None
         }
     }
}
*/

impl Event for SyslogEvent {
    fn id(&self) -> Cow<str> {
        if let Some(ref msg_id) = self.msg_id {
            Cow::Borrowed(msg_id)
        } else {
            Cow::Owned(Uuid::new_v4().simple().to_string())
        }
    }

    fn source(&self) -> Cow<str> {
        Cow::Borrowed(&self.hostname)
    }

    fn timestamp(&self) -> DateTime<UTC> {
        self.timestamp.with_timezone(&UTC)
    }

    fn payload(&self) -> Option<Payload> {
        match self.message {
            Some(Message::String(ref s)) => Some(Payload::String(Cow::Borrowed(s))),
            Some(Message::MaybeString(ref ms)) => Some(Payload::Data(Cow::Borrowed(ms))),
            None => None
        }
    }
}

impl LogstashEvent for SyslogEvent {
    fn timestamp(&self) -> DateTime<UTC> {
        self.timestamp.with_timezone(&UTC)
    }

    fn message(&self) -> Option<Cow<str>> {
        match self.message {
            Some(Message::String(ref s)) => Some(Cow::Borrowed(s)),
            Some(Message::MaybeString(ref ms)) => Some(Cow::Owned(ms.as_maybe_str().to_lossy_string())),
            None => None
        }
    }

    fn event_type(&self) -> &str {
        "syslog"
    }

    fn tags(&self) -> Vec<&'static str> {
        vec!["class:syslog"]
    }

    fn processed(&self) -> DateTime<UTC> {
        self.processed
    }

    fn id(&self) -> Cow<str> {
        if let Some(ref msg_id) = self.msg_id {
            Cow::Borrowed(msg_id)
        } else {
            Cow::Owned(Uuid::new_v4().simple().to_string())
        }
    }

    fn fields<F: FieldSerializer>(&self, serializer: &mut F) -> Result<(), F::Error> {
        serializer.serialize_field_str("logsource", &self.hostname)?;

        let severity = match self.severity {
            Severity::Emergency => "Emergency",
            Severity::Alert => "Alert",
            Severity::Critical => "Critical",
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Notice => "Notice",
            Severity::Informational => "Informational",
            Severity::Debug => "Debug",
        };
        serializer.serialize_field_str("severity", severity)?;

        let facility = match self.facility {
            Facility::KernelMessages => "kernel",
            Facility::UserLevelMessages => "user-level",
            Facility::MailSystem => "mail",
            Facility::SystemDaemons => "system",
            Facility::SecurityMessages => "security/authorization",
            Facility::Internal => "syslogd",
            Facility::LinePrinterSubsystem => "line printer",
            Facility::NetworkNewsSubsystem => "network news",
            Facility::UucpSubsystem => "UUCP",
            Facility::ClockDaemon => "clock",
            Facility::AuthPrivMessage => "security/authorization",
            Facility::FtpDaemon => "FTP",
            Facility::NtpSubsystem => "NTP",
            Facility::LogAudit => "log audit",
            Facility::LogAlert => "log alert",
            Facility::SchedulingDaemon => "clock",
            Facility::Local0 => "local0",
            Facility::Local1 => "local1",
            Facility::Local2 => "local2",
            Facility::Local3 => "local3",
            Facility::Local4 => "local4",
            Facility::Local5 => "local5",
            Facility::Local6 => "local6",
            Facility::Local7 => "local7",
        };
        serializer.serialize_field_str("facility", facility)?;

        if let Some(ref program) = self.program {
            serializer.serialize_field_str("program", &program)?;
        }

        if let Some(ref pid) = self.proc_id {
            serializer.serialize_field_str("pid", &pid)?;
        }
        Ok(())
    }
}

named!(priority<&[u8], u8>, return_error!(ErrorKind::Custom(1),
    complete!(delimited!(tag!(b"<"), map_res!(take_until!(">1"), parse::int_u8), tag!(b">")))));

named!(timestamp<&[u8], DateTime<FixedOffset> >, return_error!(ErrorKind::Custom(2),
    complete!(terminated!(map_res!(take_until!(" "), parse::timestamp), tag!(b" ")))));

named!(hostname<&[u8], &str>, return_error!(ErrorKind::Custom(3),
    complete!(do_parse!(
        not!(tag!("- ")) >>
        h: terminated!(map_res!(take_until!(" "), parse::string), tag!(b" ")) >>
        (h)
    ))));

named!(opt_str<&[u8], Option<&str> >, complete!(map!(
        terminated!(map_res!(take_until!(" "), parse::string), tag!(b" ")),
        |s| if s == "-" { None } else { Some(s) }
    )));

named!(program<&[u8], Option<&str> >, return_error!(ErrorKind::Custom(4), opt_str));
named!(proc_id<&[u8], Option<&str> >, return_error!(ErrorKind::Custom(5), opt_str));
named!(msg_id<&[u8], Option<&str> >, return_error!(ErrorKind::Custom(6), opt_str));

named!(structured_data_param<&[u8], (&str, String)>, return_error!(ErrorKind::Custom(12), complete!(do_parse!(
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
    ))));

named!(structured_data_element<&[u8], StructuredElement>, return_error!(ErrorKind::Custom(11), complete!(do_parse!(
        tag!(b"[") >>
        id: map_res!(take_until!(" "), parse::string) >>
        tag!(b" ") >>
        params: separated_list!(tag!(b" "), call!(structured_data_param)) >>
        tag!(b"]") >>
        (StructuredElement {
            id: id.to_owned(),
            params: params.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
        })
    ))));

named!(sp_or_eof<&[u8], &[u8]>, alt!(eof!() | tag!(b" ")));

named!(structured_data<&[u8], Option<StructuredData> >, do_parse!(
        minus: opt!(tag!(b"-")) >>
        res: cond_with_error!(minus.is_none(), many_till!(
            call!(structured_data_element),
            peek!(call!(sp_or_eof))
        )) >>
        (res.map(|r| StructuredData { elements: r.0 }))
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

named!(pub syslog_rfc5424<&[u8], SyslogEvent>, complete!(do_parse!(
    facility: map_res!(peek!(priority), |p| Facility::from_priority(p)) >>
    severity: map!(priority, |p| Severity::from_priority(p)) >>
    tag!(b"1 ") >> // Fromat version 1
    timestamp: timestamp >>
    hostname: hostname >>
    program: program >>
    proc_id: proc_id >>
    msg_id: msg_id >>
    structured_data: structured_data >>
    message: message >>
    (SyslogEvent {
        facility: facility,
        severity: severity,
        timestamp: timestamp,
        hostname: hostname.to_owned(),
        program: program.map(|s| s.to_owned()),
        proc_id: proc_id.map(|s| s.to_owned()),
        msg_id: msg_id.map(|s| s.to_owned()),
        structured_data: structured_data,
        message: message,
        processed: UTC::now(),
    }))));

named!(pub syslog_rfc5425_frame<&[u8], &[u8]>, do_parse!(
        msg_len: return_error!(ErrorKind::Custom(1),
           terminated!(map_res!(take_until!(" "), parse::int_u8), tag!(" "))) >>
        syslog_msg: take!(msg_len) >>
        (syslog_msg)
    ));

named!(pub syslog_rfc5424_in_rfc5425_frame<&[u8], SyslogEvent>,
       flat_map!(call!(syslog_rfc5425_frame), call!(syslog_rfc5424)));

// framing not allowing to use \n in messages - use #012 to represent \n and replace in final
// message
named!(pub syslog_newline_frame<&[u8], &[u8]>, terminated!(take_until!("\n"), tag!(b"\n")));

named!(pub syslog_rfc5424_in_newline_frame<&[u8], SyslogEvent>, map!(
       flat_map!(call!(syslog_newline_frame), call!(syslog_rfc5424)),
       |m: SyslogEvent| m.decode_newlines()));

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

    pub fn syslog_newline_frame(input: &[u8]) -> IResult<&[u8], &[u8], &'static str> {
        super::syslog_newline_frame(input).map_err(|err| ErrorKind::Custom(match err {
            _ => "Syslog new line frame parser did not match"
        }))
    }

    named!(pub syslog_rfc5424_in_newline_frame<&[u8], SyslogEvent, &'static str>, map!(
           flat_map!(call!(syslog_newline_frame), call!(syslog_rfc5424)),
           |m: SyslogEvent| m.decode_newlines()));
}

pub fn tcp_syslog_input(handle: Handle, addr: &SocketAddr) -> mpsc::Receiver<SyslogEvent> {
    //tcp_nom_input("syslog", handle, addr, simple_errors::syslog_rfc5424_in_rfc5425_frame)
    tcp_nom_input("syslog", handle, addr, simple_errors::syslog_rfc5424_in_newline_frame)
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
mod syslog_newline_frame_tests {
    use super::simple_errors::syslog_newline_frame;

    #[test]
    fn framing() {
        let (i, o) = syslog_newline_frame(b"foo\nbar\nbaz\n").unwrap();
        assert_eq!(i, &b"bar\nbaz\n"[..]);
        assert_eq!(o, &b"foo"[..]);

        let (i, o) = syslog_newline_frame(i).unwrap();
        assert_eq!(i, &b"baz\n"[..]);
        assert_eq!(o, &b"bar"[..]);

        let (i, o) = syslog_newline_frame(i).unwrap();
        assert!(i.is_empty());
        assert_eq!(o, &b"baz"[..]);
    }

    #[test]
    fn incomplete() {
        use nom::Needed;
        let needed = syslog_newline_frame(b"fo").unwrap_inc();
        assert_eq!(needed, Needed::Unknown);
    }
}

#[cfg(test)]
mod syslog_rfc5424_tests {
    pub use super::{Message, Facility, Severity};
    pub use maybe_string::MaybeString;
    pub use nom::ErrorKind;
    use super::simple_errors::syslog_rfc5424;

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
        use chrono::DateTime;
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.timestamp, DateTime::parse_from_rfc3339("2003-10-11T22:14:15.003Z").unwrap());
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
    fn program() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.program, Some("evntslog".to_owned()));
    }

    #[test]
    fn app_name_none() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com - - ID47 - foobar\n").unwrap();
        assert!(i.is_empty());
        assert_eq!(o.program, None);
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

        assert_eq!(o.structured_data.as_ref().unwrap().elements.len(), 1);

        let e = o.structured_data.as_ref().unwrap().elements.get(0).unwrap();
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
        assert_eq!(o.structured_data.as_ref().unwrap().elements.len(), 2);

        let e = o.structured_data.as_ref().unwrap().elements.get(0).unwrap();
        assert_eq!(e.id, "exampleSDID@32473");
        assert_eq!(e.params.len(), 3);
        assert_eq!(e.params["iut"], "3".to_owned());
        assert_eq!(e.params["eventSource"], "Application".to_owned());
        assert_eq!(e.params["eventID"], "1011".to_owned());

        let e = o.structured_data.as_ref().unwrap().elements.get(1).unwrap();
        assert_eq!(e.id, "examplePriority@32473");
        assert_eq!(e.params.len(), 1);
        assert_eq!(e.params["class"], "high".to_owned());

        assert_eq!(o.message, Some(Message::String("foo\nbar".to_owned())));
    }

    #[test]
    fn structured_data_multi_escapes() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Appli\\\\catio\\]n\" eventID=\"1011\"][examplePriority@32473 class=\"hi\\\"gh\"] \xEF\xBB\xBFfoo\nbar").unwrap();

        assert!(i.is_empty());
        assert_eq!(o.structured_data.as_ref().unwrap().elements.len(), 2);

        let e = o.structured_data.as_ref().unwrap().elements.get(0).unwrap();
        assert_eq!(e.id, "exampleSDID@32473");
        assert_eq!(e.params.len(), 3);
        assert_eq!(e.params["iut"], "3".to_owned());
        assert_eq!(e.params["eventSource"], "Appli\\catio]n".to_owned());
        assert_eq!(e.params["eventID"], "1011".to_owned());

        let e = o.structured_data.as_ref().unwrap().elements.get(1).unwrap();
        assert_eq!(e.id, "examplePriority@32473");
        assert_eq!(e.params.len(), 1);
        assert_eq!(e.params["class"], "hi\"gh".to_owned());

        assert_eq!(o.message, Some(Message::String("foo\nbar".to_owned())));
    }

    #[test]
    fn structured_data_multi_no_message() {
        let (i, o) = syslog_rfc5424(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut=\"3\" eventSource=\"Application\" eventID=\"1011\"][examplePriority@32473 class=\"high\"]").unwrap();
        assert!(i.is_empty());

        assert_eq!(o.structured_data.as_ref().unwrap().elements.len(), 2);

        let e = o.structured_data.as_ref().unwrap().elements.get(0).unwrap();
        assert_eq!(e.id, "exampleSDID@32473");
        assert_eq!(e.params.len(), 3);
        assert_eq!(e.params["iut"], "3".to_owned());
        assert_eq!(e.params["eventSource"], "Application".to_owned());
        assert_eq!(e.params["eventID"], "1011".to_owned());

        let e = o.structured_data.as_ref().unwrap().elements.get(1).unwrap();
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

    #[cfg(test)]
    mod in_syslog_newline_frame_tests {
        use super::*;
        use super::super::simple_errors::syslog_rfc5424_in_newline_frame;

        #[test]
        fn framing() {
            let (i, o) = syslog_rfc5424_in_newline_frame(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - \xEF\xBB\xBFfoo\n<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo\n").unwrap();
            assert_eq!(o.message, Some(Message::String("foo".to_owned())));

            let (i, o) = syslog_rfc5424_in_newline_frame(i).unwrap();
            assert!(i.is_empty());
            assert_eq!(o.message, Some(Message::MaybeString(MaybeString::from_bytes(b"foo".as_ref().to_owned()))));
        }

        #[test]
        fn incomplete() {
            use nom::Needed;
            let needed = syslog_rfc5424_in_newline_frame(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - \xEF\xBB\xBFfoo").unwrap_inc();
            assert_eq!(needed, Needed::Unknown);
        }

        #[test]
        fn octal_newline_excapes_when_message_is_string() {
            let (i, o) = syslog_rfc5424_in_newline_frame(b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - \xEF\xBB\xBFfoo#012bar\n<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - foo#012bar\n").unwrap();
            assert_eq!(o.message, Some(Message::String("foo\nbar".to_owned())));

            let (i, o) = syslog_rfc5424_in_newline_frame(i).unwrap();
            assert!(i.is_empty());
            assert_eq!(o.message, Some(Message::MaybeString(MaybeString::from_bytes(b"foo#012bar".as_ref().to_owned()))));
        }
    }
}
