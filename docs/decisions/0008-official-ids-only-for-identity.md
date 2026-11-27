# 0008 Official ids only for identity

Status: accepted
Date: 2026-11-26

## Context
Names and aliases collide. Acting on a fuzzy match could message or grant access to the wrong person.

## Decision
Entities attach automatically only on ids issued by the source system. Aliases and fuzzy matches create merge proposals a person confirms. Every action runs on official ids.

## Options considered
Embedding-based auto-merge (higher recall, unsafe for permissions).

## Consequences
Some duplicates stay until a lead confirms a merge.
