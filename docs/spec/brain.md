<!-- Copy of README sections: "## Company brain". Change the README and this file together; `cargo xtask spec-check` enforces it. -->

## Company brain

The brain is a scoped, bitemporal knowledge graph built only from the event log, with four memory tiers on top; it never hands an agent more than a small, cited slice, and every claim it makes links back to a source.

```mermaid
%%{init: {"flowchart": {"curve": "basis", "nodeSpacing": 28, "rankSpacing": 42}}}%%
flowchart TB
  EV[("Event log")]:::store
  X["Extraction pipeline"]:::sec
  subgraph TIERS["Memory tiers"]
    direction LR
    WM["Working memory<br/>the run's context window"]:::brain
    EP["Episodic<br/>run logs, progress files,<br/>decisions"]:::brain
    SEM["Semantic<br/>entities + bitemporal<br/>facts graph"]:::brain
    PRO["Procedural<br/>skill packs +<br/>department playbooks"]:::brain
  end
  subgraph DERIVED["Derived layers"]
    direction LR
    CS["Community summaries<br/>per scope"]:::brain
    BF["Precomputed briefs<br/>per leader and department"]:::brain
  end
  REF["Reflector"]:::srv
  CUR["Curator"]:::srv
  RET["Retrieval pipeline"]:::srv
  PACK["Slice packer<br/>budget, citations, pointers"]:::srv
  GE["God's eye"]:::app
  EV --> X
  X --> SEM
  SEM --> CS
  CS --> BF
  EP --> REF
  REF --> CUR
  CUR --> PRO
  SEM --> RET
  CS --> RET
  PRO --> RET
  EP --> RET
  RET --> PACK
  PACK --> WM
  BF --> GE
  RET --> GE
  classDef app fill:#dbeafe,stroke:#2563eb,color:#0b1b3a,stroke-width:1.5px
  classDef srv fill:#ede9fe,stroke:#6d28d9,color:#1e1338,stroke-width:1.5px
  classDef brain fill:#dcfce7,stroke:#15803d,color:#0b2a16,stroke-width:1.5px
  classDef store fill:#e5e7eb,stroke:#374151,color:#111827,stroke-width:1.5px
  classDef sec fill:#fee2e2,stroke:#b91c1c,color:#3b0a0a,stroke-width:1.5px
  classDef ext fill:#fef3c7,stroke:#b45309,color:#3a2504,stroke-width:1.5px
  classDef later fill:#f3f4f6,stroke:#6b7280,color:#374151,stroke-dasharray:5 3
  classDef gate fill:#ffffff,stroke:#111827,color:#111827,stroke-width:2px
```

### Fact model

- **Entities:** people, repos, PRs, issues, channels, projects, decisions, customers, documents; each with a kind, name, summary and external ids.
- **Facts:** subject, relation, object and a plain statement. Each fact has two time axes: `valid_from` and `valid_to` (when it was true in the world) and `recorded_at` and `retracted_at` (when the brain learned or withdrew it). A contradiction closes the old fact and opens a new one; nothing is overwritten.
- **Provenance:** every fact carries `source_event_id`, `source_ref` (link, title, time), `extractor_version`, `confidence` and `taint`; every answer built from facts cites them.
- **Point-in-time reads:** "what did we know on date X" filters on `recorded_at`; "what was true on date X" filters on `valid_from` and `valid_to`.

### Extraction pipeline

```mermaid
%%{init: {"flowchart": {"curve": "basis", "nodeSpacing": 28, "rankSpacing": 42}}}%%
flowchart TB
  EV[("events<br/>taint=external")]:::store
  Q["Quarantine model<br/>no tools"]:::sec
  J["Structured output<br/>entities, relations,<br/>statements, source spans"]:::srv
  S{"JSON schema<br/>valid?"}:::gate
  DL[("Dead letter<br/>+ alert")]:::store
  ER["Entity resolution"]:::brain
  M1["Attach to<br/>existing entity"]:::brain
  M2["Attach +<br/>record alias"]:::brain
  MP["Merge proposal<br/>for review"]:::app
  NE["New entity"]:::brain
  CF["Contradiction check<br/>against open facts"]:::brain
  OPS["Entity and fact ops<br/>with provenance"]:::brain
  SEQ["Sequencer<br/>numbered delta + state root"]:::srv
  EV --> Q --> J --> S
  S -->|no| DL
  S -->|yes| ER
  ER -->|external id match| M1
  ER -->|alias match| M2
  ER -->|fuzzy match| MP
  ER -->|no match| NE
  M1 & M2 & NE --> CF
  CF --> OPS --> SEQ
  classDef app fill:#dbeafe,stroke:#2563eb,color:#0b1b3a,stroke-width:1.5px
  classDef srv fill:#ede9fe,stroke:#6d28d9,color:#1e1338,stroke-width:1.5px
  classDef brain fill:#dcfce7,stroke:#15803d,color:#0b2a16,stroke-width:1.5px
  classDef store fill:#e5e7eb,stroke:#374151,color:#111827,stroke-width:1.5px
  classDef sec fill:#fee2e2,stroke:#b91c1c,color:#3b0a0a,stroke-width:1.5px
  classDef ext fill:#fef3c7,stroke:#b45309,color:#3a2504,stroke-width:1.5px
  classDef later fill:#f3f4f6,stroke:#6b7280,color:#374151,stroke-dasharray:5 3
  classDef gate fill:#ffffff,stroke:#111827,color:#111827,stroke-width:2px
```

- The quarantine model sees only the event text and the output schema; it has no tools and its output cannot carry instructions forward, only typed fields. It runs as a model relay job in locked mode on a member's local CLI, under the strictest rules, or on the company API key on the server.
- Entity resolution attaches automatically only on official ids issued by the source system: GitHub numeric user id, Slack user id, a verified email, and the ids Google, Linear, Atlassian and other plugins issue. The entity_aliases table and fuzzy name or embedding matches never attach on their own; they create a merge proposal that a person confirms by linking the official ids. Every action (assigning, mentioning, messaging, permission checks) runs against official ids, never names or aliases. Merges and splits are themselves sequenced ops, so they replay.
- Projection is idempotent on `source_event_id` and `extractor_version`; re-running extraction with a new version produces a clean set of fact changes.

### Retrieval pipeline

```mermaid
%%{init: {"flowchart": {"curve": "basis", "nodeSpacing": 28, "rankSpacing": 42}}}%%
flowchart TB
  QY["Question or task<br/>+ scope set + token budget"]:::app
  PL["Query plan"]:::srv
  subgraph CAND["Candidates, in parallel"]
    direction LR
    FTS["Full-text<br/>BM25"]:::brain
    VEC["Vector search<br/>local replica only"]:::brain
    GR["Graph expansion<br/>1-2 hops"]:::brain
    SUM["Community summaries<br/>global questions"]:::brain
  end
  FU["Reciprocal rank fusion"]:::srv
  RR["Reranker"]:::srv
  PK["Slice packer"]:::srv
  OUT["Cited slices + pointers<br/>within budget"]:::app
  QY --> PL
  PL --> FTS & VEC & GR & SUM
  FTS & VEC & GR & SUM --> FU
  FU --> RR --> PK --> OUT
  classDef app fill:#dbeafe,stroke:#2563eb,color:#0b1b3a,stroke-width:1.5px
  classDef srv fill:#ede9fe,stroke:#6d28d9,color:#1e1338,stroke-width:1.5px
  classDef brain fill:#dcfce7,stroke:#15803d,color:#0b2a16,stroke-width:1.5px
  classDef store fill:#e5e7eb,stroke:#374151,color:#111827,stroke-width:1.5px
  classDef sec fill:#fee2e2,stroke:#b91c1c,color:#3b0a0a,stroke-width:1.5px
  classDef ext fill:#fef3c7,stroke:#b45309,color:#3a2504,stroke-width:1.5px
  classDef later fill:#f3f4f6,stroke:#6b7280,color:#374151,stroke-dasharray:5 3
  classDef gate fill:#ffffff,stroke:#111827,color:#111827,stroke-width:2px
```

- **Where it runs:** members' questions run on their local node (full-text, vectors, graph, local reranker). God's eye retrieval runs on the server across every scope the leader holds (full-text, graph, community summaries, then the same small reranker on CPU), and the answer is written on the leader's own machine through the model relay, so no laptop needs every department's data and the server needs no model.
- **Budgets:** the slice packer fits results to the caller's token budget; subagents return 1,000 to 2,000 tokens to their parent.
- **Restorable compression:** a slice that does not fit is replaced by its pointer (entity id or URL), which the agent can expand later; nothing is summarised away without a way back.
- **Untrusted marking:** slices derived from external text arrive in delimited blocks marked untrusted and carry their taint into the run.

### Community summaries

- The entity graph of each scope is clustered (Leiden), and a model writes one summary per community with citations to its facts.
- A summary is rewritten only when its community changes materially (see Change detection); there is no schedule.
- Summaries answer company-wide questions cheaply: God's eye reads summaries first and drills into facts only where needed.

### Precomputed briefs (change-driven)

- A brief per leader and per department is written from community summaries, recent deltas and open tasks before anyone asks, but only when something it covers has changed.
- Material changes in a brief's scopes mark it stale; stale briefs refresh in priority order through the model relay, so God's eye opens on a current brief instantly.
- Briefs cite their facts like any answer. Precomputed briefs are a higher-tier feature; base tiers get briefs on demand.

### Change detection

Summaries and briefs change only when the company changes. Nothing refreshes on a timer, so a quiet day costs no tokens.

- **Change score:** every delta that touches a community adds to its score: a decision or a status change 5, a retracted or contradicted fact 4, a new entity 3, a new fact 1, an edited statement 0.5. Reactions, views and bot noise are dropped at ingest and score nothing.
- **Novelty check:** a new fact that repeats one the summary already cites (same subject and relation, near-identical statement by its text fingerprint) scores 0. The node that takes the job checks again with local embeddings and returns a no-op if nothing is new.
- **Trigger:** a summary is rewritten when its score reaches 10, or straight away when a decision fact changes, after 15 quiet minutes so a burst of activity causes one rewrite.
- **Incremental rewrite:** the job gets the previous summary and only the changed facts, never the whole community, so cost follows the size of the change.
- **Briefs:** a brief refreshes only when a summary it cites was rewritten, a task it lists changes state or an approval it shows is decided. With no change, God's eye shows the current brief marked with the time of the last material change, and spends nothing.

### Department playbooks

```mermaid
%%{init: {"flowchart": {"curve": "basis", "nodeSpacing": 28, "rankSpacing": 42}}}%%
flowchart LR
  RUN["Agent run<br/>trace + evaluator result"]:::srv
  RF["Reflector<br/>lessons from success and failure"]:::srv
  TNT{"Run read<br/>untrusted input?"}:::gate
  PEND["Item waits for<br/>department lead approval"]:::sec
  CU["Curator<br/>incremental ops only"]:::srv
  PB[("Playbook items<br/>per department")]:::store
  RET["Retrieved into<br/>skill pack context"]:::brain
  LEAD["Department lead"]:::app
  RUN --> RF
  RF --> TNT
  TNT -->|yes| PEND
  TNT -->|no| CU
  PEND --> PB
  RF --> CU
  CU -->|add item| PB
  CU -->|bump helpful or<br/>harmful counter| PB
  CU -->|deprecate item| PB
  PB --> RET
  LEAD -->|edit, pin, delete| PB
  classDef app fill:#dbeafe,stroke:#2563eb,color:#0b1b3a,stroke-width:1.5px
  classDef srv fill:#ede9fe,stroke:#6d28d9,color:#1e1338,stroke-width:1.5px
  classDef brain fill:#dcfce7,stroke:#15803d,color:#0b2a16,stroke-width:1.5px
  classDef store fill:#e5e7eb,stroke:#374151,color:#111827,stroke-width:1.5px
  classDef sec fill:#fee2e2,stroke:#b91c1c,color:#3b0a0a,stroke-width:1.5px
  classDef ext fill:#fef3c7,stroke:#b45309,color:#3a2504,stroke-width:1.5px
  classDef later fill:#f3f4f6,stroke:#6b7280,color:#374151,stroke-dasharray:5 3
  classDef gate fill:#ffffff,stroke:#111827,color:#111827,stroke-width:2px
```

- Playbooks are lists of small items, never one document; the curator applies add, counter and deprecate ops so a playbook cannot collapse into a vague rewrite.
- Items are scoped to their department and retrieved into context like facts, ranked by relevance and helpful-minus-harmful score.
- Lessons from runs that read untrusted input wait for a department lead's approval, so injected text cannot plant instructions in a playbook.
