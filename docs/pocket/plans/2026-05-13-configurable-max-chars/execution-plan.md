# EXECUTION PLAN — Configurable Max Chars

**Date:** 2026-05-13
**Spec:** docs/pocket/spec/2026-05-13-configurable-max-chars/max-chars.md
**Status:** approved
**Total tasks:** 3

---

### Test-Architect Summary
Tasks enriched: 3
Integration test tasks added: 0 (all tests are unit-level — no cross-unit GWT scenarios)
TDD order corrections made: 0 (all tasks already had correct order)
Test framework used: cargo test (Rust built-in), tempfile crate for fixtures
Coverage areas:
  - T1: Config serialization/deserialization with serde default, backward compat for missing field
  - T2: Validation function (validate_max_chars_input) — rejects 0, accepts 1, empty→default, rejects non-numeric
  - T3: Diff size check function (check_diff_size) — within limit, exceeds limit, exact boundary
  - Not tested: interactive inquire prompts (require TUI), full preflight git integration (requires git repo fixture)

---

## Execution Overview

### Recommended Order
```
T1 → T2, T3 (parallel)
```

> Dependency order above is **recommended** — pocket skill enforces actual
> parallelism and sequencing based on its routing logic.

### Parallelizable Groups
| Group | Tasks | Unblocked After |
|-------|-------|-----------------|
| Group A | T2, T3 | T1 completes |

### Constraints Reminder
**Architecture:** Only touch config.rs, setup.rs, preflight.rs, main.rs. NEVER touch ai.rs, openrouter.rs, tui.rs, prompt.rs, sanitization.rs. Use serde defaults, atomic save, existing inquire prompt style.
**Out-of-scope:** `--max-chars` CLI flag, per-project config (`.comma.json` in repo root), dynamic suggestions on failure.
**Assumptions at risk:** None — all open questions resolved during discovery.
**Sequencing:** T2 and T3 are independent after T1. Both only read from Config (T1's output). They modify different files with no overlap.

### File Structure Map

```
Rule: max_chars default
  Modify: src/config.rs                    (by: T1)
  Modify: src/setup.rs                     (by: T1 — compile fix only, T2 replaces)
  Test:   tests/config_tests.rs            (by: T1 — new tests + compile fixes to existing)

Rule: max_chars validation
  Modify: src/setup.rs                     (by: T2 — prompt + validation fn)
  Test:   tests/config_tests.rs            (by: T2)

Rule: setup prompt
  Modify: src/setup.rs                     (by: T2)
  Test:   tests/config_tests.rs            (by: T2)

Rule: preflight uses config
  Modify: src/preflight.rs                 (by: T3)
  Modify: src/main.rs                      (by: T3)
  Test:   tests/config_tests.rs            (by: T3)
```

---

## Pocket Packets

---

### Task 1: Add max_chars field to Config struct [prereq]

## OBJECTIVE
Add `max_chars: usize` field to Config struct with `#[serde(default = "default_max_chars")]` so new configs get 15,000 and existing configs without the field also load with 15,000.

Files:
- Modify: `src/config.rs`
- Modify: `src/setup.rs` (compile fix only — add temporary `max_chars: 15_000` to Config literal)
- Test: `tests/config_tests.rs`

Steps:
1. Write failing test for: max_chars defaults to 15000
   File: `tests/config_tests.rs`
   Test code:
   ```rust
   #[test]
   fn test_config_max_chars_defaults_to_15000() {
       let tmp = TempDir::new().unwrap();
       let path = tmp.path().join("comma.json");
       // Write JSON without max_chars field (simulates existing config)
       std::fs::write(&path, r#"{"api_key":"sk-or-v1-test","model_id":"test/model"}"#).unwrap();
       let loaded = git_comma::config::Config::load_from_path(&path).unwrap();
       assert_eq!(loaded.max_chars, 15_000);
   }

   #[test]
   fn test_config_max_chars_round_trip() {
       let tmp = TempDir::new().unwrap();
       let path = tmp.path().join("comma.json");
       let cfg = git_comma::config::Config {
           api_key: "sk-or-v1-test".to_string(),
           model_id: "test/model".to_string(),
           max_chars: 20_000,
       };
       cfg.save(&path).unwrap();
       let loaded = git_comma::config::Config::load_from_path(&path).unwrap();
       assert_eq!(loaded.max_chars, 20_000);
   }

   #[test]
   fn test_config_max_chars_appears_in_saved_json() {
       let tmp = TempDir::new().unwrap();
       let path = tmp.path().join("comma.json");
       let cfg = git_comma::config::Config {
           api_key: "sk-or-v1-test".to_string(),
           model_id: "test/model".to_string(),
           max_chars: 15_000,
       };
       cfg.save(&path).unwrap();
       let json = std::fs::read_to_string(&path).unwrap();
       assert!(json.contains("\"max_chars\""), "saved JSON should contain max_chars field");
       assert!(json.contains("15000"), "saved JSON should contain max_chars value");
   }
   ```
   Test verifies:
   - Given a JSON string without max_chars field, When loaded via Config::load_from_path, Then max_chars = 15000
   - Given a Config with max_chars=20000, When saved and loaded, Then max_chars = 20000

2. Run test — verify FAIL:
   `cargo test config_tests`
   Expected failure: compilation error — `max_chars` field doesn't exist on Config

3. Implement minimal code to satisfy the test:
   File: `src/config.rs`
   Implement:
   - Add `fn default_max_chars() -> usize { 15_000 }` function
   - Add `#[serde(default = "default_max_chars")]` attribute above new field
   - Add `pub max_chars: usize` field to Config struct

   File: `src/setup.rs` (compile fix)
   Implement:
   - Add `max_chars: 15_000` to the Config struct literal at line ~105 (temporary value — T2 will replace with prompt-derived value)

   File: `tests/config_tests.rs` (compile fix)
   Implement:
   - Add `max_chars: 15_000` to existing Config struct literals in `test_config_load_preserves_api_key_field` and `test_save_config_atomic`

4. Run test — verify PASS:
   `cargo test config_tests`
   Expected: PASS (all existing tests + new tests pass)

5. Commit:
   `git add src/config.rs src/setup.rs tests/config_tests.rs`
   `git commit -m "feat(config): add max_chars field with serde default 15000"`

## REFERENCES LOADED
docs/pocket/spec/2026-05-13-configurable-max-chars/max-chars.md — rule: max_chars default, GWT: "Given new config, When setup completes, Then max_chars=15000" and "Given existing config without max_chars, When loaded, Then defaults to 15000"
src/config.rs — current Config struct has only api_key + model_id, derives Serialize/Deserialize, uses atomic save pattern
tests/config_tests.rs — existing tests use tempfile::TempDir, test round-trip save/load, test malformed JSON

## WHY THIS AGENT
agent: general-purpose
model: haiku
Justification: Single file modification, clear spec, no judgment needed — add one field with serde attribute

## SANDWICH CONTEXT
[CRITICAL: Config must remain backward-compatible — existing ~/.comma.json files without max_chars must load without error]
You are implementing max_chars field addition for Configurable Max Chars.
Spec: docs/pocket/spec/2026-05-13-configurable-max-chars/max-chars.md
Design decision: Option A — pass limit as parameter (Config holds the value, preflight receives it)
Files in scope: src/config.rs, src/setup.rs (compile fix only), tests/config_tests.rs — no other files
Test framework: cargo test, tempfile crate for fixtures
Available after: none (prereq)
Architecture rule: Config layer only — setup.rs change is a minimal compile fix (add `max_chars: 15_000`), not feature work. T2 will replace with prompt-derived value.
[RESTATE: Config must remain backward-compatible — existing ~/.comma.json files without max_chars must load without error]

## DELIVERABLE
Verification — task is DONE when all pass:

Given a new Config with api_key and model_id, When serialized to JSON and deserialized, Then max_chars equals 15000
Given a JSON file without max_chars field, When loaded via Config::load_from_path, Then max_chars defaults to 15000
Given a JSON file with max_chars=20000, When loaded, Then max_chars equals 20000

All tests PASS. Commit exists with message matching `feat(config): add max_chars field with serde default 15000`.

Format: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED

## QUALITY BAR
Must-have:
  - max_chars field with #[serde(default = "default_max_chars")]
  - default_max_chars() returns 15_000
  - Existing configs without max_chars load successfully (backward compat)
  - Tests written BEFORE implementation (TDD — not after)
  - Commit message follows conventional commits format

Must-not-have:
  - Changes to preflight.rs, main.rs, or any other file beyond compile fixes
  - Hardcoded 15_000 anywhere except the default function and temporary compile fixes
  - Breaking existing Config load/save behavior

Open question risks:
  — None — all resolved

Rollback note:
  - Revert the commit — no data migration, old configs still work

## STOP CONDITIONS
Done when: all DELIVERABLE scenarios pass, tests green, commit created
Uncertain when: serde default behavior doesn't work as expected (unlikely)
Escalate when: changes touch files outside listed scope

---

### Task 2: Add max_chars prompt during --setup [depends: T1]

## OBJECTIVE
Add an inquire prompt during --setup that asks user for max_chars after model selection. Show current value (or 15000 for first run) as default. Validate that entered value is >= 1, re-prompting on failure.

Files:
- Modify: `src/setup.rs`
- Test: `tests/config_tests.rs`

Steps:
1. Write failing test for: max_chars validation rejects 0, accepts 1
   File: `tests/config_tests.rs`
   Test code:
   ```rust
   #[test]
   fn test_validate_max_chars_rejects_zero() {
       let result = git_comma::setup::validate_max_chars_input("0");
       assert!(result.is_err());
       assert_eq!(result.unwrap_err(), "Must be at least 1");
   }

   #[test]
   fn test_validate_max_chars_accepts_one() {
       let result = git_comma::setup::validate_max_chars_input("1");
       assert_eq!(result.unwrap(), 1);
   }

   #[test]
   fn test_validate_max_chars_accepts_empty_as_default() {
       let result = git_comma::setup::validate_max_chars_input("");
       assert_eq!(result.unwrap(), 15_000); // keeps default
   }

   #[test]
   fn test_validate_max_chars_rejects_non_numeric() {
       let result = git_comma::setup::validate_max_chars_input("abc");
       assert!(result.is_err());
   }
   ```
   Note: Extract `pub fn validate_max_chars_input(input: &str) -> Result<usize, String>` from setup.rs to make validation testable. This function parses input, validates >= 1, returns error message on failure. Empty string returns default 15000.
   Test verifies:
   - Given input "0", When validated, Then error "Must be at least 1"
   - Given input "1", When validated, Then returns 1
   - Given empty input, When validated, Then returns 15000 (default)
   - Given input "abc", When validated, Then error (non-numeric)

2. Run test — verify FAIL:
   `cargo test config_tests`
   Expected failure: compilation error — `validate_max_chars_input` function doesn't exist yet

3. Implement minimal code to satisfy the test:
   File: `src/setup.rs`
   Implement:
   - Change existing config loading to keep full `Config` (not just `api_key`) so `max_chars` is available for re-run default display:
     ```rust
     let existing_config = if !is_first_run {
         home_config_path().ok().and_then(|path| Config::load_from_path(&path).ok())
     } else { None };
     let existing_key = existing_config.as_ref().map(|c| c.api_key.clone());
     let existing_max_chars = existing_config.as_ref().map(|c| c.max_chars);
     ```
   - Add `pub fn validate_max_chars_input(input: &str) -> Result<usize, String>` that:
     - Returns Ok(15_000) for empty input
     - Parses input as usize, returns Err("Must be at least 1") if < 1
     - Returns Err for non-numeric input
   - After model selection in run_setup_flow(), add inquire::Text prompt: "Max characters for diff (current: {value}):"
   - For first run, show default 15000. For re-run, show existing_max_chars (or 15000 if config missing)
   - Use validate_max_chars_input in a loop, re-prompt on error
   - Replace the temporary `max_chars: 15_000` (from T1) with the prompt-derived value

4. Run test — verify PASS:
   `cargo test config_tests`
   Expected: PASS

5. Commit:
   `git add src/setup.rs tests/config_tests.rs`
   `git commit -m "feat(setup): add max_chars prompt with validation"`

## REFERENCES LOADED
docs/pocket/spec/2026-05-13-configurable-max-chars/max-chars.md — rule: max_chars validation + setup prompt, GWT: "Given user enters 0, When validated, Then error" and "Given first run, When model selected, Then prompted for max_chars with default 15000"
src/setup.rs — uses inquire crate for prompts, run_setup_flow(is_first_run: bool) returns Config, existing prompt pattern: inquire::Password and ui::model_select_prompt

## WHY THIS AGENT
agent: general-purpose
model: haiku
Justification: Single file modification, clear inquire prompt pattern already established in setup.rs

## SANDWICH CONTEXT
[CRITICAL: Must use inquire crate for prompts — matches existing setup flow style]
You are implementing max_chars setup prompt for Configurable Max Chars.
Spec: docs/pocket/spec/2026-05-13-configurable-max-chars/max-chars.md
Design decision: Option A — pass limit as parameter (setup builds Config with max_chars)
Files in scope: src/setup.rs, tests/config_tests.rs — no other files
Test framework: cargo test, tempfile crate for fixtures
Available after: T1 (Config struct must have max_chars field)
Architecture rule: Setup layer only — do NOT touch config.rs, preflight.rs, or main.rs in this task
[RESTATE: Must use inquire crate for prompts — matches existing setup flow style]

## DELIVERABLE
Verification — task is DONE when all pass:

Given a first run with no config, When --setup runs past model selection, Then user sees "Max characters for diff" prompt with default 15000
Given a re-run with max_chars=20000, When --setup runs, Then prompt shows current 20000
Given user enters 0, When validated, Then error "Must be at least 1" and prompt repeats
Given user enters 1, When validated, Then value accepted
Given user presses enter without input, When prompt completes, Then default/current value is kept

All tests PASS. Commit exists with message matching `feat(setup): add max_chars prompt with validation`.

Format: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED

## QUALITY BAR
Must-have:
  - Prompt appears after model selection in setup flow
  - Shows "current: {value}" for re-runs, "default: 15000" for first run
  - Validates >= 1, re-prompts on failure with error message
  - Empty input preserves current/default value
  - Uses inquire crate (matches existing pattern)
  - Tests written BEFORE implementation (TDD — not after)

Must-not-have:
  - Changes to config.rs, preflight.rs, main.rs, or any other file
  - Custom prompt library (must use inquire)
  - CLI flag handling (--max-chars is out of scope)

Open question risks:
  — None

Rollback note:
  - Revert the commit — setup flow reverts to api_key + model_id only

## STOP CONDITIONS
Done when: all DELIVERABLE scenarios pass, tests green, commit created
Uncertain when: inquire API doesn't support the prompt pattern needed
Escalate when: changes touch files outside listed scope

---

### Task 3: Pass max_chars from config into preflight [depends: T1]

## OBJECTIVE
Change preflight::run_with_filter() and preflight::run() to accept a `limit: usize` parameter. Replace the hardcoded SOFT_DIFF_LIMIT with this parameter. Update main.rs to pass config.max_chars to all preflight calls.

Files:
- Modify: `src/preflight.rs`
- Modify: `src/main.rs`
- Test: `tests/config_tests.rs`

Steps:
1. Write failing test for: preflight uses passed limit instead of hardcoded constant
   File: `tests/config_tests.rs`
   Test code:
   ```rust
   #[test]
   fn test_check_diff_size_within_limit() {
       let diff = "a".repeat(25_000);
       let result = git_comma::preflight::check_diff_size(&diff, 30_000);
       assert!(result.is_ok());
   }

   #[test]
   fn test_check_diff_size_exceeds_limit() {
       let diff = "a".repeat(15_000);
       let result = git_comma::preflight::check_diff_size(&diff, 10_000);
       assert!(result.is_err());
       match result.unwrap_err() {
           git_comma::preflight::PreflightError::DiffTooLarge { size } => {
               assert_eq!(size, 15_000);
           }
           other => panic!("Expected DiffTooLarge, got: {:?}", other),
       }
   }

   #[test]
   fn test_check_diff_size_exact_limit() {
       let diff = "a".repeat(10_000);
       let result = git_comma::preflight::check_diff_size(&diff, 10_000);
       assert!(result.is_ok()); // exactly at limit passes
   }
   ```
   Note: Extract `pub fn check_diff_size(diff: &str, limit: usize) -> Result<(), PreflightError>` from preflight.rs to make the size check testable in isolation. This function contains the `diff.len() > limit` check and returns DiffTooLarge on failure.
   Test verifies:
   - Given diff=25000 chars, limit=30000, When checked, Then passes
   - Given diff=15000 chars, limit=10000, When checked, Then DiffTooLarge { size: 15000 }
   - Given diff=10000 chars, limit=10000, When checked, Then passes (exact boundary)

2. Run test — verify FAIL:
   `cargo test config_tests`
   Expected failure: compilation error — `check_diff_size` function doesn't exist yet

3. Implement minimal code to satisfy the test:
   File: `src/preflight.rs`
   Implement:
   - Add `pub fn check_diff_size(diff: &str, limit: usize) -> Result<(), PreflightError>` that returns Ok(()) if diff.len() <= limit, else Err(PreflightError::DiffTooLarge { size: diff.len() })
   - Change `pub fn run_with_filter(mode: FilterMode)` to `pub fn run_with_filter(mode: FilterMode, limit: usize)`
   - Change `pub fn run()` to `pub fn run(limit: usize)`
   - Replace inline `diff_content.len() > SOFT_DIFF_LIMIT` with call to `check_diff_size(&diff_content, limit)?`
   - Update `run_with_diff_bypass()` to accept `limit: usize` and pass it through
   - Remove `SOFT_DIFF_LIMIT` constant (now unused)

   File: `src/main.rs`
   Implement:
   Pass `config.max_chars` to all preflight call sites (6 total):
   1. `preflight::run()` → `preflight::run(config.max_chars)`  (smart filter path, initial check)
   2. `preflight::run_with_filter(preflight::FilterMode::NoFilter)` → `preflight::run_with_filter(preflight::FilterMode::NoFilter, config.max_chars)`  (no-filter path, initial check)
   3. `preflight::run_with_filter(preflight::FilterMode::NoFilter)` → `preflight::run_with_filter(preflight::FilterMode::NoFilter, config.max_chars)`  (no-filter path, after git add retry)
   4. `preflight::run_with_filter(preflight::FilterMode::NoFilter)` → `preflight::run_with_filter(preflight::FilterMode::NoFilter, config.max_chars)`  (no-filter path, after diff-too-large confirm)
   5. `preflight::run()` → `preflight::run(config.max_chars)`  (smart filter path, after git add retry)
   6. `preflight::run_with_diff_bypass()` → `preflight::run_with_diff_bypass(config.max_chars)`  (smart filter path, after diff-too-large confirm)

4. Run test — verify PASS:
   `cargo test config_tests`
   Expected: PASS

5. Commit:
   `git add src/preflight.rs src/main.rs tests/config_tests.rs`
   `git commit -m "feat(preflight): use config.max_chars instead of hardcoded limit"`

## REFERENCES LOADED
docs/pocket/spec/2026-05-13-configurable-max-chars/max-chars.md — rule: preflight uses config, GWT: "Given max_chars=30000, diff=25000, When preflight runs, Then passes" and "Given max_chars=10000, diff=15000, When preflight runs, Then DiffTooLarge"
src/preflight.rs — run_with_filter(mode: FilterMode) currently uses SOFT_DIFF_LIMIT = 15_000, run() delegates to run_with_filter(Smart), run_with_diff_bypass() handles DiffTooLarge
src/main.rs — calls preflight::run() and preflight::run_with_filter(FilterMode::NoFilter), handles DiffTooLarge with ui::confirm_large_diff

## WHY THIS AGENT
agent: general-purpose
model: haiku
Justification: Two files but straightforward signature change + wiring — no judgment needed

## SANDWICH CONTEXT
[CRITICAL: preflight.rs must not import config.rs — limit is passed as parameter, not read from config directly]
You are implementing preflight limit parameter for Configurable Max Chars.
Spec: docs/pocket/spec/2026-05-13-configurable-max-chars/max-chars.md
Design decision: Option A — pass limit as parameter (no global state, no module-level static)
Files in scope: src/preflight.rs, src/main.rs, tests/config_tests.rs — no other files
Test framework: cargo test, tempfile crate for fixtures
Available after: T1 (Config struct must have max_chars field)
Architecture rule: preflight must NOT import config — limit flows through function parameters only. Layers NOT touched: ai.rs, openrouter.rs, tui.rs, prompt.rs, sanitization.rs.
[RESTATE: preflight.rs must not import config.rs — limit is passed as parameter, not read from config directly]

## DELIVERABLE
Verification — task is DONE when all pass:

Given config.max_chars=30000, When preflight checks a 25000-char diff, Then check passes (no error)
Given config.max_chars=10000, When preflight checks a 15000-char diff, Then PreflightError::DiffTooLarge { size: 15000 }
Given main.rs runs preflight, When config is loaded, Then config.max_chars is passed to preflight functions
[must-not] Given preflight.rs is examined, When imports are checked, Then crate::config is NOT imported

All tests PASS. Commit exists with message matching `feat(preflight): use config.max_chars instead of hardcoded limit`.

Format: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED

## QUALITY BAR
Must-have:
  - run_with_filter() accepts limit: usize parameter
  - run() accepts limit: usize parameter
  - SOFT_DIFF_LIMIT replaced by limit parameter in diff size check
  - main.rs passes config.max_chars to all preflight calls
  - Tests written BEFORE implementation (TDD — not after)
  - Commit message follows conventional commits format

Must-not-have:
  - preflight.rs importing crate::config (limit passed as param)
  - Changes to ai.rs, openrouter.rs, tui.rs, prompt.rs, sanitization.rs
  - Global state or module-level static for the limit
  - Hardcoded 15_000 remaining in diff size check

Open question risks:
  — None

Rollback note:
  - Revert the commit — preflight reverts to SOFT_DIFF_LIMIT constant

## STOP CONDITIONS
Done when: all DELIVERABLE scenarios pass, tests green, commit created
Uncertain when: preflight signature change breaks other callers unexpectedly
Escalate when: preflight.rs imports config.rs, or changes touch files outside listed scope

---

## Plan Summary

| Task | Name | Depends | Agent | Model | Key Verification |
|------|------|---------|-------|-------|-----------------|
| T1 | Add max_chars to Config struct | prereq | general-purpose | haiku | Existing config without field loads with default 15000 |
| T2 | Add max_chars prompt during --setup | T1 | general-purpose | haiku | Enter 0 → error, enter 1 → accepted, empty → keeps default |
| T3 | Pass max_chars from config into preflight | T1 | general-purpose | haiku | limit=30000 + diff=25000 → pass; limit=10000 + diff=15000 → DiffTooLarge |
