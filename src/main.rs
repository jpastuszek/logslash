extern crate futures;
extern crate tokio_core;

use std::env;
use std::net::SocketAddr;
use std::io::Read;
use std::io::Error as IoError;

use futures::Future;
use futures::stream::Stream;
use futures::future::BoxFuture;
use tokio_core::io::{self, Io, Codec};
use tokio_core::io::EasyBuf;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle};

struct SyslogInput {
    socket: TcpListener
}

struct SyslogCodec {
}

impl Codec for SyslogCodec {
    type In = String; // for now
    type Out = ();

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, IoError> {
        let have_bytes = buf.len();
        Ok(Some(String::from_utf8_lossy(buf.drain_to(have_bytes).as_slice()).into_owned()))
    }
    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> Result<(), IoError> {
        panic!("SyslogCodec: encode unimplemented!")
    }
}



impl SyslogInput {
    fn new(handle: &Handle, addr: &SocketAddr) -> SyslogInput {
        SyslogInput {
            socket: TcpListener::bind(addr, handle).unwrap()
        }
    }

    fn events(self, handle: Handle) -> Box<Stream<Item=String, Error=IoError> + 'static> {
        Box::new(self.socket.incoming().map(move |(stream, addr)| {
            println!("connection from: {}", addr);
            stream.framed(SyslogCodec{})
        }).flatten())
    }

    fn messages(self, handle: Handle) -> Box<futures::Future<Item=(), Error=IoError> + 'static> {
        Box::new(self.socket.incoming().for_each(move |(socket, addr)| {
            println!("connection from: {}", addr);
            let (mut reader, writer) = socket.split();
            drop(writer);

            let messages = io::read_to_end(reader, Vec::new()).then(|result| {
                let (_reader, messages) = result.expect("messages read");
                println!("got messages: {:?}", messages);
                Ok(())
            });

            handle.spawn(messages);

            Ok(())
        }))
    }
}

fn main() {
    println!("Hello, world!");

    let mut event_loop = Core::new().unwrap();
    let handle = event_loop.handle();

    let syslog = SyslogInput::new(&handle, &"127.0.0.1:5514".parse().unwrap());

    let input = syslog.events(handle.clone()).for_each(|event| {
        println!("got event: {}", event);
        Ok(())
    });

    event_loop.run(input);
}
