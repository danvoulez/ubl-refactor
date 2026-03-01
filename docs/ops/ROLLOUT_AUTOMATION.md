# Rollout P0 -> P1 Automation

**Status**: active  
**Mode**: chip-native governance

## Canonical automation path

Rollout governance is now recorded and enforced inside UBL through chip submission and receipt verification.

Required artifacts:

- proposal chip receipt CID
- activation/approval chip receipt CID
- trace for each rollout decision
- external witness for activation milestone

## What changed

- External preflight script `scripts/rollout_p0_p1_check.sh` was retired from the operational path.
- `make rollout-check` was removed.
- Production decision authority remains CHECK/TR/WF + receipt chain.

## Break-glass rule

Break-glass is still allowed only as a runtime-operational emergency path and must generate a dedicated receipt/witness trail.

## Pointers

- Rollout model: `ROLLOUT_P0_TO_P1.md`
- Forever bootstrap flow: `docs/ops/FOREVER_BOOTSTRAP.md`
