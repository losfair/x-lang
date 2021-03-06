extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate rpds;
extern crate slab;

pub mod ast;
pub mod builtin;
pub mod corelib;
pub mod error;
pub mod eval;
pub mod host;
pub mod parser;
pub mod typeck;

#[cfg(test)]
mod typeck_test;
