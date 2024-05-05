#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![allow(clippy::redundant_static_lifetimes)]

//! Gourd allows

use std::fs;
use std::path::PathBuf;
use std::process::ExitStatus;

use crate::constants::X86_64_E_MACHINE;
use crate::shared::Measurement;
use crate::wrapper::wrap;
use crate::wrapper::Program;

#[cfg(test)]
/// The tests validating the behaviour of `gourd`.
mod tests;

/// The error type of `gourd`.
pub mod error;

/// The binary wrapper around run programs.
pub mod wrapper;

/// Constant values.
pub mod constants;

/// Code shared between the wrapper and `gourd`.
pub mod shared;

/// The main entrypoint.
///
/// This function is the main entrypoint of the program.
#[cfg(not(tarpaulin_include))]
fn main() {
    println!("Hello, world!");

    let path = "./bin".parse::<PathBuf>().unwrap();

    let _: Vec<ExitStatus> = wrap(
        vec![Program {
            binary: path,
            arguments: vec![],
        }],
        vec!["./test1".parse().unwrap()],
        X86_64_E_MACHINE,
    )
    .unwrap()
    .iter_mut()
    .map(|x| {
        println!("{:?}", x);
        x.spawn().unwrap().wait().unwrap()
    })
    .collect();

    let results: Measurement = toml::from_str(
        &String::from_utf8(fs::read("/tmp/gourd/algo_0/0_result").unwrap()).unwrap(),
    )
    .unwrap();

    println!("{:?}", results);
}
