// This file does NOT belong in a module.
// It is a resource compiled independently in the unit tests for `runner.rs`.
#![allow(unused)]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let x: u128 = args[1].parse().expect("Invalid number (u64)");
    println!("{}", fibonacci(x));
}
fn fibonacci(x: u128) -> u128 {
    match x {
        0 => 0,
        1 => 1,
        _ => fibonacci(x - 1) + fibonacci(x - 2),
    }
}