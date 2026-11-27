<!-- Copy of README sections: "## Evals and quality". Change the README and this file together; `cargo xtask spec-check` enforces it. -->

## Evals and quality

Four eval suites gate every release; each starts with 20 to 50 tasks drawn from real failures, grades outcomes rather than exact steps, and reports pass^k so reliability across repeated runs is measured, not just a single success.

```mermaid
%%{init: {"flowchart": {"curve": "basis", "nodeSpacing": 28, "rankSpacing": 42}}}%%
flowchart LR
  FAIL["Real failures +<br/>bug reports"]:::sec
  TR["Run traces"]:::srv
  SUITE["Eval suites"]:::srv
  RUNK["Run each task k times"]:::srv
  GRADE["Outcome grader<br/>tests, schemas, rubric,<br/>human labels"]:::srv
  PK["pass^k per suite"]:::srv
  GATE{"Above threshold?"}:::gate
  REL["Release"]:::brain
  BLOCK["Block release<br/>+ open issue"]:::sec
  FAIL --> SUITE
  TR --> SUITE
  SUITE --> RUNK --> GRADE --> PK --> GATE
  GATE -->|yes| REL
  GATE -->|no| BLOCK
  classDef app fill:#dbeafe,stroke:#2563eb,color:#0b1b3a,stroke-width:1.5px
  classDef srv fill:#ede9fe,stroke:#6d28d9,color:#1e1338,stroke-width:1.5px
  classDef brain fill:#dcfce7,stroke:#15803d,color:#0b2a16,stroke-width:1.5px
  classDef store fill:#e5e7eb,stroke:#374151,color:#111827,stroke-width:1.5px
  classDef sec fill:#fee2e2,stroke:#b91c1c,color:#3b0a0a,stroke-width:1.5px
  classDef ext fill:#fef3c7,stroke:#b45309,color:#3a2504,stroke-width:1.5px
  classDef later fill:#f3f4f6,stroke:#6b7280,color:#374151,stroke-dasharray:5 3
  classDef gate fill:#ffffff,stroke:#111827,color:#111827,stroke-width:2px
```

| Suite | What it measures | Graded by |
| --- | --- | --- |
| Harness | Department tasks end to end: correct result, within budget, no premature victory, correct single-agent vs fan-out choice | Tests for code, schemas for structured output, rubric plus calibrated grader for design and marketing |
| Brain | Extraction accuracy, entity resolution, retrieval recall@k, citation correctness, point-in-time answers, brief freshness | Labelled event sets and question sets |
| Ambient | Proposal usefulness, and silence: staying quiet when nothing is worth raising | Labelled scenarios, dismissal rates |
| Security | Injection corpus through quarantine, taint enforcement, rug-pull detection, scope isolation, exfiltration attempts | Pass or fail per case; any failure blocks release |

- pass^k is the share of tasks that succeed on all k runs; at 75% per run, three runs in a row succeed about 42% of the time, which is why pass^k is the bar for anything customers rely on.
- Eval results are stored in `eval_runs` per build and shown to admins.
- New failures found in production become eval tasks.
