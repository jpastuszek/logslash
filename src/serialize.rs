use futures::stream::Stream;

/*
pub trait Serialize<T, E> {
    type Output;
    type Error;
    //TODO: moving T by value will be slow
    // ideally input would be a stream of Write targets and output would be a stream of WriteAll
    // futures
    fn serialize<I: Stream<Item=T, Error=E>>(&self, events: I) -> Box<Stream<Item=(T, Self::Output), Error=Self::Error>>;
}
*/

/*
pub trait Serialize<T> {
    type Error;
    //TODO: how to avoid allocations?
    fn serialize(&self, event: &T) -> Result<Vec<u8>, Error>:
}
*/

// Given Write output and refrence to event T it will return future that will not return untill all
// data has be serialized into output
pub trait Serialize<T, W: Write> {
    type Error;
    // ideally input would be a stream of Write targets and output would be a stream of WriteAll
    // futures
    fn serialize(&self, output: W, event: &T) -> Box<Future<Item=W, Error=Self::Error>>;
}

/*
struct JSONEventSerializer {};
impl Serialize<T, E> for  JSONEventSerializer {
    type Output = String;
    type Error = ;

    fn serialize<I: Stream<Item=T, Error=E>>(&self, events: I) -> Box<Stream<Item=(T, Self::Output), Error=Self::Error>> {
        Box::new(events.then(|event| {
            match event {
                Ok(event) => {
                    let data = Cursor::new(Vec::new());
                    let mut serializer = serde_json::ser::Serializer::new(data);

                    serializer.serialize_map_end(state)?;
                    let mut state = serializer.serialize_map(None)?;
                }
            }
        })
    }
}
*/
