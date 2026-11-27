# 0010 Provider runners in Rust, with no fork of T3 Code

Status: accepted
Date: 2026-11-26

## Context
T3 Code is a Node server plus Electron client built for remote and mobile access. We replace the server with a Rust node and redesign the whole UI, so little of the fork would remain, and its trademark rules require rebranding anyway.

## Decision
The desktop app is written here, not forked. T3 Code (MIT) is a reference for provider adapters and event formats only. The adapters are rewritten as Rust runners in the local node, one local process per machine, no listening port. Any ported code keeps its MIT notice.

## Options considered
Fork and strip (carries upstream merges, trademark scrubbing and remote-access code). Keep T3's Node server beside the Rust node (two local processes and two adapter sets).

## Consequences
We maintain our own Electron shell and thread UI, and track provider CLI changes ourselves.
