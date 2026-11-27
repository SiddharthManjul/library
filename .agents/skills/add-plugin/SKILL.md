---
name: add-plugin
description: Add a standard plugin: catalog entry, sign-in mode, tool review defaults and a smoke test.
---

# Add plugin

1. Add an entry to `plugins/catalog.yaml`: id, name, departments, endpoint (confirmed against the vendor's docs), sign-in mode (1 automatic, 2 our app, 3 customer app, 4 key), scopes requested, default tier per tool, event adapter, smoke test.
2. Request only the scopes in README: Decisions on the open questions (item 11). Nothing wider.
3. New tools start as "always ask" until an admin sets a tier.
4. The smoke test connects, lists tools, pins their hashes and calls one read tool through the dispatcher.
5. Mode 1 plugins get a probe that fails the build if the vendor drops dynamic registration.
6. Only the dispatcher calls the plugin; user-added URLs are checked against private ranges on every request, with no redirects.
