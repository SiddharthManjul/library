//! Local node binary. The desktop app starts it as a child process and speaks JSON-RPC over stdio.
//! It opens no listening port. stdout is for protocol messages only.

use std::io::{BufReader, BufWriter};

fn main() {
    let node = library_node::rpc::Node::new();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    if let Err(e) = node.serve(BufReader::new(stdin.lock()), BufWriter::new(stdout.lock())) {
        eprintln!("library-node: stdio closed: {e}");
        std::process::exit(1);
    }
}
