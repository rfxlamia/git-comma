# Configurable Max Chars

**Date:** 2026-05-13
**Status:** approved
**Author:** brainstorm session
**Spec path:** docs/pocket/spec/2026-05-13-configurable-max-chars/max-chars.md

---

## Summary

The diff size limit (`SOFT_DIFF_LIMIT`) is currently hardcoded at 15,000 chars in `preflight.rs`. Users hit this limit on real commits and have no way to adjust it without editing source code. This feature adds `max_chars` to the config, prompts for it during `--setup`, and uses it in preflight checks.

---

## Context

### Current State
- `SOFT_DIFF_LIMIT = 15_000` hardcoded in `preflight.rs:3`
- `Config` struct only has `api_key` and `model_id`
- `run_with_diff_bypass()` skips the check entirely (all-or-nothing)
- `--setup` prompts for API key and model only

### Problem / Motivation
The 15k char limit is arbitrary and too low for many real commits. Users hit `Diff too large: 19273 chars` on normal workflows. There's no way to tune it — only bypass entirely via the confirmation prompt.

### Related Areas
- `src/config.rs` — Config struct, load/save
- `src/setup.rs` — setup flow prompts
- `src/preflight.rs` — diff size check
- `src/main.rs` — passes config to preflight

---

## Scope

### In-Scope
- Add `max_chars` field to Config with `#[serde(default = "default_max_chars")]`
- Prompt for max_chars during `--setup` (after model selection, with current value as default)
- Enforce minimum value of 1
- Pass `max_chars` from config into preflight as a parameter
- Existing configs without `max_chars` load with default 15,000

### Out-of-Scope
- `--max-chars` CLI flag
- Per-project config (`.comma.json` in repo root)
- Dynamic suggestions on failure

---

## Architecture Constraints

- Layers touched: config.rs, setup.rs, preflight.rs, main.rs
- Layers NOT touched: ai.rs, openrouter.rs, tui.rs, prompt.rs, sanitization.rs
- Patterns: serde defaults, atomic save, existing inquire prompt style
- Architecture validation: PASS

---

## Stories + Scenarios

### Story: Configurable diff limit
> As a comma user, I want to set my preferred max diff char limit during setup, so that I'm not blocked by an arbitrary hardcoded limit.

**Rule 1: max_chars defaults to 15,000**
- Example A: New config → max_chars = 15000
- Example B: Existing config without field → max_chars = 15000 (serde default)

```gherkin
Scenario: New user runs --setup
  Given  user has no config
  When   --setup completes
  Then   config contains max_chars with value 15000 (or user-chosen value)

Scenario: Existing config without max_chars field
  Given  ~/.comma.json has api_key and model_id only
  When   config is loaded
  Then   max_chars defaults to 15000
```

**Rule 2: max_chars must be >= 1**
- Example A: User enters 0 → error, prompt repeats
- Example B: User enters 1 → accepted

```gherkin
Scenario: User sets max_chars below minimum
  Given  user is prompted for max chars
  When   user enters 0
  Then   error: "Must be at least 1"
  And    prompt repeats

Scenario: User sets max_chars to 1
  Given  user is prompted for max chars
  When   user enters 1
  Then   value accepted
```

**Rule 3: max_chars prompted during --setup**
- Example A: First run → prompted with default 15000
- Example B: Re-run setup → prompted with current config value

```gherkin
Scenario: First run setup
  Given  no existing config
  When   --setup runs past model selection
  Then   user sees: "Max characters for diff (default: 15000):"
  And    pressing enter sets 15000

Scenario: Re-run setup preserves current value
  Given  config has max_chars=20000
  When   --setup runs
  Then   user sees: "Max characters for diff (current: 20000):"
  And    pressing enter keeps 20000
```

**Rule 4: preflight uses config.max_chars**
- Example A: config.max_chars=30000, diff=25000 → passes
- Example B: config.max_chars=10000, diff=15000 → DiffTooLarge error

```gherkin
Scenario: Diff within configured limit
  Given  config has max_chars=30000
  When   preflight checks a 25000-char diff
  Then   check passes

Scenario: Diff exceeds configured limit
  Given  config has max_chars=10000
  When   preflight checks a 15000-char diff
  Then   PreflightError::DiffTooLarge { size: 15000 }
```

---

## Acceptance Criteria

```
Rule: max_chars default
  ✓ Given new config, When setup completes, Then max_chars=15000
  ✓ Given existing config without max_chars, When loaded, Then defaults to 15000

Rule: max_chars validation
  ✓ Given user enters 1, When validated, Then accepted
  ✗ Given user enters 0, When validated, Then error "Must be at least 1"

Rule: setup prompt
  ✓ Given first run, When model selected, Then prompted for max_chars with default 15000
  ✓ Given re-run with max_chars=20000, When prompted, Then shows current 20000

Rule: preflight uses config
  ✓ Given max_chars=30000, diff=25000, When preflight runs, Then passes
  ✓ Given max_chars=10000, diff=15000, When preflight runs, Then DiffTooLarge
```

---

## Design Decision

**Chosen option:** Option A — Pass limit as parameter

**Summary:** Add `limit: usize` param to `preflight::run_with_filter()`. main.rs reads `config.max_chars` and passes it. No global state.

**Rejected options:**
- Option B (module-level static): rejected because hidden global state is harder to test and less explicit

**Key tradeoffs accepted:**
- Minor signature change to preflight functions (acceptable, already has FilterMode param)

---

## Open Questions / Assumptions

All questions resolved during discovery.

---

## Rollback

- Revert the commit — no data migration, no breaking changes
- Old configs without max_chars still work (serde default handles it)
