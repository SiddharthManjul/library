//! Server binary: wires every crate into one process.

fn main() {
    println!("library-server {}", env!("CARGO_PKG_VERSION"));
}
