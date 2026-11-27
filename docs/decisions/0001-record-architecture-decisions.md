# 0001 Record architecture decisions

Status: accepted
Date: 2026-11-26

## Context
Agents and people hand work to each other across sessions. Reasons for a decision are lost unless they are written down where the next session will look.

## Decision
Every decision that changes architecture, a contract or a dependency family gets a numbered ADR in `docs/decisions/`. ADRs are never edited after acceptance; a new ADR supersedes an old one.

## Options considered
Keep decisions in the README only (fast, but no history and no options). Use an issue tracker (loses the link to the code).

## Consequences
Contracts change only with an ADR in the same pull request, checked by a pre-commit hook.
