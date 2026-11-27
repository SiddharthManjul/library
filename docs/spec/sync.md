<!-- Copy of README sections: "## Sync engine". Change the README and this file together; `cargo xtask spec-check` enforces it. -->

## Sync engine

Sync works like a chain without consensus: the sequencer is the single writer per tenant, every change is a numbered delta per scope with a state root, and replicas apply deltas in order, verify, and repair themselves from snapshots.

```mermaid
%%{init: {"sequence": {"showSequenceNumbers": true, "mirrorActors": false, "messageAlign": "left"}}}%%
sequenceDiagram
  autonumber
  box rgb(237,233,254) Server
    participant S as Sequencer
    participant P as Postgres
    participant G as Gateway
  end
  box rgb(229,231,235) Storage
    participant R as R2
  end
  box rgb(219,234,254) Member machine
    participant N as Local node
    participant D as SurrealDB replica
  end
  S->>P: one transaction: state rows + delta(scope, seq, ops, state_root)
  S->>R: snapshot on schedule or after a large batch
  N->>G: sync.subscribe(scope, applied_seq, state_root) per scope
  G->>G: user still holds scope?
  alt far behind or new device
    G-->>N: sync.snapshot link (short-lived R2 URL)
    N->>R: download snapshot
    N->>D: load snapshot, set applied_seq
  end
  G-->>N: missing deltas in order, then live sync.delta pushes
  N->>D: apply each delta in one local transaction
  N->>N: recompute root = SHA-256(previous root || canonical delta)
  alt root mismatch
    N->>D: discard scope partition
    N->>G: reload from snapshot
  end
  G-->>N: scope.revoked
  N->>D: delete scope partition, drop scope key
```

- **State roots:** each delta's root is `SHA-256(previous root || canonical delta)`. Two replicas with the same root at the same sequence have applied exactly the same history. A Merkle tree over state, for proving single facts, is a later upgrade.
- **What deltas carry:** entity, fact, summary and playbook changes with statements and source references (link, title, time), never raw event bodies. Replicas stay small and hold less sensitive material.
- **Scope keys:** each scope's replica is encrypted at rest with a scope key; the server wraps it to the device's X25519 public key; keys rotate, and revoked users stop receiving them.
- **Cursors:** `replica_cursors` records each device's applied sequence and root, so drift is visible on the server.
- **Offline:** an offline node catches up from its cursor on reconnect.
- **Writes from clients:** replicas never write state; the node submits actions through the gateway, and only the sequencer turns accepted actions into deltas.
