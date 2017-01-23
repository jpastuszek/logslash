use std::fmt::{self, Display, Debug};
use std::error::Error;
use std::borrow::Cow;
use std::io::stdout;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::io::Write;
use futures::{Future, Stream, Sink};
use futures::future::ok;
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio_core::io::write_all;
use tokio_core::reactor::Handle;
use chrono::{DateTime, UTC};
use PipeError;

pub trait DebugPort {
    type SerializeError: Error;
    fn id(&self) -> Cow<str>;
    fn timestamp(&self) -> DateTime<UTC>;
    fn source(&self) -> Cow<str>;
    fn write_payload<W: Write>(&self, out: W) -> Result<W, Self::SerializeError>;
}

#[derive(Debug)]
pub enum DebugOuputError<SE> {
    Serialization(SE),
}

impl<SE: Debug + Display> Display for DebugOuputError<SE> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DebugOuputError::Serialization(ref error) => write!(f, "{}: {}", self.description(), error),
        }
    }
}

impl<SE: Debug + Display> Error for DebugOuputError<SE> {
    fn description(&self) -> &str {
        match *self {
            DebugOuputError::Serialization(_) => "Failed to serialise event",
        }
    }
}

pub fn print_event<T, IE>(handle: Handle) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + Debug + 'static, IE: 'static {
    let (sender, receiver): (Sender<T>, Receiver<T>) = channel(100);

    // TOOD: how do I capture state for whole future
    //let stdout = io::stdout();
    //let mut handle = stdout.lock();

    let (buf_sender, buf_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(1);

    let init_buf_sender = buf_sender.clone();

    let pipe = receiver
        .zip(buf_receiver)
        .map(|(event, mut buf)| {
            // Note: We need allocation per message here as it needs to be alive until it is all
            // written out
            //let mut buf = Vec::with_capacity(64);

            write!(buf, "{} {} [{}] -- ",  event.id().as_ref(), event.source().as_ref(), event.timestamp()).expect("header written to buf");

            event.write_payload(buf)
                .map_err(|error| DebugOuputError::Serialization(error))
                .map(|mut buf| {
                    buf.push(b'\n');
                    buf
                })
        })
        .filter_map(|ser_result|
            match ser_result {
                Ok(ok) => Some(ok),
                Err(err) => {
                    println!("Failed to prepare event for debug output: {}", err);
                    None
                }
            }
        )
        .and_then(move |body| {
             let buf_sender = buf_sender.clone();
             write_all(stdout(), body)
            .and_then(move |(_stdout, mut buf)| {
                // clear the buffer and reuse
                buf.clear();
                buf_sender
                    .send(buf)
                    .map_err(|_| IoError::new(ErrorKind::BrokenPipe, "failed to send buf back for reuse"))
            })
            .map_err(|err| println!("Failed to write debug ouptu: {}", err))
        });

    handle.spawn(pipe.for_each(|_| Ok(())));

    handle.spawn(init_buf_sender
                 .send(Vec::with_capacity(64))
                 .map_err(|_| panic!("can't send initial buf"))
                 .map(|_| ()));

    Box::new(sender.with(|message| {
        ok::<T, PipeError<IE, ()>>(message)
    }))
}
