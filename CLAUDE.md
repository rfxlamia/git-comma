# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**comma** is a Rust CLI tool that generates git commits using AI (OpenRouter API). It prompts users for API key and model selection on first run, then saves config to `~/.comma.json`.

## Build & Test Commands

```bash
cargo build        # Build the project
cargo test         # Run all tests
cargo test config_tests    # Run specific test file
cargo test -- --nocapture  # Run tests with output
cargo run          # Run the application
```

## Architecture

```
src/
├── main.rs        # Entry point; orchestrates preflight → AI → commit flow
├── lib.rs         # Re-exports AI engine components for integration tests
├── config.rs      # Config struct (api_key, model_id), load/save with atomic writes
├── openrouter.rs  # OpenRouter API client for model listing and chat completions
├── setup.rs       # First-startup flow and model reconfiguration
├── preflight.rs   # Git repo validation, staged file checks, diff size limits
├── ai.rs          # AI engine: orchestrates prompt building, API calls, sanitization
├── prompt.rs      # Builds OpenRouter API payload from diff content
├── sanitization.rs # Cleans/safeguards AI response before use
├── tui.rs         # Terminal UI: editor integration, action prompts (accept/edit/regenerate)
├── ui.rs          # User interface helpers (prompts, messages)
└── error.rs       # AI error types (ApiError, RateLimitExceeded, Network, etc.)
```

**Main flow:** `main.rs` checks for `~/.comma.json` → if missing/malformed, calls `setup::run_first_startup()`. On valid config: preflight checks → `ai::run_ai_engine()` → action loop (accept/edit/regenerate) → `git commit -F`.

**Lib exports (`lib.rs`):** Exposes AI engine components so integration tests can call `run_ai_engine`, `commit_with_draft`, etc. without depending on `main.rs`.

## Key Patterns

- Config file: `~/.comma.json` (created on first run)
- Permissions: 0o600 (readable only by owner) on Unix
- Atomic writes: write to `.json.tmp` then rename
- API errors: `Unauthorized` (401), `Forbidden` (403), `RateLimited` (429) trigger model switch or re-setup flow
- Soft diff limit: 15,000 chars (`SOFT_DIFF_LIMIT` in `preflight.rs`) — user can bypass
- Safety net: draft saved to `.git/comma_msg.txt` before commit attempt; survives hook failures

## Behavioral Guidelines

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:

- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:

- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:

- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:

- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:

```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

