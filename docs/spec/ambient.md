<!-- Copy of README sections: "## Ambient agents". Change the README and this file together; `cargo xtask spec-check` enforces it. -->

## Ambient agents

Ambient agents sense changes in the brain and propose actions without being asked; they never act on their own, and they are tuned to stay quiet when nothing is worth raising. They are built last and limited to higher tiers.

```mermaid
%%{init: {"flowchart": {"curve": "basis", "nodeSpacing": 28, "rankSpacing": 42}}}%%
flowchart TB
  DL["New deltas"]:::store
  CRON["Scheduled checks"]:::srv
  TR["Trigger evaluator"]:::srv
  QUIET["Stay silent"]:::later
  DUP{"Already proposed<br/>or dismissed?"}:::gate
  RUN["Harness run with the<br/>trigger's skill pack"]:::srv
  LVL{"Level"}:::gate
  N["Notification only"]:::app
  Q["Asks the owner<br/>a question"]:::app
  P["Proposal with an<br/>exact action preview"]:::app
  APR["Owner or department lead<br/>approves, edits or dismisses"]:::sec
  DSP["Dispatcher + audit"]:::sec
  FB["Feedback to trigger tuning<br/>+ decision log"]:::srv
  DL --> TR
  CRON --> TR
  TR -->|no trigger| QUIET
  TR -->|trigger fires, e.g. PR open<br/>24h with no review| DUP
  DUP -->|yes| QUIET
  DUP -->|no| RUN
  RUN --> LVL
  LVL -->|notify| N
  LVL -->|question| Q
  LVL -->|review| P
  P --> APR
  APR -->|approved| DSP
  APR -->|dismissed| FB
  classDef app fill:#dbeafe,stroke:#2563eb,color:#0b1b3a,stroke-width:1.5px
  classDef srv fill:#ede9fe,stroke:#6d28d9,color:#1e1338,stroke-width:1.5px
  classDef brain fill:#dcfce7,stroke:#15803d,color:#0b2a16,stroke-width:1.5px
  classDef store fill:#e5e7eb,stroke:#374151,color:#111827,stroke-width:1.5px
  classDef sec fill:#fee2e2,stroke:#b91c1c,color:#3b0a0a,stroke-width:1.5px
  classDef ext fill:#fef3c7,stroke:#b45309,color:#3a2504,stroke-width:1.5px
  classDef later fill:#f3f4f6,stroke:#6b7280,color:#374151,stroke-dasharray:5 3
  classDef gate fill:#ffffff,stroke:#111827,color:#111827,stroke-width:2px
```

- **Triggers:** each trigger is a declared rule over state (kind, condition, scope, cooldown); the first is a PR open 24 hours with no review, which proposes a Slack nudge to its owner.
- **Three levels:** notify (inform only), question (ask the owner something) and review (a proposed action waiting for approval), following the notify, question and review pattern for human-in-the-loop agents.
- **Silence:** cooldowns, duplicate suppression and dismissal feedback keep noise down; the silence eval measures how often the engine stays quiet when it should.
- **Execution:** an approved proposal goes through the dispatcher and the audit log like any other side effect; ambient runs carry the same taint rules as task runs.
