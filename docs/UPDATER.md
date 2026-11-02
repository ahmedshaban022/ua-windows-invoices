# Auto-update — setup and release procedure

The application can update itself from **GitHub Releases**. The update is
always **user initiated**: the app only *notifies*, and the user clicks
"Update Now". Nothing is installed silently.

> **The private signing key must never be committed, never pasted into a chat
> and never sent to anyone.** It is generated on the owner's machine and only
> ever exists in two places: the owner's local file and the CI secret store.

---

## 1. Generate the keypair (once, on the owner's machine)

```powershell
node scripts/generate-updater-key.mjs
```

The script:

1. calls `tauri signer generate --write-keys %USERPROFILE%\.tauri\invoice-updater.key`,
2. writes the key **outside the repository**, in `%USERPROFILE%\.tauri\`,
3. prints the **public key** (safe to commit) and never prints the private key.

Choose a long password and store it in a password manager — you will need it
in two places (the local file and the CI secret).

---

## 2. Configure the application

In `src-tauri/tauri.conf.json`:

```jsonc
{
  "plugins": {
    "updater": {
      "active": true,
      "dialog": false,          // the app renders its own Arabic dialog
      "pubkey": "<the public key printed by the script>",
      "endpoints": [
        "https://github.com/ahmedshaban022/ua-windows-invoices/releases/latest/download/latest.json"
      ]
    }
  }
}
```

The endpoint must point at the `latest.json` file that the release workflow
publishes.

Until the public key is replaced the updater simply reports that it cannot
reach the update source; the rest of the application is unaffected.

---

## 3. Configure the CI secrets

In the GitHub repository: **Settings → Secrets and variables → Actions**.

| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | The full contents of `%USERPROFILE%\.tauri\invoice-updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The password you chose |

`GITHUB_TOKEN` is provided automatically by GitHub.

---

## 4. Publish a release

```powershell
# 1. Bump the version in src-tauri/tauri.conf.json
#    "version": "1.0.1"

# 2. Commit and tag
git add -A
git commit -m "release: 1.0.1"
git tag v1.0.1
git push origin feat/invoice-desktop-app --tags
```

> The tag **must** be lowercase `v` and the version — the workflow fails
> otherwise. This repository's default branch is `feat/invoice-desktop-app`;
> there is no `main` branch to push to.

The `release.yml` workflow then:

1. checks that the tag matches the version in `tauri.conf.json`,
2. builds the NSIS installer,
3. signs the updater artifacts with the private key from the secret,
4. uploads `latest.json`, the `.nsis.zip` and the installer `.exe`,
5. creates a **draft** release (review it, then publish it).

### Publishing the draft — the step that makes the update visible

A **draft** release is invisible to the updater: the endpoint reads
`releases/latest/download/latest.json`, and `latest` only ever points at a
**published** release. So after the workflow finishes:

1. open the repository → **Releases** → the new draft,
2. check the assets are there (`latest.json`, the `.nsis.zip`, the installer),
3. click **Publish release**.

Only then does a client see the new version. Nothing else is needed — the app
checks on every launch and shows the banner.

### Verifying the update locally first

```powershell
# After the draft is published
npm run tauri dev
```

The banner appears in the sidebar when a newer version is published. The
installed version is shown in **الإعدادات → عن البرنامج**.

---

## 5. Failure behaviour (PRD §10.12)

| Situation | Behaviour |
| --- | --- |
| No update available | "أنت تستخدم أحدث إصدار" — nothing else happens. |
| Offline / unreachable feed | The check fails with an Arabic message; the application keeps working normally. |
| User chooses "Later" | The version is remembered (`update_deferred_version`) and the notification is not shown again for that version. |
| Invalid signature | The Tauri updater plugin rejects the download; the installed version is untouched. |
| Interrupted download | The download is discarded; the installed version stays in place. |
| Failed install | The plugin reports the failure; user data is in `%APPDATA%` and is never touched by the updater. |

User data is **never** removed by an update: the database lives in the
per-user appdata folder, which the installer and the updater both leave alone.

---

## 6. Authenticode (optional, not implemented)

The installer is **not** signed with a Windows Authenticode certificate, so
Windows SmartScreen shows a warning on the first run. This is separate from
the updater signature and is optional; see PRD §11.
