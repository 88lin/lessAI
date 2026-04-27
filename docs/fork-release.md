# Fork Release Maintenance

This fork publishes its own GitHub Releases and its own Tauri updater feed.

## Current fork settings

- GitHub repository: `88lin/lessAI`
- Updater endpoint: `https://github.com/88lin/lessAI/releases/latest/download/latest.json`
- Stable base version in repo config: `0.3.4`

## Signing key

- Public key is stored in `src-tauri/tauri.conf.json`.
- Private key should stay outside the repository.
- Local private key path used on this machine:
  `C:\Users\Computer\.lessai-release\88lin\tauri-signing.key`

## GitHub Actions secrets

Set the updater signing key on the repository:

```powershell
$secret = Get-Content C:\Users\Computer\.lessai-release\88lin\tauri-signing.key -Raw
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo 88lin/lessAI --body $secret
```

If the key is password-protected, also set:

```powershell
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo 88lin/lessAI
```

## Automatic package publishing

When you push to `master`, GitHub Actions:

1. Reads the base version from `src-tauri/tauri.conf.json`.
2. Publishes the first fork release as `v0.3.4` if that tag does not exist.
3. Publishes later fork releases as `v0.3.4.1`, `v0.3.4.2`, and so on.
4. Uses Tauri-compatible internal versions such as `0.3.4` and `0.3.4-1`.
5. Builds Windows, Linux, and macOS installers.
6. Publishes a GitHub Release with installers, signatures, `latest.json`, `system-packages.json`, and checksums.

There is also a scheduled fallback every 15 minutes. If a `master` push does not start Actions immediately, GitHub will still detect the new HEAD and publish one automatic package release for that commit.

## Manual stable release

If you want a manually controlled version number, create and push a `v*` tag.

```powershell
git tag v0.3.4
git push origin v0.3.4
```

## Suggested sync flow

```powershell
git fetch upstream --tags
git checkout master
git merge --ff-only upstream/master
git push origin master
```
