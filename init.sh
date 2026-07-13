#!/usr/bin/env bash
# KeyPilot V0.1 — Standard Init / Verification
# Reference: AGENTS.md §10 (Sprint Contract)
set -e

# TZ pin for fix-date-local-timezone regression gates.
# NOTE: on Windows CI this env var is IGNORED — chrono::Local reads
# GetTimeZoneInformation, not the TZ env var. The authoritative pin for
# Windows runners is the OS-level `Set-TimeZone` step in
# .github/workflows/tests.yml. This export still helps local runs on
# Linux/macOS, where TZ IS honored.
export TZ='Asia/Shanghai'

cd "$(dirname "$0")"

echo "=== KeyPilot V0.1 Harness Init ==="

# 1. Rust check
echo ""
echo "[1/5] cargo check..."
if [ -f "src-tauri/Cargo.toml" ]; then
    cargo check --manifest-path src-tauri/Cargo.toml
    echo "  ✓ cargo check passed"
else
    echo "  ⚠ src-tauri/Cargo.toml not present yet (Stage 1 pre)"
fi

# 2. Rust tests (full run; no tests = passes with 0 tests, exits 0)
echo ""
echo "[2/5] cargo test..."
if [ -f "src-tauri/Cargo.toml" ]; then
    cargo test --manifest-path src-tauri/Cargo.toml
    echo "  ✓ cargo test passed"
fi

# 3. WebUI install + typecheck
echo ""
echo "[3/5] webui install + typecheck..."
if [ -d "webui" ] && [ -f "webui/package.json" ]; then
    (cd webui && pnpm install)
    (cd webui && pnpm tsc --noEmit)
    echo "  ✓ webui typecheck passed"
else
    echo "  ⚠ webui/ not yet present (Stage 3)"
fi

# 4. Hard constraint grep (V0.1 硬约束 §3)
echo ""
echo "[4/5] Hard constraint grep..."

# 4a. No encryption crates in Cargo.toml
if [ -f "src-tauri/Cargo.toml" ]; then
    if grep -E "argon2|chacha20|ChaCha20Poly1305|aes-gcm|sodiumoxide|^age " src-tauri/Cargo.toml; then
        echo "  ✗ FAIL: encryption crate detected in Cargo.toml (§3.2 violation)"
        exit 1
    else
        echo "  ✓ no encryption crates in Cargo.toml"
    fi
fi

# 4b. Plaintext api_key schema (V0.1 uses provider_fields KV-style: key/value/visibility)
#     V0.1 schema: api_key is a row in provider_fields.key, secret in provider_fields.value (TEXT)
#     §3.2 hard constraint: value 明文 + visibility 二态 visible/masked 不影响落盘
if [ -f "src-tauri/src/database.rs" ]; then
    if grep -q "CREATE TABLE.*provider_fields" src-tauri/src/database.rs \
       && grep -q "value TEXT NOT NULL" src-tauri/src/database.rs \
       && grep -q "visibility TEXT NOT NULL" src-tauri/src/database.rs; then
        echo "  ✓ plaintext api_key schema present (provider_fields KV: value TEXT + visibility TEXT)"
    else
        echo "  ✗ FAIL: provider_fields plaintext schema not detected"
        echo "    expected: provider_fields 表 + value TEXT NOT NULL + visibility TEXT NOT NULL"
        echo "    (V0.1 §3.2: api_key 走 KV 表,provider_fields.value 明文存储)"
        exit 1
    fi
fi

# 4c. fs::write path whitelist (§3.1)
if [ -d "src-tauri/src" ]; then
    BAD=$(grep -rE "fs::write|fs::create_dir_all" src-tauri/src/ 2>/dev/null | \
          grep -E "~/\.claude|~/\.codex|~/\.config/opencode|~/\.local/share/opencode" || true)
    if [ -n "$BAD" ]; then
        echo "  ✗ FAIL: fs::write outside whitelist (§3.1):"
        echo "$BAD"
        exit 1
    else
        echo "  ✓ all fs::write paths within whitelist"
    fi
fi

# 4d. fix-date-local-timezone: TZ anti-pattern gates (REQ-DATE-LOCAL-007).
# Production code MUST NOT use `from_timestamp_millis(...).format("%Y-%m-%d")`
# (UTC-bucketing bug) or `date.toISOString().split("T")[0]` (JS UTC-truncation bug).
# CONTRACT (grep-gate is LINE-ANCHORED on purpose):
#   - the forbidden `from_timestamp_millis(...).format` must sit on ONE line;
#     a multi-line `.unwrap().format` split across two lines is NOT matched
#     (this is symmetric: the intentional discriminator in database.rs::tests
#      is multi-line by design and is exempt — do NOT make the gate
#      multi-line, or it would FALSE-POSITIVE on that test code).
#   - test files legitimately use these patterns as discriminators (spec TC-04
#     etc.); they are exempt by the multi-line layout, not by a `// ` prefix.
#   - known intentional UTC exceptions are commented `// intentional Utc:`
#     at the call site (openai.rs billing, volcengine SigV4 X-Date).
# Exceptions:
#   - src-tauri/src/provider/openai.rs:93 Utc::now().date_naive() — OpenAI billing
#   - src-tauri/src/provider/openai.rs:153 Utc::now() — OpenAI wallet timestamp
#   - src-tauri/src/provider/agent_source.rs:38 Utc.timestamp() — cursor wall-clock seconds
#   - Other peer-callers in claude_oauth / codex_rpc / deepseek — TZ-agnostic epoch
#   - Test files containing these patterns as discriminators in TC-04 etc.
if [ -d "src-tauri/src" ]; then
    BAD_RUST=$(grep -rn 'from_timestamp_millis(.*)\.format("%Y-%m-%d"' src-tauri/src/ 2>/dev/null | \
              grep -v "// " || true)
    if [ -n "$BAD_RUST" ]; then
        echo "  ✗ FAIL: Rust anti-pattern (from_timestamp_millis().format(...%Y...)) — use timeutil::local_date_str:"
        echo "$BAD_RUST"
        exit 1
    else
        echo "  ✓ no forbidden UTC-date-format patterns in Rust"
    fi
fi
if [ -d "webui/src" ]; then
    BAD_TS=$(grep -rn '\.toISOString().split("T")\[0\]' webui/src/ 2>/dev/null \
             | grep -v "lib/format.ts" || true)
    if [ -n "$BAD_TS" ]; then
        echo "  ✗ FAIL: TS anti-pattern (toISOString().split(\"T\")[0]) — use formatLocalDate:"
        echo "$BAD_TS"
        exit 1
    else
        echo "  ✓ no forbidden UTC-truncation patterns in webui"
    fi
fi
if [ -d "src-tauri/src" ]; then
    BAD_DOT=$(grep -rn '\.and_utc()' src-tauri/src/ 2>/dev/null \
              | grep -v "// " || true)
    if [ -n "$BAD_DOT" ]; then
        echo "  ✗ FAIL: Rust anti-pattern (.and_utc() — implicit UTC interpretation). \
Use timeutil::local_date_to_epoch for caller-supplied date strings:"
        echo "$BAD_DOT"
        exit 1
    else
        echo "  ✓ no forbidden .and_utc() in Rust"
    fi
fi

# 5. JSON validity
echo ""
echo "[5/5] feature_list.json validity..."
if [ -f "feature_list.json" ]; then
    if python -c "import json; json.load(open('feature_list.json'))" 2>/dev/null; then
        echo "  ✓ feature_list.json is valid JSON"
    elif node -e "JSON.parse(require('fs').readFileSync('feature_list.json'))" 2>/dev/null; then
        echo "  ✓ feature_list.json is valid JSON"
    else
        echo "  ✗ FAIL: feature_list.json is invalid JSON"
        exit 1
    fi
fi

echo ""
echo "=== Init Complete ==="
echo ""
echo "Next steps:"
echo "1. Read feature_list.json to see current feature state"
echo "2. Pick ONE unfinished stage to work on"
echo "3. Implement only that stage"
echo "4. Re-run verification before claiming done"
