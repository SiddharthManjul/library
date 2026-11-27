# 0012 The brand kit is the design source of truth

Status: accepted
Date: 2026-11-26

## Context
The Library brand kit (26 Nov 2026) supersedes the older design notes: Carbon Ice palette, Inter Tight, Inter and JetBrains Mono, a new shell and 16 screens.

## Decision
`BRAND.md` and `tokens.json` in the brand kit rule brand, theme, layout and screens; `tokens.json` wins on values. The kit is private and never committed. Only runtime assets (tokens, fonts, icons) are copied into `apps/desktop/assets/brand/`, and the CSS tokens come from them.

## Options considered
Commit the whole kit (about 62 MB of images, and it is private). Re-derive tokens by hand (drift).

## Consequences
The kit's designs show macOS shortcut glyphs and a few spec conflicts, listed in the README, which the app corrects as screens are built.
