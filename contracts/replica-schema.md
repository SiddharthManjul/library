# Replica schema (SurrealDB embedded)

One database per scope inside a tenant namespace, so one scope can be wiped or re-keyed alone. Replicas never write state; they apply numbered deltas from the server and verify state roots. Source: README, Data model (Client: SurrealDB embedded replica).

| Table | Type | Fields |
| --- | --- | --- |
| `entity` | node | Mirrors the server row, plus local `embedding` |
| `fact` | edge (entity to entity) | Mirrors the server row, plus local `embedding` |
| `summary` | record | Mirrors `community_summaries`, plus local `embedding` |
| `playbook_item` | record | Mirrors active `playbook_items` for the scope's department |
| `sync_meta` | record | `applied_seq`, `state_root`, `key_version` |

## Encryption layout

- Every record's content (statements, summaries, source titles and links, embeddings) is sealed with XChaCha20-Poly1305 under the scope key before it is written.
- A fresh random 24-byte nonce per record, and the record id as associated data, so a sealed value cannot be moved to another record.
- Readable on disk so graph traversal works: ids, kinds, relation types, scope, sequence numbers, timestamps.
- Indexes (full-text, HNSW) are built in memory when the node unlocks and are never written to disk. SurrealDB's own on-disk full-text and vector indexes must not be used: they would store plaintext.
- Embeddings never leave the machine in v0.
- The node refuses to sync a scope when OS disk encryption is off.
- Revocation: delete the scope database and the scope key; the server stops wrapping keys for that device.

## State root

`root[n] = SHA-256(root[n-1] || canonical(delta[n]))`, with `root[0]` a fixed per-scope genesis value. Canonical encoding is deterministic CBOR (`delta.schema.json`). Never hash JSON text.
