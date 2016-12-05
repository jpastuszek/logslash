mod nom;
pub mod syslog;

// Shared parsing primitives
mod parse {
    use std::str::{from_utf8, Utf8Error};
    use std::num::ParseIntError;

    error_chain! {
        foreign_links {
            Utf8Error(Utf8Error);
            ParseIntError(ParseIntError);
        }
    }

    pub fn string(bytes: &[u8]) -> Result<&str> {
        from_utf8(bytes).map_err(From::from)
    }

    pub fn int_u8(bytes: &[u8]) -> Result<u8> {
        string(bytes).map_err(From::from)
            .and_then(|s| s.parse().map_err(From::from))
    }
}
