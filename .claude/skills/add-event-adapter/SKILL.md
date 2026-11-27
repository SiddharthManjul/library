---
name: add-event-adapter
description: Add an inbound event adapter with signature check, dedupe and normalisation.
---

# Add event adapter

1. Verify the signature before reading the body: GitHub HMAC-SHA256 with constant-time compare, Slack signing secret over timestamp and body with a 5-minute window, MCP Events with Standard Webhooks.
2. Normalise to `contracts/events.schema.json`; `taint` is always `external`.
3. Insert with the unique key (tenant_id, source, external_id); answer 2xx only after the commit; process asynchronously.
4. Never log event bodies.
5. Tests: forged signature, stale timestamp, replayed delivery, duplicate delivery producing one event, oversized body.
6. Add the fixture to the injection corpus if the source carries free text.
