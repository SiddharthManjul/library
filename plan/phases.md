# Build phases P0 to P10

Ordered by dependency, not by date. A phase starts only when the one before it has passed its exit gate. Each phase has a task file `plan/Pn.tasks.json`.

Task status is one of `todo`, `claimed`, `blocked`, `done`. Only CI sets `passes`.

| Phase | Name | Tasks | Exit gate |
| --- | --- | --- | --- |
| P0 | Foundation | 16 | RLS isolation tests pass against real Postgres; sign-in works on Windows 11 and Linux from installed packages; `just check` green on the Linux and Windows CI runners. |
| P1 | Trust core | 9 | audit chain verifier passes; log-redaction test finds no secret; every method rejects without role, scope or plan. |
| P2 | Ingest and quarantine | 9 | forged and replayed webhooks rejected; injection corpus produces no instruction-bearing output. |
| P3 | Brain core | 12 | replaying the event log gives identical state roots; extraction eval at its threshold (F1 0.85). |
| P4 | Local node, sync and retrieval | 10 | tampered replica detected and repaired; revoked scope deleted and keys stop; retrieval eval at its threshold (recall@10 0.90). |
| P5 | Harness | 14 | harness eval suite at its pass^k bar; budget and loop kill tests pass. |
| P6 | Plugins | 8 | every standard plugin passes its smoke test; a changed tool definition is blocked until re-approved. |
| P7 | Work and approvals | 5 | sign-off race test passes; tainted run cannot make an external write without approval. |
| P8 | Brain intelligence | 7 | brain eval suite at its thresholds; briefs refresh on new deltas. |
| P9 | Ambient agents | 4 | silence eval and proposal eval at their thresholds (60% useful, silent in 95%). |
| P10 | Production hardening | 7 | the restore drill rebuilds the VPS and replicas match roots; security evals pass; all prior gates still green.

--- |

## Platform gates

Everywhere a gate names platforms it means Windows 11 x64 and Linux x64 (README: Platforms and packaging). macOS comes after v0.

## P0 exit gate, as adjusted on 26 Nov 2026

The original gate asks that sign-in works on both platforms. OAuth providers were deferred (P0-16), so P0 closes on: row-level security isolation tests green against real Postgres; the installed app starts its bundled node and shows the sign-in screen on Windows 11 and Linux; `just check` green on the Linux and Windows CI runners. Sign-in itself becomes the exit gate of P0-16.
