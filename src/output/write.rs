use std::fmt::{Display, Debug};
use std::io::{Write, BufWriter};
use std::cell::RefCell;
use std::rc::Rc;
use std::mem::replace;
use std::thread;
use futures::{Future, Stream, Sink};
use futures::future::ok;
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio_core::io::write_all;
use tokio_core::reactor::Handle;
use PipeError;

pub fn write_threaded<T, W, IE, SE, F>(out: W, serialize: F) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: Send + 'static, W: Send + 'static, IE: 'static, SE: Debug + Display + 'static, W: Write, F: Fn(&T, &mut Vec<u8>) -> Result<(), SE> + Send + 'static {
    let (sender, receiver): (Sender<T>, Receiver<T>) = channel(100);

    thread::Builder::new().name("write_threaded".into()).spawn(|| {
        let buf_cell = Rc::new(RefCell::new(Some(Vec::with_capacity(64))));
        let buf_cell_taker = buf_cell.clone();
        let buf_cell_putter = buf_cell.clone();

        let out_cell = Rc::new(RefCell::new(Some(BufWriter::new(out))));
        let out_cell_taker = out_cell.clone();
        let out_cell_putter = out_cell.clone();

        let pipe = receiver
            // populate the buffer with message
            .map(move |event| {
                let mut buf = buf_cell_taker.borrow_mut().take().expect("taken");
                serialize(&event, &mut buf)
                    .map(|_| buf)
            })
            // if something when wrong log and drop the message
            .filter_map(|ser_result|
                match ser_result {
                    Ok(ok) => Some(ok),
                    Err(err) => {
                        println!("Failed to prepare event for write output: {}", err);
                        None
                    }
                }
            )
            // write message to stdout and send back the buffer for reuse
            .and_then(move |body| {
                let out = out_cell_taker.borrow_mut().take().expect("taken");
                write_all(out, body).map_err(|err| println!("Failed to write event: {}", err))
            })
            // cleanup and back for reuse
            .map(move |(out, mut buf)| {
                // give out back
                replace(&mut *(out_cell_putter.borrow_mut()), Some(out));

                // clear buffer and give it back
                buf.clear();
                replace(&mut *(buf_cell_putter.borrow_mut()), Some(buf));

                ()
            })
            .for_each(|_| Ok(()));

        pipe.wait().expect("write_threaded future failed");
    }).expect("failed to spawn thread for write_threaded");

    // TODO: thread needs to be joined to make sure that file is synced at shutdown

    Box::new(sender.with(|message| {
        ok::<T, PipeError<IE, ()>>(message)
    }))
}

// This will block unless W can register events in event loop and
// write can return Err(std::io::ErrorKind::WouldBlock) if it would block
pub fn write_blocking<T, W, IE, SE, F>(handle: Handle, out: W, serialize: F) -> Box<Sink<SinkItem=T, SinkError=PipeError<IE, ()>>> where T: 'static, W: 'static, IE: 'static, SE: Debug + Display + 'static, W: Write, F: Fn(&T, &mut Vec<u8>) -> Result<(), SE> + 'static {
    let (sender, receiver): (Sender<T>, Receiver<T>) = channel(100);

    let buf_cell = Rc::new(RefCell::new(Some(Vec::with_capacity(64))));
    let buf_cell_taker = buf_cell.clone();
    let buf_cell_putter = buf_cell.clone();

    let out_cell = Rc::new(RefCell::new(Some(BufWriter::new(out))));
    let out_cell_taker = out_cell.clone();
    let out_cell_putter = out_cell.clone();

    let pipe = receiver
        // populate the buffer with message
        .map(move |event| {
            let mut buf = buf_cell_taker.borrow_mut().take().expect("taken");
            serialize(&event, &mut buf)
                .map(|_| buf)
        })
        // if something when wrong log and drop the message
        .filter_map(|ser_result|
            match ser_result {
                Ok(ok) => Some(ok),
                Err(err) => {
                    println!("Failed to prepare event for write output: {}", err);
                    None
                }
            }
        )
        // write message to stdout and send back the buffer for reuse
        .and_then(move |body| {
            let out = out_cell_taker.borrow_mut().take().expect("taken");
            write_all(out, body).map_err(|err| println!("Failed to write event: {}", err))
        })
        // cleanup and back for reuse
        .map(move |(out, mut buf)| {
            // give out back
            replace(&mut *(out_cell_putter.borrow_mut()), Some(out));

            // clear buffer and give it back
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
