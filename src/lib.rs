#![warn(missing_docs)]

//! Quake, ported to Rust.

extern crate byteorder;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;

pub mod defs;
pub mod parms;
pub use parms::Parms;
pub mod fs;
pub mod try_from_temp;
