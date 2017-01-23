use std::fmt::{self, Display, Debug};
use std::error::Error;
use std::borrow::Cow;
use std::io::stdout;
use std::io::Write;
use std::cell::RefCell;
use std::rc::Rc;
use std::mem::replace;
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

    let buf_cell = Rc::new(RefCell::new(Some(Vec::with_capacity(64))));
    let buf_cell_taker = buf_cell.clone();
    let buf_cell_putter = buf_cell.clone();

    let pipe = receiver
        // populate the buffer with message
        .map(move |event| {
            let mut buf = buf_cell_taker.borrow_mut().take().expect("taken");
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
        // clean the buffer and put it back for reuse
        .map(move |(_stdout, mut buf)| {
            buf.clear();
            replace(&mut *(buf_cell_putter.borrow_mut()), Some(buf));
            ()
        })
        .for_each(|_| Ok(()));

    handle.spawn(pipe);

    Box::new(sender.with(|message| {
        ok::<T, PipeError<IE, ()>>(message)
    }))
}
