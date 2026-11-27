---
name: add-provider-runner
description: Add a provider runner in the node for one official CLI, with locked mode and permission mapping.
---

# Add provider runner

1. Start the official CLI as a child process with an explicit argument list. Never build a shell string. On Windows resolve `.cmd` and `.ps1` shims or start them through `cmd.exe` with strict quoting. Never read the CLI's login files.
2. Stream its events into the common run event type; support resume on the same machine; report usage to the meter.
3. Coding runs: start in the most restrictive permission mode, keep the CLI's own MCP servers off unless the member allowed them, map its permission requests to Library approvals, apply standing rules, use the provider's OS sandbox where one exists.
4. Locked mode for relay jobs: every tool, MCP server, file and web access off, in an empty temp folder. Record the exact flags per platform in the runner's `AGENTS.md`.
5. The runner is relay-eligible only after the canary passes on Windows and Linux: a job that asks the model to read a file, run a command, fetch a URL and call a tool must produce no tool events, touch nothing outside the temp folder and return only schema-valid fields.
6. T3 Code's adapters are a reference only. Any code ported keeps its MIT notice.
