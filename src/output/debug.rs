use futures::sync::mpsc;
use futures::future::Future;
use futures::Stream;
use std::fmt::Debug;

pub fn print_debug<T: Debug + 'static>(source: mpsc::Receiver<T>) -> Box<Future<Item=(),Error=()>> {
    Box::new(source.for_each(|message| {
        println!("{:#?}", &message);
        Ok(())
    }))
}

use std::io::Cursor;
use maybe_string::MaybeString;
use event::{LogstashEvent, SerializeEvent, SerdeFieldSerializer, FieldSerializer};
use serde_json::ser::Serializer;

pub fn print_logstash<T: LogstashEvent + 'static>(source: mpsc::Receiver<T>) -> Box<Future<Item=(),Error=()>> {
    Box::new(source.for_each(|message| {
        let data = Cursor::new(Vec::new());
        let mut ser = Serializer::new(data);

        {
            let field_ser = SerdeFieldSerializer::new(&mut ser).expect("field serializer")
                .rename("severity", "log_level")
                .map_str("severity", |l| l.to_lowercase())
                .map_str("message", |m| m.replace("#012", "\n"));
            message.serialize(field_ser).expect("serialized message");
        }

        let json = ser.into_inner().into_inner();
        // TODO error handling
        println!("{}", MaybeString(json));

        Ok(())
    }))
}
