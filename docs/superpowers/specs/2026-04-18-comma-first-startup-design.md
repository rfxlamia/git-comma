# comma-cli: First Startup Flow — Design Spec

## Overview

`comma` is a flag-free terminal interface for AI-generated git commits. This spec covers the **first startup flow** only: the onboarding experience when `~/.comma.json` does not yet exist.

---

## Config File

**Path:** `~/.comma.json`

```json
{
  "api_key": "sk-or-v1-...",
  "model_id": "anthropic/claude-3-haiku"
}
```

- `api_key`: OpenRouter API key (Bearer token)
- `model_id`: Selected model identifier (e.g., `anthropic/claude-3-haiku`)

---

## First Startup Flow

```
comma invoked
    │
    ▼
Check ~/.comma.json exists?
    │
    ├── YES → Skip to main auto-commit flow (out of scope for this spec)
    │
    └── NO ↓

Print welcome greeting
    │
    ▼
Prompt: "Masukkan OpenRouter API Key" (masked input)
    │
    ▼
Fetch model list from OpenRouter (includes API key verification)
    │
    ├── FAIL 401 → Clear screen, print error, re-prompt API key
    │
    └── SUCCESS ↓

Print model count and "Ketik untuk mencari..." instruction
    │
    ▼
Display interactive select menu (inquire::Select)
    │
    ├── Option 0: "[ Ketik Manual ID Model... ]" → Text input prompt
    ├── Option 1..N: "provider/model-name" (full list, no truncation)
    │
    ▼
On selection:
    │
    ├── If manual → Prompt for free-text model ID
    │
    └── Save ~/.comma.json with { api_key, model_id }
        │
        ▼
    Continue to main auto-commit flow (out of scope for this spec)
```

---

## API Call

**Endpoint:** `GET https://openrouter.ai/api/v1/models`

**Headers:**
```
Authorization: Bearer <api_key>
```

**Success:** Parse JSON response. Extract each model's `id` and `name` fields. Build display list as `"provider/model-name"`.

**Failure handling:**
- `401 Unauthorized` → `match` on response error, clear screen, print helpful error message, loop back to API key prompt
- Network error → print error, allow retry (loop)

---

## UX Details

| Element | Behavior |
|---------|----------|
| API key input | `PasswordDisplayMode::Masked` — terminal shows `********` only |
| Model list | Full list from OpenRouter, no truncation |
| Fuzzy filter | Built-in via `inquire::Select` — type to filter automatically |
| Manual option | Top of list, triggers `Text::new()` free-text input on selection |
| Error message | Clear and actionable — explains what went wrong |
| Re-prompt on error | Clean loop — clear screen, show error, re-ask |
| Welcome message | Brief, friendly greeting on first run |

---

## Dependencies

```toml
[dependencies]
inquire  = "0.5"    # Terminal UI prompts
ureq     = "3"      # HTTP client (sync, blocking)
serde    = { version = "1", features = ["derive"] }
serde_json = "1"
home     = "0.4"    # Find home directory for ~/.comma.json path
```

---

## Scope

This spec covers **only** the first startup/onboarding flow:
- Config file creation
- API key verification via model list fetch
- Interactive model selection

The main auto-commit flow (git diff → send to OpenRouter → generate commit) is **out of scope** for this spec and will be designed separately.
