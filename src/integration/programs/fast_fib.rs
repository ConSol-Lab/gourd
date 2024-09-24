// This file does NOT belong in a module.
// It is a resource compiled independently in the unit tests for `runner.rs`.
#![allow(unused)]

/// Fibonacci sequence, iterative implementation O(n)
///
/// Accept one u128 through **stdin** and print the fibonacci number in **stdout**
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_line = String::new();
    std::io::stdin()
        .read_line(&mut input_line)
        .expect("Failed to read line");
    let x: u128 = input_line.trim().parse().expect("Input not an integer");

    let fast = args.len() > 1 && args[1].trim() == "-f";
    if fast {
        println!("{}", fibonacci(x));
    } else {
        let _y = fibonacci(x + 10000000);
        println!("{}", fibonacci(x));
    }
}
fn fibonacci(x: u128) -> u128 {
    match x {
        0 => 0,
        1 => 1,
        _ => {
            let mut a: u128 = 0;
            let mut b: u128 = 1;
            for _ in 2..=x {
                let c = a.overflowing_add(b).0;
                a = b;
                b = c;
            }
            b
        }
    }
}