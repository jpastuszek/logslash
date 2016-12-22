use std::fmt::{self, Display, Debug};
use std::error::Error;
use std::borrow::Cow;
use std::io::stdout;
use futures::{Future, Stream, Sink};
use futures::future::ok;
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio_core::io::write_all;
use tokio_core::reactor::Handle;
use chrono::{DateTime, UTC};
use serialize::Serialize;
use PipeError;

pub trait DebugPort {
    fn id(&self) -> Cow<str>;
    fn timestamp(&self) -> DateTime<UTC>;
    fn source(&self) -> Cow<str>;
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

pub fn print_serde_json<S, T, IE, SE>(handle: Handle, serializer: S) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: DebugPort + Debug + 'static, S: Serialize<T, Error=SE> + 'static, SE: Error + 'static, IE: 'static {
    let (sender, receiver): (Sender<T>, Receiver<T>) = channel(100);

    // TOOD: how do I capture state for whole future
    //let stdout = io::stdout();
    //let mut handle = stdout.lock();

    let pipe = receiver
        .map(move |event| match serializer.serialize(&event) {
            Ok(body) => {
                let header = format!("{} {}[{}] -- ",  event.id().as_ref(), event.timestamp(), event.source().as_ref());
                Ok((header, body))
            },
            Err(error) => Err(DebugOuputError::Serialization(error))
        })
        //TODO: provide error stream
        .filter_map(|ser_result| match ser_result {
            Ok(ok) => Some(ok),
            Err(err) => {
                println!("Failed to prepare event for debug output: {}", err);
                None
            }
        })
        .and_then(move |(header, body)| write_all(stdout(), header)
            .and_then(|(stdout, _buf)| write_all(stdout, body))
            .map(|(_stdout, _buf)| ())
            .map_err(|err| println!("Failed to write debug ouptu: {}", err))
        );

    handle.spawn(pipe.for_each(|_| Ok(())));

    Box::new(sender.with(|message| {
        ok::<T, PipeError<IE, ()>>(message)
    }))
}
