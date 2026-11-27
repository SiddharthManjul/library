# 0004 Device keys: X25519 for sealing, Ed25519 for signing

Status: accepted
Date: 2026-11-26

## Context
Relay envelopes are sealed to one device and relay results are signed by it. Registration must prove the device holds its keys without sending them.

## Decision
Each device generates an X25519 key and an Ed25519 key at install. Both private keys live only in the OS keychain (Windows Credential Manager, Linux Secret Service), loaded to use and zeroised after. Registration sends both public keys and an Ed25519 signature over a domain-tagged message binding the tenant, the user and the X25519 key. `devices` stores both public keys per tenant.

## Options considered
One Ed25519 key converted for sealing (couples two jobs to one key). Server-generated keys (the server would hold device secrets).

## Consequences
The keychain protects keys per OS user; a compromised account can read them. Revoking a device ends its sessions and stops key issue.
