pub mod built {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() {
    println!("Hello, world!");
}
