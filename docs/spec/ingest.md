<!-- Copy of README sections: "## Ingest and event adapter". Change the README and this file together; `cargo xtask spec-check` enforces it. -->

## Ingest and event adapter

Every external event enters through one ingest path that verifies, deduplicates and commits it before anything reads it; the text inside is untrusted from that moment on.

```mermaid
%%{init: {"flowchart": {"curve": "basis", "nodeSpacing": 28, "rankSpacing": 42}}}%%
flowchart LR
  subgraph SRC["Sources"]
    direction TB
    GH["GitHub App webhook"]:::ext
    SL["Slack Events API<br/>tenant's internal app"]:::ext
    MCP["MCP server<br/>with SEP-3415 events"]:::ext
    POLL["MCP server<br/>without events"]:::ext
  end
  SUB["Subscription manager<br/>events/subscribe,<br/>renew before expiry"]:::srv
  PA["Poll adapter<br/>diff tools/resources"]:::srv
  subgraph ING["Ingest crate"]
    direction TB
    V1["HMAC-SHA256<br/>X-Hub-Signature-256"]:::sec
    V2["Slack signing secret v0<br/>5-minute window"]:::sec
    V3["Standard Webhooks<br/>signature"]:::sec
    NORM["Normalise to internal event<br/>map connection to tenant + scope"]:::srv
    DED["Insert with unique<br/>tenant_id, source, external_id"]:::srv
    ACK["2xx only after commit"]:::srv
  end
  EV[("events<br/>append only, taint=external")]:::store
  Q["Quarantine reader"]:::sec
  GH --> V1
  SL --> V2
  MCP --> V3
  SUB --> MCP
  POLL --> PA
  PA --> NORM
  V1 & V2 & V3 --> NORM
  NORM --> DED
  DED --> ACK
  DED --> EV
  EV --> Q
  classDef app fill:#dbeafe,stroke:#2563eb,color:#0b1b3a,stroke-width:1.5px
  classDef srv fill:#ede9fe,stroke:#6d28d9,color:#1e1338,stroke-width:1.5px
  classDef brain fill:#dcfce7,stroke:#15803d,color:#0b2a16,stroke-width:1.5px
  classDef store fill:#e5e7eb,stroke:#374151,color:#111827,stroke-width:1.5px
  classDef sec fill:#fee2e2,stroke:#b91c1c,color:#3b0a0a,stroke-width:1.5px
  classDef ext fill:#fef3c7,stroke:#b45309,color:#3a2504,stroke-width:1.5px
  classDef later fill:#f3f4f6,stroke:#6b7280,color:#374151,stroke-dasharray:5 3
  classDef gate fill:#ffffff,stroke:#111827,color:#111827,stroke-width:2px
```

- **Sources in v0:** a native adapter for every catalog plugin whose events we can take in directly (signed webhooks or push). GitHub (our GitHub App, one webhook secret) and Slack (each tenant's internal Slack app, its signing secret kept per tenant in the vault and matched by app and team id) come first, then Google Workspace (Drive and Calendar push), Linear, Notion, Sentry, Atlassian, HubSpot, Box, Asana, monday.com, Airtable, Attio, Intercom, Stripe, Vercel and Webflow, each checked against the vendor's webhook docs as it is built. Anything else connects as a custom event source: an MCP server with events (SEP-3415), the poll fallback, or a generic signed webhook per connection with its secret in the vault. Native adapters keep being added one by one.
- **SEP-3415 adapter:** speaks `events/list`, `events/subscribe`, `events/unsubscribe`, `events/poll` and `events/stream` with servers that support MCP Events; webhook delivery is the default mode; the subscription manager renews subscriptions before they expire.
- **Poll fallback:** for servers without events, the adapter polls the server's list tools or resources on a schedule and emits an event for each change it detects.
- **Ordering and duplicates:** delivery can repeat or arrive out of order; the unique key drops repeats, and the projector orders by the source's own timestamp or sequence where one exists.
- **Limits:** body size limits per source; processing is asynchronous after the commit, so webhooks are answered fast.
- **Taint:** every event row carries `taint = external`; anything derived from it inherits that taint (see Security model).
