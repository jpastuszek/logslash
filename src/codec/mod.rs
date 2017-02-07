pub mod nom;
pub mod syslog;

use tokio_core::io::Codec as TCodec;

pub trait IntoCodec {
    type Codec: TCodec;
    fn into_codec(self) -> Self::Codec;
}

impl<T> IntoCodec for T where T: TCodec {
    type Codec = T;
    fn into_codec(self) -> Self::Codec { self }
}

/*
use tokio_core::io::EasyBuf;
use std::io::Result;
pub trait AsMutCodec {
    type Codec: TCodec;
    fn as_mut_codec(&mut self) -> &mut Self::Codec;
}

impl<T> TCodec for T where T: AsMutCodec {
    type In = <<T as AsMutCodec>::Codec as TCodec>::In;
    type Out = <<T as AsMutCodec>::Codec as TCodec>::Out;

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>> {
        self.as_mut_codec().decode(buf)
    }
    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> Result<()> {
        self.as_mut_codec().encode(msg, buf)
    }
}
*/

// Shared parsing primitives
mod parse {
    use std::str::{from_utf8, Utf8Error};
    use std::num::ParseIntError;

    use chrono::{DateTime, FixedOffset};
    use chrono::format::ParseError as ChronoParesError;

    error_chain! {
        foreign_links {
            Utf8Error(Utf8Error);
            ParseIntError(ParseIntError);
            TimestampError(ChronoParesError);
        }
    }

    pub fn string(bytes: &[u8]) -> Result<&str> {
        from_utf8(bytes).map_err(From::from)
    }

    pub fn int_u8(bytes: &[u8]) -> Result<u8> {
        string(bytes).map_err(From::from)
        .and_then(|s| s.parse().map_err(From::from))
    }

    pub fn timestamp(bytes: &[u8]) -> Result<DateTime<FixedOffset>> {
        let s = string(bytes)?;

        DateTime::parse_from_rfc3339(s)
        .or(DateTime::parse_from_str(s, "%b %e %H:%M:%S"))
        .or(DateTime::parse_from_str(s, "%b %d %H:%M:%S"))
        .map_err(From::from)
    }
}
