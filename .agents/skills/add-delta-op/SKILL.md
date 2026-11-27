---
name: add-delta-op
description: Add a delta op with canonical encoding, apply logic on server and replica, and replay tests.
---

# Add delta op

1. Add the op to `contracts/delta.schema.json` and a valid and an invalid fixture under `contracts/fixtures/delta/`.
2. Encoding is deterministic CBOR (sorted keys, shortest integers, no floats in hashed fields, times as integer microseconds). Never hash JSON text.
3. Implement apply once, in a shared module the sequencer and the node both call.
4. Tests: round trip, byte-identical encoding across runs, replaying the whole log from empty gives the same state roots, and a replica applying the op reaches the server's root.
5. Only the sequencer writes state tables; the op must not need any other writer.
6. Update ADR 0003 if the root formula or encoding changes.
