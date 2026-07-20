# Conditional NSIS Install Directory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a per-machine NSIS installer that defaults fresh installs to fixed drive `D:` when available and otherwise uses Windows Program Files, without changing an existing installation directory.

**Architecture:** Copy the official Tauri 2.11.4 NSIS template into the repository and change only the fresh-install directory block in `.onInit`. Keep Tauri's `RestorePreviousInstallLocation` call after default selection so upgrades win, and point `tauri.conf.json` at the custom template.

**Tech Stack:** Tauri 2.11.4 configuration, NSIS/LogicLib, Vitest, PowerShell.

---

### Task 1: Lock installer configuration and directory behavior

**Files:**
- Modify: `src/release-config.test.ts`
- Test: `src/release-config.test.ts`

- [ ] **Step 1: Write the failing configuration test**

Read `installer/custom-installer.nsi` and assert:

```ts
expect(tauri.bundle.windows.nsis.installMode).toBe('perMachine')
expect(tauri.bundle.windows.nsis.template).toBe('installer/custom-installer.nsi')
expect(nsisTemplate).toContain('GetDriveTypeW(w "D:\\")')
expect(nsisTemplate).toContain('D:\\Program Files\\${PRODUCTNAME}')
expect(nsisTemplate).toContain('$PROGRAMFILES64\\${PRODUCTNAME}')
expect(nsisTemplate.indexOf('GetDriveTypeW')).toBeLessThan(
  nsisTemplate.indexOf('Call RestorePreviousInstallLocation'),
)
expect(nsisTemplate).toContain('!insertmacro MUI_PAGE_DIRECTORY')
```

- [ ] **Step 2: Run the test and verify RED**

Run: `npm run test -- --run src/release-config.test.ts`

Expected: FAIL because the installer remains `currentUser` and the custom template does not exist.

### Task 2: Add the custom Tauri NSIS template

**Files:**
- Create: `src-tauri/installer/custom-installer.nsi`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Copy the official template**

Use the official template from:

```text
https://raw.githubusercontent.com/tauri-apps/tauri/tauri-v2.11.4/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi
```

- [ ] **Step 2: Change only the per-machine fresh-install branch**

Inside `.onInit`, replace the upstream per-machine default selection with:

```nsis
System::Call 'kernel32::GetDriveTypeW(w "D:\\") i .r0'
${If} $0 == 3
  StrCpy $INSTDIR "D:\Program Files\${PRODUCTNAME}"
${Else}
  ; retain upstream architecture-aware Program Files logic
${EndIf}
Call RestorePreviousInstallLocation
```

- [ ] **Step 3: Configure Tauri**

Set:

```json
"installMode": "perMachine",
"template": "installer/custom-installer.nsi"
```

- [ ] **Step 4: Run the focused test and verify GREEN**

Run: `npm run test -- --run src/release-config.test.ts`

Expected: all release configuration tests pass.

### Task 3: Verify and package

**Files:**
- Modify: `docs/verification-report.md`

- [ ] **Step 1: Run complete checks**

Run: `npm run check`

Expected: TypeScript, frontend tests, Rust fmt, Clippy, unit tests, and integration tests exit 0.

- [ ] **Step 2: Build Release and NSIS**

Run: `npm run build`

Expected: exit 0 and a new `Codex Relay_0.1.0_x64-setup.exe`.

- [ ] **Step 3: Verify the artifact**

Enumerate the installer path, byte length, last-write timestamp, and SHA-256. Confirm the custom template compiled and the Tauri configuration still uses `perMachine` and the expected template path.

- [ ] **Step 4: Update evidence and commit**

Update `docs/verification-report.md`, run `git diff --check`, request independent review, and commit the implementation.

