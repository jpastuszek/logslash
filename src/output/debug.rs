use std::fmt::{self, Display, Debug};
use std::error::Error;
use std::borrow::Cow;
use std::io::stdout;
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

    // This channel will receive the buffer used to collect the message bytes after it was written
    // out so it can be reused for the next event
    let (buf_sender, buf_receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(1);

    // We need to send the inital buffer here
    let init_buf_sender = buf_sender.clone();

    let pipe = receiver
        // rendezvous with the buffer
        .zip(buf_receiver)
        // populate the buffer with message
        .map(|(event, mut buf)| {
            write!(buf, "{} {} [{}] -- ",  event.id().as_ref(), event.source().as_ref(), event.timestamp()).expect("header written to buf");

            event.write_payload(buf)
                .map_err(|error| DebugOuputError::Serialization(error))
                .map(|mut buf| {
                    buf.push(b'\n');
                    buf
                })
        })
        // if something when wrong log and drop the message
        .filter_map(|ser_result|
            match ser_result {
                Ok(ok) => Some(ok),
                Err(err) => {
                    println!("Failed to prepare event for debug output: {}", err);
                    None
                }
            }
        )
        // write message to stdout and send back the buffer for reuse
        .and_then(|body|
             write_all(stdout(), body).map_err(|err| println!("Failed to print event to stdout: {}", err))
         )
        // cleanup
        .and_then(move |(stdout, mut buf)| {
            // unlock stdout
            drop(stdout);

            // clear the buffer and reuse
            buf.clear();
            buf_sender.clone()
                .send(buf)
                .map_err(|_| panic!("Failed to send buffer back for reuse"))
        });

    handle.spawn(pipe.for_each(|_| Ok(())));

    // Send the inital buffer
    handle.spawn(init_buf_sender
                 .send(Vec::with_capacity(64))
                 .map_err(|_| panic!("can't send initial buffer"))
                 .map(|_| ()));

    Box::new(sender.with(|message| {
        ok::<T, PipeError<IE, ()>>(message)
    }))
}
