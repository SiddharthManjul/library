---
name: add-eval-case
description: Add an eval case to a suite with a grader and its effect on the threshold.
---

# Add eval case

1. Pick the suite: harness, brain, ambient or security. Start from a real failure.
2. Grade the outcome, not the steps: tests for code, schemas for structured output, a rubric with calibrated grader for design and marketing.
3. Report pass^k: security k=5 and every case passes; brain k=3; harness k=3 (0.70, rising to 0.80 after launch); ambient k=3.
4. Store the case under `evals/<suite>/` and any corpus under `evals/corpus/`.
5. A drop of more than 5 points from the last release blocks the release. Never lower a threshold to pass.
