mod nom;
pub mod syslog;

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
