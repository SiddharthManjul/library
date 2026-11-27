# 0007 Change-bound summaries and briefs

Status: accepted
Date: 2026-11-26

## Context
Model work on a timer costs tokens on quiet days and gives nothing.

## Decision
Community summaries and briefs are rewritten only when change detection scores enough change (10 points, or a decision change after 15 quiet minutes). Time-based rules that cost no tokens, such as ambient trigger checks and the 24-hour snapshot rule, are allowed.

## Options considered
Nightly rebuilds (simple, wasteful, stale during the day).

## Consequences
Quiet periods cost nothing and the brief shows the time of its last material change.
