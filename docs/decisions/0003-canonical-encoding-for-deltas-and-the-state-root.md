# 0003 Canonical encoding for deltas and the state root

Status: accepted
Date: 2026-11-26

## Context
Replicas must prove they hold exactly the server's history, so the bytes that are hashed must be identical on every machine and in every language.

## Decision
A delta is hashed as deterministic CBOR (RFC 8949 section 4.2: sorted map keys, shortest integer forms). `root[n] = SHA-256(root[n-1] || canonical(delta[n]))` with a fixed per-scope genesis value. No floats in hashed fields: times are integer microseconds and confidence is integer permille (0 to 1000), as in `contracts/delta.schema.json`.

## Options considered
Hash JSON text (key order differs between serializers). Protobuf (needs a schema compiler on both sides, less strict about canonical form). A Merkle tree over state (stronger proofs, deferred as an upgrade).

## Consequences
The sequencer and the node share one apply and one encode module. Property tests cover round trips and root determinism.
