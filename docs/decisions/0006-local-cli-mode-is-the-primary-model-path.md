# 0006 Local CLI mode is the primary model path

Status: accepted
Date: 2026-11-26

## Context
Library IDE ships no models in v0. Members already pay for coding-agent subscriptions.

## Decision
The node starts each provider's official CLI on the member's machine. Library never reads or forwards a subscription's login token. API keys work alone or alongside. Brain work runs through the model relay on members' CLIs in locked mode.

## Options considered
Resell model usage (we pay, margins are thin). Read subscription tokens directly (breaks provider terms).

## Consequences
Every job type must also run on an API key, because provider rules for programmatic use of subscriptions change.
