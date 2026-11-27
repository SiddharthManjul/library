# 0005 Replica encryption at the application layer

Status: accepted
Date: 2026-11-26

## Context
SurrealDB's own encryption at rest is an Enterprise feature, and revocation must make data on a laptop unreadable.

## Decision
The node seals each record's content with XChaCha20-Poly1305 under the scope key, with a random 24-byte nonce and the record id as associated data. Ids, kinds, relation types, scope, sequence numbers and timestamps stay readable for graph traversal. Search indexes live in memory only. The node requires OS disk encryption before it syncs a scope.

## Options considered
SurrealDB Enterprise (paid, and keys are not per scope). Encrypt the whole file (no per-scope wipe).

## Consequences
Revoking a scope is deleting its key. Data already on a laptop can never be fully recalled, and customers are told so.
