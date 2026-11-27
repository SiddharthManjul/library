---
name: add-screen
description: Build a screen from its design using brand tokens, all states and accessibility checks.
---

# Add screen

1. Read the design in the brand kit (`library-brand-kit/04-screens`, sources in `07-source`) and write `docs/design/screens/<screen>.md`: phase, purpose, layout, data, actions, states, keyboard.
2. Use tokens only: `apps/desktop/src/styles/tokens.css` variables. No raw colours, font sizes or radii in components.
3. Build every state: loading, empty, error with retry, offline (replica only), sealed, no permission, waiting for approval.
4. Every agent answer shows source chips; every pending action shows an exact preview; reversible actions offer undo.
5. Keyboard reachable with a visible focus ring; contrast at least 4.5:1 in all three themes; labelled controls; respect reduced motion.
6. Data comes through the node bridge only. The renderer never holds a token or opens a network connection.
7. Shortcuts use Ctrl on Windows and Linux. Sample content in designs is sample.
