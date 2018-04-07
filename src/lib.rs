#![warn(missing_docs)]

//! Quake, ported to Rust.

extern crate byteorder;
#[macro_use] extern crate failure;
// #[macro_use] extern crate failure_derive;

pub mod cvar;
pub mod defs;
pub mod parms;
pub use parms::Parms;
pub mod fs;
#[cfg(test)]
mod test_common;
pub mod try_from_temp;
pub mod util;
pub mod wad;
