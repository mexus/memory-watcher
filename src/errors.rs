use std::ffi::OsString;
use std::{io, num, str};
use super::log4rs;

quick_error! {
    /// Error returned by a `kill` libc function.
    #[derive(Debug)]
    pub enum KillError {
        InvalidSignal
        PermissionDenied
        NotFound
    }
}

quick_error! {
    /// Various errors.
    #[derive(Debug)]
    pub enum Error {
        Io(err: io::Error) {
            cause(err)
            from()
        }
        ParseIntCtx(source: String, err: num::ParseIntError) {
            context(source: &'a str, err: num::ParseIntError)
                -> (source.to_string(), err)
            cause(err)
            display("Parse error while parsing [{}]", source)
        }
        ParseInt(err: num::ParseIntError) {
            from()
            cause(err)
        }
        OsStringConversion(os_str: OsString) {
            from()
            display("Can't parse OsString [{:?}]", os_str)
        }
        MoreThanOne {
            display("Found more than one running process")
        }
        NotFound {
            display("Process not found")
        }
        KillError(err: KillError) {
            from()
            cause(err)
        }
        LogError(err: log4rs::Error) {
            cause(err)
            from()
        }
    }
}
