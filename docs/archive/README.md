# Archive

## dev-plan-v2-groups-1-29.md

The old development plan (Groups 1–29, v0.1.0–v2.7.0) is preserved in git history.
To view: `git log --oneline docs/dev-plan.md` — commits up to `1b9cb0d` contain the old plan.

The plan was replaced on 2026-03-21 with `docs/dev-plan.md` (Groups A–J, v3.0 audit-driven rewrite)
after a senior technical audit identified broken implementations, dead code, and scope creep.

## dev-plan-v3.0-groups-A-J.md

The v3.0 plan (Groups A–J) is also superseded. All groups A–J confirmed complete as of 2026-03-25.
The current active plan is v3.1 (Groups P–V) in `docs/dev-plan.md`, targeting stabilization:
embed security gap, IR layer, backend decision, and test restructuring.

### Summary of what was completed in the old plan (Groups 1–29)

- Groups 1–22: Core language, stdlib, tooling, security, WASM, async — implemented
- Groups 23–29 (v2.1–v2.7): Infra, type hardening, security hardening, stdlib expansion,
  interactive tooling, WASM completion — partially implemented (see audit findings in new plan)

### Why the old plan was replaced

1. Several tasks marked `[x]` were partially broken (register_fn, registry server)
2. Cargo.toml version never updated (showed 0.5.0 while feature set was v2.7)
3. Scope included deployment/ecosystem concerns not relevant to language correctness
4. Groups 28–29 had missing subtasks not captured in the plan
