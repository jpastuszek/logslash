use std::fmt::{self, Display, Debug};
use std::error::Error;
use std::borrow::Cow;
use std::io::Error as IoError;
use std::io::stdout;
use futures::{IntoFuture, Future, Stream};
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio_core::io::write_all;
use tokio_core::reactor::Handle;
use chrono::{DateTime, UTC};
use serialize::Serialize;

pub trait DebugPort {
    fn id(&self) -> Cow<str>;
    fn timestamp(&self) -> DateTime<UTC>;
    fn source(&self) -> Cow<str>;
}

#[derive(Debug)]
pub enum DebugOuputError<IE, SE> {
    //TODO: chain errors?
    Input(IE),
    Serialization(SE),
    Io(IoError),
}

/*
impl<T: Debug> From<mpsc::SendError<T>> for NomInputError<T> {
    fn from(send_error: mpsc::SendError<T>) -> NomInputError<T> {
        NomInputError::SendError(send_error.into_inner())
    }
}
*/

impl<IE, SE> From<IoError> for DebugOuputError<IE, SE> {
    fn from(error: IoError) -> DebugOuputError<IE, SE> {
        DebugOuputError::Io(error)
    }
}

impl<IE: Debug + Display, SE: Debug + Display> Display for DebugOuputError<IE, SE> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DebugOuputError::Input(ref error) => write!(f, "{}: {}", self.description(), error),
            DebugOuputError::Io(ref error) => write!(f, "{}: {}", self.description(), error),
            DebugOuputError::Serialization(ref error) => write!(f, "{}: {}", self.description(), error),
        }
    }
}

impl<IE: Debug + Display, SE: Debug + Display> Error for DebugOuputError<IE, SE> {
    fn description(&self) -> &str {
        match *self {
            DebugOuputError::Input(_) => "Source of events failed",
            DebugOuputError::Io(_) => "I/O error while writing debug output",
            DebugOuputError::Serialization(_) => "Failed to serialise event",
        }
    }
}

pub fn print_serde_json<S, T, IE, SE>(handle: Handle, serializer: S) -> Sender<Result<T, IE>> where T: DebugPort + Debug + 'static, S: Serialize<T, Error=SE> + 'static, SE: Error + 'static, IE: Display + Debug + 'static {
    let (sender, receiver): (Sender<Result<T, IE>>, Receiver<Result<T, IE>>) = channel(100);

    // TOOD: how do I capture state for whole future
    //let stdout = io::stdout();
    //let mut handle = stdout.lock();

    let pipe = receiver
        .map(move |event|
             event
            .map_err(|err| DebugOuputError::Input(err))
            .and_then(|event| match serializer.serialize(&event) {
                Ok(body) => {
                    let header = format!("{} {}[{}] -- ",  event.id().as_ref(), event.timestamp(), event.source().as_ref());
                    Ok((header, body))
                },
                Err(error) => Err(DebugOuputError::Serialization(error))
            })
        )
        .and_then(move |result|
            result
            .into_future()
            .and_then(|(header, body)| {
                 write_all(stdout(), header)
                .and_then(|(stdout, _buf)| write_all(stdout, body))
                .map(|(_stdout, _buf)| ())
                .map_err(|err| DebugOuputError::Io(err))
            }).map_err(|err| {
                println!("Failed to write debug output: {}", err);
                ()
            })
        );

    handle.spawn(pipe.for_each(|_| Ok(())));

    /*
    sender.with(|message| {
        ok::<T, DebugOuputError<T>>(message)
    })
    */
    sender
}
