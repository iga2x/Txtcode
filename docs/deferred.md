# Deferred Work

These items are explicitly **out of scope** until the language itself is solid and correct.
They will be addressed before public launch but do not block language correctness or user programs.

---

## Package Registry Server

**File:** `src/bin/registry_server.rs`
**Status:** Broken — `load_index()` discards file content with `let _ = content;`
**Why deferred:** The registry server is deployment infrastructure, not part of the language runtime.
No user can `txtcode package install http-client` from the internet — the registry does not exist in
deployed form. Fixing a broken server that has no deployed backend provides no value until the
language is ready for distribution.

**When to address:** Before public beta / before writing docs that reference `txtcode package install`.

---

## Binary Release CI / Install Scripts

**Files:** `.github/workflows/release.yml`, `scripts/sign_release.sh`, `install.sh`
**Status:** CI pipeline exists but language features are still incomplete
**Why deferred:** Releasing binaries before the language is stable creates user confusion and
support burden. Install scripts should ship with the stable v3.0 release.

**When to address:** After Groups A–E are complete (language is correct and complete).

---

## Self-Update

**File:** `src/security/update_verifier.rs`, referenced in `txtcode update` subcommand
**Status:** Verification logic exists but no real update mechanism (no CDN, no release URL)
**Why deferred:** Requires deployed binary releases. Depends on registry/CDN infrastructure.

**When to address:** Same as binary releases.

---

## Docker Images

**Status:** Not yet created
**Why deferred:** Infrastructure/deployment concern. No user needs Docker to run Txtcode programs
during language development.

**When to address:** Before public launch alongside install scripts.

---

## Playground Deployment (GitHub Pages)

**Files:** `playground/`, `.github/workflows/playground.yml`, `src/bin/playground.rs`
**Status:** Code complete but CI workflow has never run successfully (wasm-bindgen version may drift)
**Why deferred:** Marketing and demonstration. The language must be correct before showing it off.

**When to address:** After WASM string support (Group H) is complete.

---

## Community Docs Site

**Status:** Not yet created
**Why deferred:** Marketing. Docs should be written once the language is stable.

**When to address:** Alongside public beta launch.

---

## Registry Publishing Workflow

**Command:** `txtcode package publish`
**Status:** CLI command exists but contacts a server that doesn't exist
**Why deferred:** Ecosystem concern. Requires deployed registry server.

**When to address:** After registry server is fixed and deployed.

---

## Ed25519 Release Signing

**Files:** `security/auth.rs`, `scripts/sign_release.sh`
**Status:** Signing code exists; placeholder key material in source tree
**Why deferred:** Tied to binary release workflow.
**Security note:** The placeholder key in `src/security/auth.rs` must be replaced with a real
hardware-backed key before any public release (see Group G Task G.4 in `docs/dev-plan.md`).

**When to address:** Before first public binary release.

---

## Summary Table

| Item                   | Depends on              | Priority when deferred work begins |
|------------------------|-------------------------|------------------------------------|
| Registry server        | Language stable         | High                               |
| Install scripts        | Groups A–E complete     | High                               |
| Binary release CI      | Install scripts         | High                               |
| Self-update            | Binary releases + CDN   | Medium                             |
| Docker images          | Binary releases         | Low                                |
| Playground deployment  | Group H (WASM strings)  | Medium                             |
| Community docs         | Language stable         | Medium                             |
| Registry publishing    | Registry server         | Low                                |
| Ed25519 key            | Binary releases         | CRITICAL (security)                |
