# 0009 Platforms and packaging

Status: accepted
Date: 2026-11-26

## Context
Most companies run Windows, so Windows ships first. The free route avoids code-signing cost for v0.

## Decision
Windows 11 x64 first through an MSIX package published in the Microsoft Store, which signs and updates it at no cost; Linux x64 as `.deb` and AppImage with published checksums; macOS after v0. The local node is inside the package. CI builds the node and app natively on Windows and Linux; server and database tests run on Linux only, because hosted Windows runners cannot run Linux containers.

## Options considered
Direct-download installer signed with Azure Artifact Signing (about $10 a month, deferred). WSL-only development on Windows (does not test what members run).

## Consequences
MSIX redirects some writes and registry access for packaged processes. Provider runners are tested from inside the installed package.
