use std::net::SocketAddr;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use std::fmt::{self, Debug, Display};
use std::error::Error;
use std::os::unix::io::AsRawFd;

use slog::Logger;

use futures::Future;
use futures::stream::Stream;
use futures::sync::mpsc;
use futures::Sink;
use futures::future;

use tokio_core::io::{Io, Codec};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Handle;

use PipeError;

#[derive(Debug)]
enum TcpInputError<T: Debug> {
    SendError(T),
    IoError(IoError)
}

impl<T: Debug> From<mpsc::SendError<T>> for TcpInputError<T> {
    fn from(send_error: mpsc::SendError<T>) -> TcpInputError<T> {
        TcpInputError::SendError(send_error.into_inner())
    }
}

impl<T: Debug> From<IoError> for TcpInputError<T> {
    fn from(io_error: IoError) -> TcpInputError<T> {
        TcpInputError::IoError(io_error)
    }
}

impl<T: Debug> Display for TcpInputError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TcpInputError::SendError(ref t) => write!(f, "{}: {:?}", self.description(), t),
            TcpInputError::IoError(ref io) => match io.get_ref() {
                Some(reason) => write!(f, "{}: {}", self.description(), reason),
                None => write!(f, "{}", self.description())
            }
        }
    }
}

impl<T: Debug> Error for TcpInputError<T> {
    fn description(&self) -> &str {
        match *self { TcpInputError::SendError(_) => "Failed to push processed input message down the pipeline", TcpInputError::IoError(ref io) => match io.kind() {
                IoErrorKind::InvalidInput => "Failed to parse input",
                IoErrorKind::InvalidData => "Failed to apply parser",
                _ => "Input error"
            }
        }
    }
}

pub fn tcp_input<C, T, OE>(logger: &Logger, name: &'static str, handle: Handle, addr: &SocketAddr, codec: C) -> Box<Stream<Item=T, Error=PipeError<(), OE>>> where C: Codec<In=T, Out=()> + Clone + 'static, T: Debug + 'static {
    let logger = logger.new(o!("input" => name));
    let (sender, receiver) = mpsc::channel(10);
    let listener_handle = handle.clone();

    let listener = TcpListener::bind(addr, &handle).expect("bound TCP socket");
    info!(&logger, "Listening for TCP connections"; "bound" => format!("{}", addr));

    let incoming_logger = logger.clone();
    listener_handle.spawn(
        listener
        .incoming()
        .for_each(move |(tcp_stream, remote_addr)| {
            let id = tcp_stream.as_raw_fd();
            let conn_logger = incoming_logger.new(o!("connection" => id, "remote" => format!("{}", remote_addr)));
            info!(&conn_logger, "Accepted TCP connection");

            let conn_err_logger = conn_logger.clone();
            let connection = sender.clone()
                .with(|message| {
                    future::ok::<T, TcpInputError<T>>(message)
                })
                .send_all(tcp_stream.framed(codec.clone()))
                .map_err(move |err| {
                    error!(&conn_err_logger, "Error while decoding input: {:?}", err);
                    ()})
                .map(move |(_sink, _stream)| {
                    info!(&conn_logger, "TCP connection closed by remote");
                    ()});

            handle.spawn(connection);
            Ok(())
        })
        .map_err(move |err| {
            error!(&logger, "Error processing incomming TCP connectionsi: {:?}", err);
            ()}));

    //TODO: provide error stream
    Box::new(receiver.map_err(|_| PipeError::Input(())))
}
