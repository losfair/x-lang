extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod ast;
pub mod builtin;
pub mod corelib;
pub mod error;
pub mod host;
pub mod typeck;

#[cfg(test)]
mod typeck_test;
