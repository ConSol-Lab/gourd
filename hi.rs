use std::io;

fn main() {
  let mut inpt = String::new();
  io::stdin()
      .read_line(&mut inpt)
      .expect("Failed to read line");

  let num: i32 = inpt.trim().parse().unwrap();

  println!("{}", num);
}
