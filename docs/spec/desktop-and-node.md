<!-- Copy of README sections: "## Desktop app and local node". Change the README and this file together; `cargo xtask spec-check` enforces it. -->

## Desktop app and local node

The desktop app is only the interface; the local node owns the session, the network link, the replica and all local compute, so the app never touches a token or the network.

```mermaid
%%{init: {"flowchart": {"curve": "basis", "nodeSpacing": 28, "rankSpacing": 42}}}%%
flowchart TB
  subgraph APP["Desktop app, sandboxed Electron renderer"]
    direction LR
    UI1["God's eye<br/>+ briefs"]:::app
    UI2["Sign-off inbox<br/>+ approvals"]:::app
    UI3["Tasks<br/>+ agent runs"]:::app
    UI4["Proposals<br/>inbox"]:::app
    UI5["Connections<br/>+ tool approvals"]:::app
    UI6["Scope badge, previews,<br/>sources, undo"]:::app
    UI7["Sync status"]:::app
  end
  BR["Allowlisted IPC bridge<br/>validated payloads"]:::sec
  UI1 & UI2 & UI3 & UI4 & UI5 & UI6 & UI7 --> BR
  subgraph NODE["Local node, Rust process"]
    RPC["stdio JSON-RPC server"]:::app
    subgraph NET["Session and network"]
      direction LR
      SES["Session<br/>keychain tokens, device key"]:::sec
      WS["WSS client<br/>one connection"]:::app
      PROX["remote.* proxy"]:::app
    end
    subgraph DATA["Replica and retrieval"]
      direction LR
      SYN["Sync client<br/>apply, verify root, repair"]:::app
      KEY["Scope key store<br/>X25519 unwrap"]:::sec
      REP[("SurrealDB replica<br/>one DB per scope")]:::store
      EMB["Embedder<br/>ort"]:::app
      RET["Hybrid retrieval<br/>FTS + HNSW + graph"]:::app
      RR["Cross-encoder<br/>reranker"]:::app
    end
    SBX["Sandbox host<br/>after v0"]:::later
    JOB["Job worker<br/>after v0"]:::later
  end
  GW["Gateway via Cloudflare"]:::sec
  BR <--> RPC
  RPC --> RET
  RPC --> PROX
  RPC --> SYN
  PROX --> WS
  SES --> WS
  WS --> SYN
  SYN --> KEY
  SYN --> REP
  REP --> EMB
  EMB --> REP
  RET --> REP
  RET --> RR
  SBX -.-> PROX
  JOB -.-> WS
  WS <--> GW
  classDef app fill:#dbeafe,stroke:#2563eb,color:#0b1b3a,stroke-width:1.5px
  classDef srv fill:#ede9fe,stroke:#6d28d9,color:#1e1338,stroke-width:1.5px
  classDef brain fill:#dcfce7,stroke:#15803d,color:#0b2a16,stroke-width:1.5px
  classDef store fill:#e5e7eb,stroke:#374151,color:#111827,stroke-width:1.5px
  classDef sec fill:#fee2e2,stroke:#b91c1c,color:#3b0a0a,stroke-width:1.5px
  classDef ext fill:#fef3c7,stroke:#b45309,color:#3a2504,stroke-width:1.5px
  classDef later fill:#f3f4f6,stroke:#6b7280,color:#374151,stroke-dasharray:5 3
  classDef gate fill:#ffffff,stroke:#111827,color:#111827,stroke-width:2px
```

- **Process model:** the Electron main process starts the local node as a child process and talks to it over stdio; the renderer has `contextIsolation` and `sandbox` on, `nodeIntegration` off, a strict content security policy and no remote content. T3 Code's bundled Node WebSocket server is removed; its provider adapters become Rust provider runners inside the local node, so no local port is opened.
- **Session:** tokens and the device's private keys (X25519 for sealing, Ed25519 for signing) are stored only in the OS keychain through `keyring`: Windows Credential Manager on Windows, the Secret Service (GNOME Keyring or KWallet) on Linux. The node loads a key into memory only to use it and zeroises it afterwards. The keychain protects keys per OS user, so any process running as that user can read them; the threat model assumes the member's own account is not compromised. Access tokens refresh in the node.
- **Replica:** one SurrealDB database per scope inside a tenant namespace, so a scope can be wiped or re-keyed alone.
- **Local retrieval:** members' questions about their own scopes run here, offline-capable; the harness can also request a context slice from a member's node when that member is the requester.
- **Reranker:** a small cross-encoder in `ort` reranks the top candidates from full-text, vector and graph search before results go into context.
- **Confidence UI:** every agent answer shows source chips; every pending action shows a preview of exactly what will happen; the scope badge shows which scopes the current agent can read; reversible actions offer undo.

### Platforms and packaging

Windows comes first, then Linux; macOS comes after v0. Every phase's exit gate that names platforms means Windows 11 and Linux.

| | Windows | Linux |
| --- | --- | --- |
| Minimum | Windows 11 x64 | x64, Ubuntu LTS and Debian (glibc) |
| Package | MSIX family package (built as `.appx` by electron-builder), published through the Microsoft Store | `.deb` and AppImage |
| Signing | The Store signs the package at no cost; the local node binary ships inside the package, so it is covered too | None needed; checksums published with each release |
| Updates | Through the Store | `.deb` through the package manager; AppImage through its own update check |
| Company rollout | Intune deploys Store apps | The company's own package tooling |
| Keychain | Credential Manager | Secret Service; the node refuses to sign in without one and says why |
| Disk encryption check | BitLocker or Windows Device encryption | LUKS (dm-crypt) on the volume holding the replica |

- **Store packaging:** the app is a full-trust packaged desktop app. MSIX redirects a packaged process's writes to AppData and parts of the registry into a private location, and child processes inherit this. The provider runners must be tested to confirm each CLI they start (Claude Code, Codex, Kiro and the rest) sees the member's real configuration and sign-in; Windows 11 lets a package turn the redirection off, which is the fallback.
- **Disk encryption check without admin rights:** the usual tools (`manage-bde`, the BitLocker WMI class) need an administrator, and members are often not administrators. The node must find the volume's protection state without elevation; until that method is proven, this is a P4 spike. If the state cannot be read, the node refuses to sync and tells the member.
- **Corporate networks:** the node honours the system proxy settings (including PAC files) and trusts the OS certificate store, so its WebSocket works behind proxies that inspect TLS.
- **Starting CLIs on Windows:** many CLIs install as `.cmd` or `.ps1` shims. The runners resolve the real executable where one exists and otherwise start the shim through `cmd.exe` with strict argument quoting, never by building a shell string.
- **Paths:** git long paths are enabled, and worktrees use short base paths, because Windows still limits many tools to 260 characters.
- **Antivirus and EDR:** company security tools inspect unsigned or unusual child processes. The node is signed inside the package, starts only the CLIs a member enabled, and logs each start, so security teams can allow it by publisher.
