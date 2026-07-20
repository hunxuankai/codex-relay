# Conditional NSIS Install Directory Design

## Goal

For fresh x64 Windows installations, prefer `D:\Program Files\Codex Relay` when `D:` is a fixed drive; otherwise default to the operating system's 64-bit Program Files directory. Preserve a previous installation directory during upgrades and keep the NSIS directory selection page available.

## Installer scope

The installer changes from `currentUser` to `perMachine`. This is required because both candidate Program Files locations normally require elevation. The installer therefore requests administrator permission and records installer metadata under HKLM.

## Directory selection

The custom template is based on the official Tauri `tauri-v2.11.4` `installer.nsi` template.

During `.onInit`, only when `$INSTDIR` still has Tauri's placeholder value:

1. Call `GetDriveTypeW("D:\")`.
2. If the result is `DRIVE_FIXED` (`3`), set `$INSTDIR` to `D:\Program Files\${PRODUCTNAME}`.
3. Otherwise use Tauri's architecture-aware `$PROGRAMFILES64` / `$PROGRAMFILES` fallback.
4. Call `RestorePreviousInstallLocation` after choosing the fresh-install default so an existing per-machine registered installation directory takes precedence.

The standard `MUI_PAGE_DIRECTORY` remains present, so users can change the proposed path interactively. Command-line `/D=` and passive/update behavior continue to use the upstream template logic.

## Safety and compatibility

- A missing, removable, optical, or network `D:` drive falls back to the Windows Program Files directory.
- Existing installs are not migrated between drives automatically.
- The earlier current-user installer stored metadata under HKCU and is not automatically migrated to the new HKLM per-machine installation. Users must uninstall that older application first; its uninstaller leaves Codex configuration and application data intact.
- The upstream Tauri install, WebView2, shortcut, registry, update, and uninstall sections remain unchanged.
- The uninstaller continues to leave Codex configuration, application data, API keys, logs, and backups untouched.

## Verification

- A Vitest regression test locks `perMachine`, the custom template path, the fixed-drive check, both destinations, ordering before `RestorePreviousInstallLocation`, and preservation of the directory page.
- `npm run check` verifies frontend and Rust checks.
- `npm run build` compiles the custom NSIS template and generates the final installer.
- The generated installer path, size, timestamp, and SHA-256 are enumerated after the build.
