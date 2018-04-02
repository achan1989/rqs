//! Temporary version of TryFrom, until it's available in stable Rust.

use std::error::Error;
use std::fmt;

pub trait TryFromTemp<T>: Sized {
    type Error;

    fn try_from_temp(value: T) -> Result<Self, Self::Error>;
}

impl TryFromTemp<i32> for usize {
    type Error = TryFromIntError;

    fn try_from_temp(val: i32) -> Result<usize, TryFromIntError> {
        if val >= 0 {
            Ok(val as usize)
        } else {
            Err(TryFromIntError(()))
        }
    }
}

impl TryFromTemp<i32> for u64 {
    type Error = TryFromIntError;

    fn try_from_temp(val: i32) -> Result<u64, TryFromIntError> {
        if val >= 0 {
            Ok(val as u64)
        } else {
            Err(TryFromIntError(()))
        }
    }
}

/// The error type returned when a checked integral type conversion fails.
#[derive(Debug, Copy, Clone)]
pub struct TryFromIntError(());

impl Error for TryFromIntError {
    fn description(&self) -> &str {
        self.__description()
    }
}

impl TryFromIntError {
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        "out of range integral type conversion attempted"
    }
}

impl fmt::Display for TryFromIntError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.__description().fmt(fmt)
    }
}
