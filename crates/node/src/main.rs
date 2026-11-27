//! Local node binary. The desktop app starts it as a child process and speaks JSON-RPC over stdio.
//! It opens no listening port. stdout is for protocol messages only.

use std::sync::Arc;

use library_node::{
    keychain::{OsKeychain, SecretStore},
    link::Link,
    rpc::Node,
    transport::GatewayTransport,
};
use tokio::io::BufReader;

/// The OS keychain. In debug builds only, `LIBRARY_DEV_KEYCHAIN_FILE` swaps in a plain file for tests.
fn open_store() -> Arc<dyn SecretStore> {
    #[cfg(debug_assertions)]
    if let Ok(path) = std::env::var("LIBRARY_DEV_KEYCHAIN_FILE") {
        return Arc::new(library_node::keychain::DevFileStore::new(path));
    }
    Arc::new(OsKeychain::new("library-ide"))
}

/// Where the gateway lives. Set at build time for releases, and by the environment for development.
fn gateway_url() -> String {
    std::env::var("LIBRARY_GATEWAY_URL").unwrap_or_else(|_| "https://api.library.ide".to_string())
}

#[tokio::main]
async fn main() {
    // Secrets live in the OS keychain only. A keychain that cannot be read stops the node with a reason,
    // instead of starting as if nobody were signed in.
    let store = open_store();
    let (link, mut pushes) = match Link::new(GatewayTransport::new(gateway_url()), store) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("library-node: {}", e.message);
            std::process::exit(2);
        }
    };
    let link = Arc::new(link);
    // Signed in at startup: try to come online. Failure is not fatal; the node stays usable offline.
    if link.identity().await.is_some() {
        let _ = link.connect(library_node::link::now_secs()).await;
    }
    // Notifications are drained here until the app-side channel exists (P4).
    tokio::spawn(async move { while pushes.recv().await.is_some() {} });

    let node = Arc::new(Node::new(link));
    if let Err(e) = node
        .serve(BufReader::new(tokio::io::stdin()), tokio::io::stdout())
        .await
    {
        eprintln!("library-node: stdio closed: {e}");
        std::process::exit(1);
    }
}
