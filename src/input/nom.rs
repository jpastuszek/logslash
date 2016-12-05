use std::net::SocketAddr;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use std::fmt::Debug;

use futures::Future;
use futures::stream::Stream;
use futures::sync::mpsc;
use futures::Sink;
use futures::future;

use tokio_core::io::{Io, Codec, EasyBuf};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Handle;

use nom::IResult;

pub type NomParser<T> = fn(&[u8]) -> IResult<&[u8], T, &'static str>;

struct NomCodec<T> {
    parser: NomParser<T>
}

impl<T> NomCodec<T> {
    fn new(parser: NomParser<T>) -> NomCodec<T> {
        NomCodec {
            parser: parser
        }
    }
}

impl<T> Codec for NomCodec<T> {
    type In = T;
    type Out = ();

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, IoError> {
        let have_bytes = buf.len();

        let mut consumed = 0;
        let result = match (self.parser)(buf.as_slice()) {
            IResult::Done(input_left, output) => {
                consumed = have_bytes - input_left.len();
                Ok(Some(output))
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

    fn encode(&mut self, _msg: Self::Out, _buf: &mut Vec<u8>) -> Result<(), IoError> {
        panic!("SyslogCodec: encode unimplemented!")
    }
}


#[derive(Debug)]
enum NomInputError<T> {
    SendError(T),
    IoError(IoError)
}

impl<T> From<mpsc::SendError<T>> for NomInputError<T> {
    fn from(send_error: mpsc::SendError<T>) -> NomInputError<T> {
        NomInputError::SendError(send_error.into_inner())
    }
}

impl<T> From<IoError> for NomInputError<T> {
    fn from(io_error: IoError) -> NomInputError<T> {
        NomInputError::IoError(io_error)
    }
}

pub fn tcp_nom_input<T>(name: &'static str, handle: Handle, addr: &SocketAddr, parser: NomParser<T>) -> mpsc::Receiver<T> where T: Debug + 'static {
    let (sender, receiver) = mpsc::channel(10);
    let listener_handle = handle.clone();

    let listener = TcpListener::bind(addr, &handle).expect("bound TCP socket")
        .incoming()
        .for_each(move |(tcp_stream, remote_addr)| {
            println!("connection from: {}", remote_addr);
            let connection = sender.clone()
                .with(|message| {
                    future::ok::<T, NomInputError<T>>(message)
                })
                .send_all(tcp_stream.framed(NomCodec::new(parser)))
                .map_err(move |err| {
                    println!("error while decoding input {}: {:?}", name, err);
                    ()})
                .map(move |(_sink, _stream)| {
                    println!("connection closed by remote for input {}", name);
                    ()});
            handle.spawn(connection);
            Ok(())
        })
        .map_err(move |err| {
            println!("error processing incomming connections for input {}: {:?}", name, err);
            ()});

    listener_handle.spawn(listener);

    receiver
}
