# 0011 Coding agents' own tools need the member's permission

Status: accepted
Date: 2026-11-26

## Context
Coding agents (Claude Code, Codex and others) keep their own shell, network, git and MCP tools, which do not pass through the dispatcher.

## Decision
Inside the task's worktree an agent reads and edits freely, and edits reach the member as a diff. Everything else (shell, network, its own MCP servers, installs, push, pull requests, files outside the worktree) is off until the member allows it, once or by a standing rule per project. In a tainted run, pushes, pull requests and outside network calls ask every time. Runners use the strictest permission mode and the provider's OS sandbox where one exists. Relay jobs never get tools.

## Options considered
Run everything through the dispatcher (breaks the coding agents' quality). Allow everything (an injected repository file could push code).

## Consequences
Claude Code has no sandbox on native Windows, so its permission checks are the only gate there, and the app says so.
