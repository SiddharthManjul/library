---
name: add-skill-pack
description: Add a department skill pack with its tool allowlist, schemas and evaluator criteria.
---

# Add skill pack

1. Create `skills/<department>/SKILL.md` of about 100 lines: purpose, rules, when to use, links to deeper references. Only name and description sit in the prompt; the body loads on use.
2. Add the tool allowlist by registry id, output schemas, and evaluator criteria with calibration examples.
3. Link the department's playbook; playbook items are small ops (add, counter, deprecate), never one rewritten document.
4. Version the pack. Each run records the pack hash.
5. Add harness eval cases for the pack with `add-eval-case`.
