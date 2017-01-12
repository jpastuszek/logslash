use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;

use tokio_core::io::{Codec, EasyBuf};

use nom::{IResult, ErrorKind};

pub type NomParser<T> = fn(&[u8]) -> IResult<&[u8], T, &'static str>;

pub struct NomCodec<T> {
    parser: NomParser<T>
}

impl<T> NomCodec<T> {
    pub fn new(parser: NomParser<T>) -> NomCodec<T> {
        NomCodec {
            parser: parser
        }
    }
}

impl<T> Clone for NomCodec<T> {
    fn clone(&self) -> NomCodec<T> {
        NomCodec {
            parser: self.parser
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
            IResult::Error(ErrorKind::Custom(err)) => {
                Err(IoError::new(IoErrorKind::InvalidInput, err))
            }
            IResult::Error(_) => {
                Err(IoError::new(IoErrorKind::InvalidData, "unexpected parser error"))
            }
            IResult::Incomplete(_) => {
                Ok(None)
            }
        };

        if consumed > 0 {
            buf.drain_to(consumed);
        }
        result
    }

    fn encode(&mut self, _msg: Self::Out, _buf: &mut Vec<u8>) -> Result<(), IoError> {
        panic!("NomCodec: encode unimplemented!")
    }
}
