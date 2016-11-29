extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate nom;

use std::env;
use std::net::SocketAddr;
use std::io::Read;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;

use futures::Future;
use futures::stream::Stream;
use futures::future::BoxFuture;
use futures::sync::mpsc;
use futures::Sink;
use futures::future;

use tokio_core::io::{self, Io, Codec};
use tokio_core::io::EasyBuf;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle};

use nom::IResult;
use nom::FindSubstring;

// "(?m)<%{POSINT:priority}>(?:%{SYSLOGTIMESTAMP:timestamp}|%{TIMESTAMP_ISO8601:timestamp8601}) (?:%{SYSLOGFACILITY} )?(:?%{SYSLOGHOST:logsource} )?(?<program>[^ \[]+)(?:\[%{POSINT:pid}\])?: %{GREEDYDATA:message}"
named!(syslog_parser, terminated!(take_until!(&b" "[..]), tag!(b" ")));

struct SyslogInput {
    //listener: TcpListener,
    messages: mpsc::Receiver<String>
}

struct SyslogCodec {
}

#[derive(Debug)]
enum SyslogError {
    SendError(String),
    IoError(IoError)
}

impl From<futures::sync::mpsc::SendError<std::string::String>> for SyslogError {
    fn from(send_error: futures::sync::mpsc::SendError<std::string::String>) -> SyslogError {
        SyslogError::SendError(send_error.into_inner())
    }
}

impl From<IoError> for SyslogError {
    fn from(io_error: IoError) -> SyslogError {
        SyslogError::IoError(io_error)
    }
}

impl Codec for SyslogCodec {
    type In = String; // for now
    type Out = ();

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, IoError> {
        let have_bytes = buf.len();
        //Ok(Some(String::from_utf8_lossy(buf.drain_to(have_bytes).as_slice()).into_owned()))

        //let result: nom::IResult<_, Result<T, _>, u32> = opt_res!(buf.as_slice(), complete!(syslog_parser));
        // unwrap here is safe as complete! eliminates Incomplete variant and opt_res! remaining Error variant
        //result.unwrap().1.expect("parsed syslog message")

        let mut consumed = 0;
        let result = match syslog_parser(buf.as_slice()) {
            IResult::Done(input_left, output) => {
                println!("parsed out: {:?}", output);
                // consume data
                consumed = have_bytes - input_left.len();
                Ok(Some(String::from_utf8_lossy(output).into_owned()))
            }
            IResult::Error(err) => {
                println!("err: {}", err);
                Err(IoError::new(IoErrorKind::InvalidInput, err))
            }
            IResult::Incomplete(_) => {
                println!("incomplete");
                Ok(None)
            }
        };

        if consumed > 0 {
            buf.drain_to(consumed);
        }
        result
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> Result<(), IoError> {
        panic!("SyslogCodec: encode unimplemented!")
    }
}

impl SyslogInput {
    fn new(handle: Handle, addr: &SocketAddr) -> SyslogInput {
        let (sender, receiver) = mpsc::channel(10);
        let listener_handle = handle.clone();

        let listener = TcpListener::bind(addr, &handle).expect("bound TCP socket")
            .incoming()
            .for_each(move |(stream, addr)| {
                println!("connection from: {}", addr);
                let connection = sender.clone()
                    .with(|message| {
                        future::ok::<String, SyslogError>(message)
                    })
                .send_all(stream.framed(SyslogCodec{}))
                    .map_err(|err| {
                        println!("error while decoding syslog: {:?}", err);
                        ()})
                    .map(|(_sink, _stream)| {
                        println!("messages sent");
                        ()});
                handle.spawn(connection);
                Ok(())
            })
            .map_err(|err| {
                println!("error processing incomming connections: {:?}", err);
                ()});

        listener_handle.spawn(listener);

        SyslogInput {
            messages: receiver
        }
    }

    fn messages(self) -> mpsc::Receiver<String> {
        self.messages
    }
}

fn main() {
    println!("Hello, world!");

    let mut event_loop = Core::new().unwrap();
    let handle = event_loop.handle();

    let syslog = SyslogInput::new(handle, &"127.0.0.1:5514".parse().unwrap());

    let input = syslog.messages().for_each(|event| {
        println!("got event: {}", event);
        Ok(())
    });

    event_loop.run(input);
}
