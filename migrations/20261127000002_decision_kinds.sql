-- P1-07: decision log kinds. The harness, relay, work and brain crates record these; an unknown kind is refused
-- so the v1 decision API is trained on a closed, documented set. Gap 7 of the build guide added relay
-- assignment and change-detection triggers to the README's list.
ALTER TABLE decision_log ADD CONSTRAINT decision_log_kind_known CHECK (kind IN (
  'routing',            -- which model source ran a step
  'decomposition',      -- single agent or fan-out, with B, S and W
  'approval_tier',      -- which tier a tool call got and whether a person was asked
  'lease',              -- an artifact lease granted, denied, renewed or expired
  'advisor_call',       -- the executor asked the advisor
  'relay_assignment',   -- which node took a relay job
  'change_trigger'      -- change detection decided a summary or brief needed a rewrite
));
