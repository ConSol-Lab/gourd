#![allow(unused)]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("Hello, {}!", args[1]);
    eprintln!("I don't know {}", args[1..].join(" or "));
}