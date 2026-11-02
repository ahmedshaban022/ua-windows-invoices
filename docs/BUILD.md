# Building ua-invoices

This document is the complete build guide for a developer or for the owner who
wants to produce the installer.

---

## 1. Prerequisites

| Tool | Version used | Notes |
| --- | --- | --- |
| Windows | 10 / 11 (x64) | The app targets `x86_64-pc-windows-msvc`. |
| Node.js | 22 LTS | `npm ci` uses the committed `package-lock.json`. |
| Rust | stable (1.8x+) | Install from <https://rustup.rs>. |
| Visual Studio Build Tools 2022 | "Desktop development with C++" | The MSVC linker is required. |
| Windows SDK | 10.0.26100.0 or newer | Included with the Build Tools. |

If the Visual Studio Build Tools are installed in a non-default location, use
the helper script in this repository to configure the environment for one
shell:

```powershell
call D:\Coding\ua-windows-invoices\scripts\msvc-env.bat
```

(The script sets `LIB`, `INCLUDE` and `PATH` for the toolchain installed at
`D:\VSBuildTools`. Edit the paths at the top of the script if your toolchain
lives elsewhere.)

---

## 2. Install the dependencies

```powershell
npm ci
cargo fetch --manifest-path src-tauri/Cargo.toml
```

---

## 3. The typeface

The interface and the invoice use **Cairo**, which is bundled with the
repository in `src/assets/fonts/` (three woff2 subsets, ~81 KB in total) and
declared in `src/styles/fonts.css`. Nothing is fetched from a font CDN at run
time, so the app renders correctly with no network — the same rule the whole
application follows.

Cairo is released under the SIL Open Font License 1.1.

---

## 4. Generate the icons

The Windows icon set is **not** committed as a binary blob; it is generated
from the brand SVG so it always matches the logo:

```powershell
npm run icons
```

This writes `src-tauri/icons/` (`32x32.png`, `128x128.png`,
`128x128@2x.png`, `icon.png`, `icon.ico` with 16/32/48/64/256 px).

---

## 5. Run it in development

```powershell
npm run tauri dev
```

Vite serves the app on <http://localhost:1420> and Tauri opens the window.
The Rust side recompiles on change.

---

## 6. Tests

```powershell
# Rust: 97 tests
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend: 6 tests
npm run test:unit

# End-to-end: 17 tests (needs a browser)
npm run test:e2e

# TypeScript (strict)
npm run typecheck
```

---

## 7. Build the installer

```powershell
npm run tauri build
```

The result is written to:

```
src-tauri/target/release/bundle/nsis/ua-invoices_1.0.0_x64-setup.exe
```

The installer installs **per user** (no administrator rights) and places the
application under `%LOCALAPPDATA%\Programs\ua-invoices`. The user's data lives in
`%APPDATA%\com.ua.invoice` — deleting the application never deletes invoices.

> **The build prints one error at the end when the updater key is not
> configured:**
>
> ```
> A public key has been found, but no private key. Make sure to set
> `TAURI_SIGNING_PRIVATE_KEY` environment variable.
> ```
>
> The **installer is already produced** at this point. The message only means
> that the *updater* artifacts (`latest.json` and the signed `.nsis.zip`) were
> not generated, because signing needs the private key. See
> [`UPDATER.md`](UPDATER.md). In CI the secret is present and the step
> succeeds.

### Building without the installer

```powershell
npm run tauri build -- --no-bundle
```

produces `src-tauri/target/release/ua-invoices.exe`.

---

## 8. Where the data lives

| What | Path |
| --- | --- |
| Database | `%APPDATA%\com.ua.invoice\data\invoices.db` |
| Logo | `%APPDATA%\com.ua.invoice\brand\logo.svg` |
| Default backups | `%APPDATA%\com.ua.invoice\backups\` |

Everything is local. The application has **no** server component, no
telemetry and no network access apart from the optional update check.

---

## 9. Troubleshooting

**`linker 'link.exe' not found`**
The MSVC environment is not active in the shell. Run
`call scripts\msvc-env.bat` first, or install the "Desktop development with
C++" workload.

**`failed to parse manifest` / `baseUrl` errors**
`tsconfig.json` targets TypeScript 7, which removed `baseUrl`. Do not
re-add it — use relative imports.

**The PDF export produces a blank page**
Chromium occasionally refuses to paint a window that has never been shown.
The export code detects a blank capture, shows the hidden render host for
700 ms and captures again. If it still fails, the WebView2 runtime is too old
— install the Evergreen runtime from Microsoft.

**The installer does not appear**
`npm run tauri build` downloads NSIS on first run. If the download fails,
run the command again with network access, or build with `--no-bundle`.
