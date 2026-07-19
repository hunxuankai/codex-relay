# Codex Relay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build, test, document, and package Codex Relay as a lightweight Windows 10/11 Tauri application that safely manages local Codex Providers and produces a verified executable and NSIS installer.

**Architecture:** Vue owns presentation only; thin Tauri commands call focused Rust services. Every file mutation is serialized by one transaction service using backups, same-directory temporary files, post-write validation, conflict fingerprints, and verified rollback. Tests inject temporary paths and failure points so no test can touch real Codex or application data.

**Tech Stack:** Tauri 2, Vue 3, TypeScript, Vite, Rust, `toml_edit`, `serde`, `serde_json`, Tokio, notify, sha2, tracing, Vitest, Vue Test Utils, Tauri Tray/Autostart/Single Instance/Notification plugins, NSIS.

---

## File map

The implementation will create these responsibility groups:

- `src-tauri/src/infrastructure/`: path isolation, atomic files, fingerprints, safe logging.
- `src-tauri/src/models/`: serializable provider, settings, health, backup, transaction, and command-result types.
- `src-tauri/src/services/`: configuration, secrets, auth, backup, transactions, Provider operations, self-check, autostart, and file watching.
- `src-tauri/src/commands/`: thin Tauri command adapters only.
- `src-tauri/src/tray.rs`: dynamic tray menu and shared Provider switch entry point.
- `src/`: Vue components, views, composables, types, and one mockable Tauri service boundary.
- `fixtures/`: fixed invalid/valid configuration samples containing only fake keys.
- `docs/`: architecture, transaction, security, and generated design/plan records.

Each task below ends in a focused commit. Commands assume PowerShell in `D:\Kai\Project\unifyProject\codex-relay`.

### Task 1: Create the Tauri/Vue project baseline

**Files:**
- Create: `.gitignore`
- Create: `package.json`
- Create: `package-lock.json` via npm
- Create: `index.html`
- Create: `tsconfig.json`
- Create: `tsconfig.app.json`
- Create: `tsconfig.node.json`
- Create: `vite.config.ts`
- Create: `vitest.setup.ts`
- Create: `src/env.d.ts`
- Create: `src/main.ts`
- Create: `src/App.vue`
- Create: `src/App.test.ts`
- Create: `src/style.css`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/main.rs`

- [ ] **Step 1: Add the Node and Rust manifests**

Create `.gitignore`:

```gitignore
node_modules/
dist/
target/
src-tauri/target/
dev-data/*
!dev-data/.gitkeep
test-data/
*.log
.env
.env.*
providers.json
auth.json
src-tauri/gen/
```

Create `package.json` with scripts that expose every required check:

```json
{
  "name": "codex-relay",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "tauri dev",
    "build": "tauri build",
    "build:debug": "tauri build --debug --no-bundle",
    "typecheck": "vue-tsc --noEmit -p tsconfig.app.json",
    "test": "vitest run",
    "test:watch": "vitest",
    "check:frontend": "npm run typecheck && npm run test",
    "check:rust": "cargo fmt --check --manifest-path src-tauri/Cargo.toml && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml",
    "check": "npm run check:frontend && npm run check:rust"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-autostart": "^2",
    "@tauri-apps/plugin-notification": "^2",
    "vue": "^3.5"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@vitejs/plugin-vue": "^6",
    "@vue/test-utils": "^2",
    "jsdom": "^26",
    "typescript": "~5.9",
    "vite": "^7",
    "vitest": "^3",
    "vue-tsc": "^3"
  }
}
```

Create `src-tauri/Cargo.toml` with these declared dependencies:

```toml
[package]
name = "codex-relay"
version = "0.1.0"
description = "Safely manage local Codex model providers"
authors = ["Codex Relay contributors"]
edition = "2024"

[lib]
name = "codex_relay_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2.6.3", features = [] }

[dependencies]
tauri = { version = "2.11.5", features = ["tray-icon"] }
tauri-plugin-autostart = "2.5.1"
tauri-plugin-notification = "2.3.3"
tauri-plugin-single-instance = "2.4.3"
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.150"
toml_edit = "0.25.13"
tokio = { version = "1.53.0", features = ["process", "rt-multi-thread", "sync", "time"] }
url = "2.5.8"
uuid = { version = "1.24.0", features = ["v4", "serde"] }
chrono = { version = "0.4.45", features = ["serde"] }
sha2 = "0.11.0"
notify = "8.2.0"
tracing = "0.1.44"
tracing-appender = "0.2.5"
tracing-subscriber = { version = "0.3.20", features = ["env-filter"] }
thiserror = "2.0.19"
regex = "1.12.2"

[dev-dependencies]
tempfile = "3.27.0"
serial_test = "3.5.0"
```

- [ ] **Step 2: Add the minimal Vue app and failing smoke test**

Create `src/App.test.ts`:

```ts
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import App from './App.vue'

describe('App', () => {
  it('renders the product name', () => {
    expect(mount(App).text()).toContain('Codex Relay')
  })
})
```

Run: `npm install && npm run test -- --run src/App.test.ts`  
Expected: FAIL because the Vue/Vite configuration and component are not yet complete.

- [ ] **Step 3: Implement the minimal Vue/Vite files**

Create `src/App.vue`:

```vue
<script setup lang="ts">
</script>

<template>
  <main class="app-shell">
    <h1>Codex Relay</h1>
  </main>
</template>
```

Create `src/main.ts` that mounts `App` and imports `style.css`. Configure Vite with Vue and Vitest `jsdom`, `globals: true`, and `setupFiles: ['./vitest.setup.ts']`.

- [ ] **Step 4: Add the minimal Tauri application**

Create `src-tauri/src/lib.rs`:

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run Codex Relay");
}
```

Create `main.rs` calling `codex_relay_lib::run()`. Configure `tauri.conf.json` with product name `Codex Relay`, identifier `com.codexrelay.app`, 900×620 resizable window, `beforeDevCommand`, `beforeBuildCommand`, frontend paths, Windows target, and NSIS bundle.

- [ ] **Step 5: Verify the baseline**

Run: `npm run typecheck`  
Expected: PASS.

Run: `npm run test -- --run src/App.test.ts`  
Expected: 1 test PASS.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`  
Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add .gitignore package.json package-lock.json index.html tsconfig*.json vite.config.ts vitest.setup.ts src src-tauri 项目提示词.txt
git commit -m "chore: initialize Tauri Vue application"
```

### Task 2: Add fixtures and protected path resolution

**Files:**
- Create: `fixtures/config-empty.toml`
- Create: `fixtures/config-single-provider.toml`
- Create: `fixtures/config-multiple-providers.toml`
- Create: `fixtures/config-with-comments.toml`
- Create: `fixtures/config-with-unknown-fields.toml`
- Create: `fixtures/config-invalid.toml`
- Create: `fixtures/auth-api-key.json`
- Create: `fixtures/auth-invalid.json`
- Create: `fixtures/providers-empty.json`
- Create: `fixtures/providers-multiple.json`
- Create: `fixtures/providers-invalid.json`
- Create: `src-tauri/src/infrastructure/mod.rs`
- Create: `src-tauri/src/infrastructure/path_service.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create all required fixtures**

Use the prompt’s sample Provider structure. `config-with-comments.toml` must include a leading comment, `[features]`, `[mcp_servers.sample]`, and an unknown Provider field. JSON fixtures use only `test-key-a-not-real` and `test-key-b-not-real`.

- [ ] **Step 2: Write failing path precedence tests**

In `path_service.rs`, add tests using `serial_test` and `tempfile`:

```rust
#[test]
#[serial]
fn relay_override_wins_over_codex_home() {
    let relay = tempfile::tempdir().unwrap();
    let codex = tempfile::tempdir().unwrap();
    let _guard = EnvGuard::set_many([
        ("CODEX_RELAY_CODEX_HOME", relay.path()),
        ("CODEX_HOME", codex.path()),
    ]);
    assert_eq!(resolve_codex_home().unwrap(), relay.path());
}
```

Add separate tests for `CODEX_HOME`, `%USERPROFILE%\.codex`, app-data override, default app-data, and rejection of real directories when `PathMode::Test` is used.

- [ ] **Step 3: Run the path tests and confirm failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml path_service -- --nocapture`  
Expected: FAIL because `PathContext`, `PathMode`, and resolution functions are not implemented.

- [ ] **Step 4: Implement path resolution and guards**

Define:

```rust
pub enum PathMode { Production, Test }

pub struct AppPaths {
    pub codex_home: PathBuf,
    pub config_file: PathBuf,
    pub auth_file: PathBuf,
    pub app_data_dir: PathBuf,
    pub providers_file: PathBuf,
    pub settings_file: PathBuf,
    pub backups_dir: PathBuf,
    pub logs_dir: PathBuf,
}

pub fn resolve_paths(mode: PathMode) -> Result<AppPaths, AppError>;
impl AppPaths {
    pub fn for_test(codex_home: PathBuf, app_data_dir: PathBuf) -> Result<Self, AppError>;
}
```

Production honors both overrides but logs that relay-specific overrides are active. Test mode additionally proves resolved paths do not equal or descend from real defaults before allowing filesystem access.

- [ ] **Step 5: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml path_service`  
Expected: all path tests PASS.

```powershell
git add fixtures src-tauri/src/infrastructure src-tauri/src/lib.rs
git commit -m "feat: add protected configuration path resolution"
```

### Task 3: Define shared models and safe errors

**Files:**
- Create: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/models/provider.rs`
- Create: `src-tauri/src/models/settings.rs`
- Create: `src-tauri/src/models/health.rs`
- Create: `src-tauri/src/models/backup.rs`
- Create: `src-tauri/src/models/transaction.rs`
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write serialization and secret-exclusion tests**

Add tests that serialize `ProviderProfile` and assert `apiKeyConfigured`, `isActive`, and `isValid` are present while `apiKey` and the fake key value are absent. Add an `AppError` test that formats an internal error containing `test-key-a-not-real` and asserts the public message excludes it.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml models error`  
Expected: FAIL because models and error mapping do not exist.

- [ ] **Step 3: Implement models and command result**

Define Provider fields with `#[serde(rename_all = "camelCase")]`:

```rust
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub wire_api: WireApi,
    pub model: Option<String>,
    pub api_key_configured: bool,
    pub is_active: bool,
    pub is_valid: bool,
    pub validation_message: Option<String>,
}
```

Define `Settings`, `HealthReport`, `HealthCheck`, `HealthLevel`, `BackupSummary`, `BackupMetadata`, and `ConfigTransaction`. Define `CommandResult<T>` and `CommandError` with stable codes and safe Chinese messages.

- [ ] **Step 4: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml models error`  
Expected: PASS.

```powershell
git add src-tauri/src/models src-tauri/src/error.rs src-tauri/src/lib.rs
git commit -m "feat: add domain models and safe command errors"
```

### Task 4: Implement fingerprints and atomic file primitives

**Files:**
- Create: `src-tauri/src/infrastructure/file_fingerprint.rs`
- Create: `src-tauri/src/infrastructure/atomic_file.rs`
- Modify: `src-tauri/src/infrastructure/mod.rs`

- [ ] **Step 1: Write failing fingerprint and atomic-write tests**

Cover missing files, equal content, changed content, trailing newline preservation, validator rejection leaving the destination unchanged, and replacement verification failure restoring the prior bytes.

Use this validator shape:

```rust
atomic_write(&path, b"value\n", |bytes| {
    std::str::from_utf8(bytes).map(|_| ()).map_err(AppError::from)
})
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml atomic_file file_fingerprint`  
Expected: FAIL because the primitives are missing.

- [ ] **Step 3: Implement the primitives**

`FileFingerprint` records existence, length, modification timestamp when available, and SHA-256. `FileSetFingerprint` contains named fingerprints for config, auth, and Provider secrets so edit sessions can send one conflict token. `atomic_write` creates a uniquely named temporary file beside the target, writes and flushes it, validates the bytes, replaces the destination, rereads it, and validates again. On Windows, use a safe rename/replace sequence that preserves the prior file until the new file is validated.

- [ ] **Step 4: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml atomic_file file_fingerprint`  
Expected: PASS.

```powershell
git add src-tauri/src/infrastructure
git commit -m "feat: add verified atomic file operations"
```

### Task 5: Implement log and error redaction

**Files:**
- Create: `src-tauri/src/infrastructure/safe_log.rs`
- Modify: `src-tauri/src/infrastructure/mod.rs`
- Modify: `src-tauri/src/error.rs`

- [ ] **Step 1: Write failing redaction tests**

Test all required patterns:

```rust
#[test]
fn redacts_structured_and_header_secrets() {
    let input = r#"OPENAI_API_KEY=test-key-a-not-real apiKey\":\"test-key-b-not-real\" Authorization: Bearer secret-token https://x.test?a=1&token=query-secret"#;
    let output = redact(input);
    for secret in ["test-key-a-not-real", "test-key-b-not-real", "secret-token", "query-secret"] {
        assert!(!output.contains(secret));
    }
    assert!(output.contains("[REDACTED]"));
}
```

Add a public-error test proving a nested I/O message containing a key is redacted.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml safe_log`  
Expected: FAIL because `redact` is absent.

- [ ] **Step 3: Implement redaction and rolling log setup**

Implement `redact(&str) -> String` without logging original secrets during parsing. Add tracing setup with non-blocking daily rolling files, Release `INFO`, a bounded file-retention cleanup routine, and no automatic Debug formatting of command payloads.

- [ ] **Step 4: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml safe_log error`  
Expected: PASS with no secret in output.

```powershell
git add src-tauri/src/infrastructure src-tauri/src/error.rs
git commit -m "feat: redact secrets from logs and errors"
```

### Task 6: Parse and edit `config.toml` without destroying user content

**Files:**
- Create: `src-tauri/src/services/mod.rs`
- Create: `src-tauri/src/services/config_service.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing Provider parsing and validation tests**

Load the required fixtures and test multiple Providers, active Provider recognition, missing fields, invalid URL, invalid ID, duplicate ID, blank name, and non-`responses` Wire API. Use explicit test names such as `reads_multiple_providers`, `rejects_toml_path_injection_id`, and `marks_invalid_base_url`.

- [ ] **Step 2: Run the parsing tests and confirm failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config_service::tests::reads_multiple_providers`  
Expected: FAIL because `ConfigService` is missing.

- [ ] **Step 3: Implement parsing and validation**

Implement focused functions:

```rust
pub struct ProviderConfig {
    pub id: String,
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub wire_api: Option<String>,
    pub model: Option<String>,
}

pub fn parse_document(source: &str) -> Result<DocumentMut, AppError>;
pub fn list_provider_configs(document: &DocumentMut) -> Vec<ProviderConfig>;
pub fn validate_provider_id(id: &str) -> Result<String, AppError>;
pub fn validate_provider_input(input: &ProviderInput) -> Result<ValidatedProviderInput, AppError>;
```

ID normalization trims and lowercases, then accepts only ASCII letters, numbers, `_`, and `-`. Base URL must parse as HTTP or HTTPS and is stored in normalized URL form without guessing paths. Name and optional model are trimmed.

- [ ] **Step 4: Write failing preservation tests for create/edit/delete/switch**

From `config-with-comments.toml`, assert that create keeps comments, `[features]`, MCP tables, unknown fields, and other Providers. Assert edit preserves the target Provider’s unknown fields and other Providers. Assert delete removes only the requested non-current table. Assert switch updates `model_provider`, writes `cli_auth_credentials_store = "file"`, updates a configured model, and preserves the old top-level model when the target has none.

- [ ] **Step 5: Implement in-memory TOML mutations**

Provide methods returning modified text without writing disk:

```rust
pub fn create_provider(source: &str, input: &ValidatedProviderInput) -> Result<String, AppError>;
pub fn update_provider(source: &str, id: &str, input: &ValidatedProviderInput) -> Result<String, AppError>;
pub fn delete_provider(source: &str, id: &str) -> Result<String, AppError>;
pub fn select_provider(source: &str, provider: &ProviderConfig) -> Result<String, AppError>;
```

Use `toml_edit` table and item APIs only. Never deserialize and regenerate the entire document.

- [ ] **Step 6: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config_service`  
Expected: all parsing, validation, preservation, and selection tests PASS.

```powershell
git add src-tauri/src/services src-tauri/src/lib.rs
git commit -m "feat: preserve Codex TOML while managing providers"
```

### Task 7: Implement Provider secret and auth JSON services

**Files:**
- Create: `src-tauri/src/services/provider_secret_service.rs`
- Create: `src-tauri/src/services/auth_service.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write failing secret-store tests**

Test default creation, multiple keys, update-one-keeps-other, delete-one-keeps-other, empty-key detection, two-space formatting, terminal newline, and damaged-file preservation. For corruption, assert the original invalid file still exists and a `.corrupt-<timestamp>` copy exists before the error is returned.

- [ ] **Step 2: Write failing auth tests**

Test parsing an existing `OPENAI_API_KEY`, generation of exactly that field, UTF-8 JSON, two spaces, newline, equality check, and safe failure for invalid JSON without echoing file contents.

- [ ] **Step 3: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml provider_secret_service auth_service`  
Expected: FAIL because services are missing.

- [ ] **Step 4: Implement secret-store and auth operations**

Define:

```rust
pub struct ProviderSecretStore {
    pub version: u32,
    pub providers: BTreeMap<String, ProviderSecret>,
}

pub struct ProviderSecret {
    pub api_key: String,
}
```

Expose read, configured-state, get-for-edit, set, explicit clear, delete, serialize, and validated-write operations. Trim accidental CR/LF at key boundaries but do not require an `sk-` prefix. `AuthService` generates `{"OPENAI_API_KEY": key}` using `serde_json::to_string_pretty` plus one newline.

- [ ] **Step 5: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml provider_secret_service auth_service`  
Expected: PASS.

```powershell
git add src-tauri/src/services
git commit -m "feat: manage provider keys and Codex auth JSON"
```

### Task 8: Implement transaction backups and restore

**Files:**
- Create: `src-tauri/src/services/backup_service.rs`
- Create: `src-tauri/src/services/transaction_service.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write failing backup tests**

Test that a backup directory contains only files that originally existed plus `metadata.json`, that metadata contains no key, that listing sorts newest first, that cleanup retains exactly 20 completed backups, and that the active transaction backup is never deleted.

- [ ] **Step 2: Write failing transaction rollback tests**

Create a `FileOps` trait and a test double with failure stages:

```rust
pub trait FileOps: Send + Sync {
    fn read_optional(&self, path: &Path) -> Result<Option<Vec<u8>>, AppError>;
    fn atomic_write_validated(&self, path: &Path, bytes: &[u8], kind: FileKind) -> Result<(), AppError>;
    fn remove_if_exists(&self, path: &Path) -> Result<(), AppError>;
}

pub enum FailureStage {
    ConfigWrite,
    AuthWrite,
    ProvidersWrite,
    PostWriteValidation,
    Rollback,
}
```

Assert failures restore all prior bytes when rollback succeeds. Assert rollback failure returns `ROLLBACK_INCOMPLETE` and never claims restoration.

- [ ] **Step 3: Run tests and verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml backup_service transaction_service`  
Expected: FAIL because the services and failure seam are missing.

- [ ] **Step 4: Implement backup service**

Create timestamped `<timestamp>-<transaction-id>` directories. Copy existing source files and serialize safe `BackupMetadata`. Implement list, validate, cleanup, and restore-source loading. Restore itself must be executed by `TransactionService`, not by direct backup writes.

- [ ] **Step 5: Implement serialized transactions**

Define one application-wide `tokio::sync::Mutex<()>`. A transaction captures pre-write fingerprints, creates backup, checks fingerprints again, writes every changed file through `FileOps`, validates the final composite state, and restores prior existence/bytes on error. Persist a small transaction marker before writes and remove it only after success or verified rollback so self-check can find interrupted work.

- [ ] **Step 6: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml backup_service transaction_service`  
Expected: PASS, including injected failures.

```powershell
git add src-tauri/src/services
git commit -m "feat: add configuration backups and verified rollback"
```

### Task 9: Implement Provider CRUD and switching as transactions

**Files:**
- Create: `src-tauri/src/services/provider_service.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/services/transaction_service.rs`

- [ ] **Step 1: Write failing aggregate-list tests**

Create temporary config and secret files, then assert `list_providers` merges TOML details with configured-key status, current status, validation status, and messages without exposing keys.

- [ ] **Step 2: Write failing CRUD transaction tests**

Cover successful create, create-and-enable, duplicate rejection, edit without key change, explicit key clear, current-key-clear warning, non-current delete, current delete rejection, preservation of other Providers/keys, backups, conflict fingerprint rejection, and user-confirmed import of an existing current `auth.json` key only into the active Provider.

- [ ] **Step 3: Write failing switch transaction tests**

Cover target missing, key missing, concurrent switches, target model present, target model absent, `cli_auth_credentials_store`, final auth equality, failure at config/auth/verification stages, successful rollback, and rollback-incomplete messaging.

- [ ] **Step 4: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml provider_service`  
Expected: FAIL because `ProviderService` is missing.

- [ ] **Step 5: Implement the Provider service**

Expose asynchronous methods:

```rust
pub async fn list_providers(&self) -> Result<Vec<ProviderProfile>, AppError>;
pub async fn create_provider(&self, input: CreateProviderInput) -> Result<ProviderMutationOutcome, AppError>;
pub async fn update_provider(&self, input: UpdateProviderInput) -> Result<ProviderMutationOutcome, AppError>;
pub async fn delete_provider(&self, id: &str, expected: FileSetFingerprint) -> Result<(), AppError>;
pub async fn switch_provider(&self, id: &str) -> Result<SwitchOutcome, AppError>;
pub async fn sync_current_provider(&self) -> Result<SwitchOutcome, AppError>;
pub async fn import_current_auth_key(&self, expected_provider_id: &str) -> Result<ProviderMutationOutcome, AppError>;
```

Define the request/outcome DTOs in `models/provider.rs`:

```rust
pub struct CreateProviderInput {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: Option<String>,
    pub api_key: String,
    pub activate_after_save: bool,
    pub expected_files: FileSetFingerprint,
}

pub struct UpdateProviderInput {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: Option<String>,
    pub api_key_change: ApiKeyChange,
    pub sync_if_active: bool,
    pub expected_files: FileSetFingerprint,
}

pub struct ProviderMutationOutcome {
    pub providers: Vec<ProviderProfile>,
    pub message: String,
}

pub struct SwitchOutcome {
    pub providers: Vec<ProviderProfile>,
    pub active_provider_id: String,
    pub message: String,
}
```

Mutation outcomes include refreshed profiles and exact safe localized messages: create `Provider「{name}」已保存。`, create-and-enable `Provider「{name}」已保存并启用。请重启 Codex 后生效。`, edit `Provider「{name}」已更新。`, current edit `Provider「{name}」已更新。请重启 Codex 后生效。`, delete `Provider「{name}」已删除。`, and switch `已切换到「{name}」。配置已写入，请重启 Codex 后生效。`.

- [ ] **Step 6: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml provider_service transaction_service`  
Expected: PASS, including the simultaneous-switch test.

```powershell
git add src-tauri/src/services
git commit -m "feat: add transactional provider management"
```

### Task 10: Implement settings, autostart, and self-check services

**Files:**
- Create: `src-tauri/src/services/settings_service.rs`
- Create: `src-tauri/src/services/autostart_service.rs`
- Create: `src-tauri/src/services/self_check_service.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write failing settings tests**

Test missing-file defaults, safe JSON persistence, window bounds, first-run flag, manual-start visibility, autostart tray-only behavior, and malformed settings backup without silent overwrite.

- [ ] **Step 2: Write failing self-check tests**

Use injectable `CodexCommandProbe` and `AutostartBackend`. Test directory readability/writability, invalid TOML/JSON, missing active Provider, invalid Provider fields, missing/mismatched keys, CLI present, CLI absent warning, CLI timeout warning, actual autostart mismatch, transaction marker, backup count, and no secret in returned checks. Add a startup-service graph test proving no HTTP client or Provider endpoint probe is constructed or invoked during either critical or extended startup checks.

- [ ] **Step 3: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml settings_service self_check_service`  
Expected: FAIL because services are missing.

- [ ] **Step 4: Implement settings and autostart abstractions**

`SettingsService` creates defaults with `closeToTray = true`, `showWindowOnManualStart = true`, and `trayOnlyOnAutostart = true`. During application bootstrap it creates the Codex directory when allowed, the application-data directory, `backups`, `logs`, version-1 empty `providers.json`, and default `settings.json`; it never invents a Provider or fake `config.toml`. `AutostartService` wraps a trait so unit tests do not change Windows startup registration; the production adapter uses the Tauri Autostart plugin and always queries actual state after changes.

- [ ] **Step 5: Implement critical and extended checks**

Provide:

```rust
pub async fn run_critical_checks(&self) -> HealthReport;
pub async fn run_extended_checks(&self) -> HealthReport;
```

Critical checks never spawn `codex`. Extended checks run `codex --version` with a bounded Tokio timeout and classify absence as warning. Reports contain only statuses, safe messages, current Provider summary, and paths.

- [ ] **Step 6: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml settings_service autostart_service self_check_service`  
Expected: PASS.

```powershell
git add src-tauri/src/services
git commit -m "feat: add settings autostart and health checks"
```

### Task 11: Implement debounced external-file monitoring

**Files:**
- Create: `src-tauri/src/services/file_watch_service.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write failing monitor tests**

Use a fake watcher/event sink and Tokio paused time. Assert bursts are debounced, application write IDs are suppressed, config changes emit Provider refresh and health refresh, auth changes emit auth health refresh, secret-store changes emit key-state refresh, and full file contents never enter events or logs.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml file_watch_service`  
Expected: FAIL because the monitor is missing.

- [ ] **Step 3: Implement watcher and write suppression**

Wrap `notify::RecommendedWatcher`. Watch the three exact files or their parent directories if files do not yet exist. Coalesce events for 300 ms, recompute fingerprints, compare against the last application transaction fingerprints, and emit typed `ConfigFilesChanged` events rather than raw content.

- [ ] **Step 4: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml file_watch_service`  
Expected: PASS.

```powershell
git add src-tauri/src/services
git commit -m "feat: monitor external Codex configuration changes"
```

### Task 12: Expose safe Tauri commands and application state

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/provider_commands.rs`
- Create: `src-tauri/src/commands/settings_commands.rs`
- Create: `src-tauri/src/commands/backup_commands.rs`
- Create: `src-tauri/src/commands/self_check_commands.rs`
- Create: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing command-adapter tests**

Test successful and failing mappings with a fake service facade. Assert command JSON uses `success/data/error`, no stack is returned, list commands never contain keys, and secret-edit commands require a specific Provider ID.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands`  
Expected: FAIL because commands and managed state are absent.

- [ ] **Step 3: Implement application state and commands**

`AppState` owns `Arc` service instances and the shared transaction mutex. Register commands for list/create/update/delete/switch, dedicated current-secret read/update, confirmed current-auth-key import, settings get/update/autostart, backups list/restore, health critical/extended, and opening the Codex directory. Each command delegates once and converts `Result<T, AppError>` to `CommandResult<T>`.

Example adapter:

```rust
#[tauri::command]
pub async fn switch_provider(
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> CommandResult<SwitchOutcome> {
    state.provider_service.switch_provider(&provider_id).await.into()
}
```

- [ ] **Step 4: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands`  
Expected: PASS.

```powershell
git add src-tauri/src/commands src-tauri/src/app_state.rs src-tauri/src/lib.rs
git commit -m "feat: expose safe Tauri command boundary"
```

### Task 13: Integrate tray, plugins, lifecycle, and notifications

**Files:**
- Create: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Write pure tray-menu model tests**

Separate menu modeling from Tauri handles. Test current checkmark, invalid or keyless Provider disabled state, switching busy state, menu rebuild after Provider changes, and required commands: open, current label, Providers, health, open directory, autostart, exit.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tray`  
Expected: FAIL because tray modeling is missing.

- [ ] **Step 3: Implement tray behavior**

Create tray immediately during Tauri setup with a placeholder current state, then rebuild after critical loading. Provider menu events call the same `ProviderService::switch_provider`. During switching set a busy flag, ignore repeat clicks, refresh on success, retain the prior selection on failure, and send safe app/Windows notifications.

Handle double-click and “打开 Codex Relay” by unminimizing, showing, and focusing the existing window. Intercept window move/resize with a short debounce and persist usable bounds; restore saved bounds only when they intersect a current monitor. Intercept window close to hide when `closeToTray` is true. Only the tray exit item sets an exit flag and terminates.

- [ ] **Step 4: Register plugins and startup behavior**

Register Single Instance first, then Autostart and Notification. On a second launch, show and focus the existing window. Inspect startup arguments to identify autostart mode; combine that with actual settings to decide initial visibility. Do not create a Windows service or require administrator access.

- [ ] **Step 5: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tray`  
Expected: PASS.

Run: `cargo check --manifest-path src-tauri/Cargo.toml --all-features`  
Expected: PASS.

```powershell
git add src-tauri
git commit -m "feat: add tray lifecycle and Windows integrations"
```

### Task 14: Add typed frontend service and composables

**Required skill before editing Vue files:** `vue-best-practices`.

**Files:**
- Create: `src/types/provider.ts`
- Create: `src/types/health.ts`
- Create: `src/types/backup.ts`
- Create: `src/types/settings.ts`
- Create: `src/types/command.ts`
- Create: `src/services/tauri.ts`
- Create: `src/composables/useProviders.ts`
- Create: `src/composables/useHealth.ts`
- Create: `src/composables/useBackups.ts`
- Create: `src/composables/useSettings.ts`
- Create: `src/services/tauri.test.ts`
- Create: `src/composables/useProviders.test.ts`

- [ ] **Step 1: Write failing service tests**

Mock `@tauri-apps/api/core` and assert `listProviders`, `createProvider`, `updateProvider`, `deleteProvider`, `switchProvider`, settings, backups, and health call the exact Rust command names and unwrap `CommandResult`. Assert a failed result throws a typed `RelayCommandError` containing only code and safe message.

- [ ] **Step 2: Write failing Provider composable tests**

Assert initial loading, refresh, create-refresh, delete-refresh, switch-refresh, busy protection, selected Provider retention, safe error state, and success text containing the backend message.

- [ ] **Step 3: Run tests to verify failure**

Run: `npm run test -- --run src/services/tauri.test.ts src/composables/useProviders.test.ts`  
Expected: FAIL because types, service, and composables are absent.

- [ ] **Step 4: Implement frontend boundary and composables**

Mirror Rust camelCase DTOs exactly. `src/services/tauri.ts` is the only file importing `invoke`. Composables use Vue refs/computed, expose explicit actions, never store API Keys outside the editor state, and subscribe to typed backend refresh events.

- [ ] **Step 5: Verify and commit**

Run: `npm run typecheck`  
Expected: PASS.

Run: `npm run test -- --run src/services/tauri.test.ts src/composables/useProviders.test.ts`  
Expected: PASS.

```powershell
git add src/types src/services src/composables
git commit -m "feat: add typed frontend command services"
```

### Task 15: Build and test Provider management UI

**Required skill before editing Vue files:** `vue-best-practices`.

**Files:**
- Create: `src/components/ProviderList.vue`
- Create: `src/components/ProviderList.test.ts`
- Create: `src/components/ProviderEditor.vue`
- Create: `src/components/ProviderEditor.test.ts`
- Create: `src/components/ProviderStatus.vue`
- Create: `src/components/ApiKeyInput.vue`
- Create: `src/components/ApiKeyInput.test.ts`
- Create: `src/components/ConfirmDialog.vue`
- Create: `src/components/AppNotification.vue`
- Create: `src/views/ProvidersView.vue`
- Create: `src/views/ProvidersView.test.ts`

- [ ] **Step 1: Write failing list and action tests**

Test Provider name/ID/Base URL/Wire API/model, current badge, invalid message, key configured/unconfigured state, keyless use disabled, current delete disabled, and emitted select/edit/use/delete actions.

- [ ] **Step 2: Write failing editor and key-input tests**

Test required ID/name/Base URL/key rules, lowercase ID, immutable ID in edit mode, fixed `responses`, optional trimmed model, password input by default, show/hide, unchanged key omission, explicit clear confirmation, and current-Provider warning.

- [ ] **Step 3: Write failing Provider view flow tests**

Mock the composable and test create refresh, edit refresh, delete confirmation and refresh, switch success containing `请重启 Codex 后生效`, switch failure, missing-key disabled state, and external-conflict message.

- [ ] **Step 4: Run tests to verify failure**

Run: `npm run test -- --run src/components src/views/ProvidersView.test.ts`  
Expected: FAIL because UI components are absent.

- [ ] **Step 5: Implement focused accessible components**

Use semantic buttons, labels, `aria-live` notifications, visible focus states, and no heavy component framework. `ProviderEditor` emits a typed payload but never writes localStorage. `ConfirmDialog` traps focus while open and restores it on close. The view asks whether to sync when current Provider fields changed.

- [ ] **Step 6: Verify and commit**

Run: `npm run typecheck`  
Expected: PASS.

Run: `npm run test -- --run src/components src/views/ProvidersView.test.ts`  
Expected: all Provider UI tests PASS.

```powershell
git add src/components src/views/ProvidersView.vue src/views/ProvidersView.test.ts
git commit -m "feat: add provider management interface"
```

### Task 16: Build health, backup, settings, onboarding, and shell UI

**Required skill before editing Vue files:** `vue-best-practices`.

**Files:**
- Create: `src/components/HealthStatus.vue`
- Create: `src/components/HealthStatus.test.ts`
- Create: `src/views/BackupsView.vue`
- Create: `src/views/BackupsView.test.ts`
- Create: `src/views/SettingsView.vue`
- Create: `src/views/SettingsView.test.ts`
- Create: `src/views/OnboardingView.vue`
- Create: `src/views/OnboardingView.test.ts`
- Modify: `src/App.vue`
- Modify: `src/App.test.ts`

- [ ] **Step 1: Write failing health and backup tests**

Assert normal/warning/error states render correctly, CLI missing remains a warning, self-check can be rerun, backup metadata excludes keys, restore requires confirmation, restore refreshes health and Providers, and restore failure displays a safe error.

- [ ] **Step 2: Write failing settings tests**

Assert the page shows actual autostart state, can enable/disable it, surfaces plugin errors, and controls tray-only autostart, close-to-tray, and manual-start visibility.

- [ ] **Step 3: Write failing onboarding and App-shell tests**

Assert onboarding appears when configuration or Providers are absent and provides exactly open directory, add first Provider, later, and exit actions. When the active Provider lacks a stored key but `auth.json` has one, assert a separate confirmation offers import into that Provider only and does nothing when declined. Assert normal mode provides Providers, health, backups, and settings navigation plus status bar fields.

- [ ] **Step 4: Run tests to verify failure**

Run: `npm run test -- --run src/views/BackupsView.test.ts src/views/SettingsView.test.ts src/views/OnboardingView.test.ts src/App.test.ts`  
Expected: FAIL because views and application shell are incomplete.

- [ ] **Step 5: Implement views and shell**

Use local view state rather than adding a router dependency. `App.vue` selects views, runs initial Provider/settings/critical-health loading, starts extended health after the first render, and responds to backend refresh events. The status bar displays the resolved config directory, current Provider, last operation, self-check summary, and open-directory action.

- [ ] **Step 6: Verify and commit**

Run: `npm run check:frontend`  
Expected: typecheck and all current frontend tests PASS.

```powershell
git add src
git commit -m "feat: add health backup settings and onboarding views"
```

### Task 17: Finish responsive Windows styling and icons

**Required skill before editing Vue files:** `vue-best-practices`.

**Files:**
- Modify: `src/style.css`
- Modify: `src/App.vue`
- Create: `src/assets/icons/*.svg`
- Create: `src-tauri/icons/icon.ico`
- Create: other Tauri-required icon sizes via the Tauri icon command
- Modify: relevant Vue component templates

- [ ] **Step 1: Add UI assertions for accessibility contracts**

Extend tests to assert named navigation/buttons, dialog roles, status live region, key visibility button labels, keyboard-triggerable actions, and no fixed-width layout that prevents a narrower window.

- [ ] **Step 2: Run the assertions and confirm failure**

Run: `npm run test`  
Expected: FAIL on missing accessibility names or layout hooks.

- [ ] **Step 3: Implement the visual system**

Use CSS variables with `prefers-color-scheme`, a two-column grid that collapses at narrow widths, minimum 44 px interactive targets, visible focus rings, restrained shadows, no large gradients, and `rem`/flex/grid sizing suitable for 100%, 125%, and 150% Windows scaling. Use small local SVG icons with text labels; no runtime network assets.

- [ ] **Step 4: Generate application icons**

Create one simple original SVG source with a relay/arrows motif and generate Tauri Windows icon assets using:

```powershell
npx tauri icon src/assets/app-icon.svg
```

Expected: Tauri icon files including `src-tauri/icons/icon.ico` are generated.

- [ ] **Step 5: Verify and commit**

Run: `npm run check:frontend`  
Expected: PASS.

```powershell
git add src src-tauri/icons
git commit -m "style: finish responsive Windows interface"
```

### Task 18: Add startup integration tests and safe development data

**Files:**
- Create: `dev-data/.gitkeep`
- Create: `scripts/prepare-dev-data.ps1`
- Create: `src-tauri/tests/provider_workflow.rs`
- Create: `src-tauri/tests/path_safety.rs`
- Modify: `.gitignore`
- Modify: `package.json`

- [ ] **Step 1: Write failing end-to-end Rust workflow tests**

Use `tempfile` and the fixture sample to run create → edit → switch → delete → backup restore through the real service graph. Assert comments/features survive, auth matches the selected fake key, delete cannot remove current Provider, and restore returns original bytes.

- [ ] **Step 2: Write explicit path-safety integration tests**

Remove both override variables, invoke the test service constructor, and assert it refuses to initialize. Set overrides to temporary directories and assert every opened or written path is beneath those roots. Record opened paths in an auditing `FileOps` wrapper and assert no path equals real Codex or application-data defaults.

- [ ] **Step 3: Run tests to verify the safety gate**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test path_safety -- --nocapture`  
Expected: initial failures expose any constructor that could fall back to real user data.

- [ ] **Step 4: Implement safe test constructors and dev-data script**

All integration tests call `AppPaths::for_test(temp_codex, temp_app_data)` and cannot use `Production`. `prepare-dev-data.ps1` creates the exact prompt sample with `test-key-not-real`, sets both override variables for the current process, and never references the real user profile.

Add a `dev:safe` package script:

```json
"dev:safe": "powershell -ExecutionPolicy Bypass -File scripts/prepare-dev-data.ps1"
```

- [ ] **Step 5: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test provider_workflow --test path_safety`  
Expected: PASS and audited paths remain temporary.

```powershell
git add .gitignore package.json scripts src-tauri/tests dev-data/.gitkeep
git commit -m "test: verify provider workflows stay in safe paths"
```

### Task 19: Write project documentation and release configuration

**Files:**
- Create: `README.md`
- Create: `AGENTS.md`
- Create: `docs/architecture.md`
- Create: `docs/config-transaction.md`
- Create: `docs/security-notes.md`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `package.json`

- [ ] **Step 1: Write the required Chinese documentation**

README must cover all 26 prompt items: purpose, features, stack, structure, prerequisites, install, development, tests, Release, path resolution, data paths, three config files, plaintext-key design and risks, tray, autostart, backup/restore, common errors, safe test directories, real exit, uninstall, limitations, and unsigned publisher warning.

`AGENTS.md` must state that real `.codex` and real application data are forbidden during development/tests, secrets cannot be committed or logged, every write goes through transactions, TOML cannot be regenerated wholesale, rollback cannot be bypassed, tests must run after changes, and results cannot be overstated.

The three docs must describe the exact architecture flows, transaction stages and rollback truthfulness, and personal-use plaintext-key security trade-off.

- [ ] **Step 2: Finalize NSIS metadata**

Set product name, executable name, current-user installation mode when supported, start-menu shortcut, icon, version, and NSIS target. Do not add uninstall hooks that remove Codex configuration, backups, or application data. Add `build:release` and `bundle:nsis` scripts if the installed Tauri CLI needs separate commands.

- [ ] **Step 3: Run documentation and config checks**

Run: `rg -n "Credential Manager|Keyring|DPAPI|Stronghold|未知发布者|CODEX_RELAY_CODEX_HOME|CODEX_RELAY_APP_DATA_DIR" README.md docs AGENTS.md`  
Expected: the security exclusions, unsigned warning, and path overrides are explicitly documented.

Run: `npm run typecheck && cargo check --manifest-path src-tauri/Cargo.toml`  
Expected: PASS after release metadata changes.

- [ ] **Step 4: Commit**

```powershell
git add README.md AGENTS.md docs package.json src-tauri/tauri.conf.json
git commit -m "docs: document Codex Relay architecture and security"
```

### Task 20: Perform the completion audit, builds, and artifact verification

**Required skills before claiming completion:** `superpowers:verification-before-completion` and `superpowers:requesting-code-review`.

**Files:**
- Modify only files required to fix failures found by the checks
- Create: `docs/verification-report.md`

- [ ] **Step 1: Run frontend verification**

Run: `npm run typecheck`  
Expected: exit 0.

Run: `npm run test`  
Expected: all frontend tests pass, including at least the 15 required behaviors.

- [ ] **Step 2: Run Rust formatting, lint, and tests**

Run: `cargo fmt --check --manifest-path src-tauri/Cargo.toml`  
Expected: exit 0.

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`  
Expected: exit 0 with no warnings.

Run: `cargo test --manifest-path src-tauri/Cargo.toml`  
Expected: all unit and integration tests pass, including every required Rust behavior and path-safety assertions.

- [ ] **Step 3: Build Debug and perform a safe launch smoke test**

Run: `npm run build:debug`  
Expected: Debug executable is generated.

Launch it only with both overrides pointing to `dev-data` and verify: tray appears, main window opens on manual start, closing hides it, second launch focuses the existing instance, and tray exit terminates it. Record exact observed results.

- [ ] **Step 4: Build Release and NSIS**

Run: `npm run build`  
Expected: Release executable and NSIS installer are generated successfully.

Enumerate actual artifacts:

```powershell
Get-ChildItem -Recurse src-tauri\target\release -Include CodexRelay.exe,*.exe | Select-Object FullName,Length,LastWriteTime
```

Expected: the application executable and NSIS installer are present. Record their exact paths rather than assuming filenames.

- [ ] **Step 5: Audit secrets and unsafe paths**

Run repository searches for likely secret prefixes, `OPENAI_API_KEY` assignments, Authorization headers, and unignored `providers.json`/`auth.json`. Review every hit; only fake fixtures and redaction tests are permitted.

Run: `git status --short --ignored`  
Expected: generated secret files, `dev-data`, `target`, `dist`, and logs are ignored.

Review test output and path-audit logs to prove tests did not open real `%USERPROFILE%\.codex` or `%LOCALAPPDATA%\CodexRelay`.

- [ ] **Step 6: Write the evidence report**

Create `docs/verification-report.md` with exact commands, timestamps, exit codes, test counts, artifact paths, manual tray/single-instance/autostart observations, secret-scan result, real-path audit result, known limitations, and any genuinely incomplete item with its real reason.

- [ ] **Step 7: Run the complete check once more after report/fixes**

Run: `npm run check`  
Expected: exit 0.

Run: `npm run build`  
Expected: exit 0 and artifacts still present.

- [ ] **Step 8: Commit verified completion**

```powershell
git add -A
git commit -m "build: verify Codex Relay release artifacts"
```

- [ ] **Step 9: Prepare final user report**

Report all 22 required final-report items from authoritative command output and `docs/verification-report.md`. Do not state that a check passed unless its recorded command actually exited successfully.
