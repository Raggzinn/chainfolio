# Releasing

Chainfolio ships signed builds through GitHub Releases. Installed apps check
`releases/latest/download/latest.json` and only accept updates signed with the
project key.

## One-time setup

1. Generate the updater key pair (already done for this repository; keep the
   private key safe and never commit it):

   ```bash
   pnpm tauri signer generate -w ~/.tauri/chainfolio.key
   ```

2. The public key lives in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
3. Add the private key to the repository secrets:

   ```bash
   gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/chainfolio.key
   gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --body ""
   ```

   Use the password you chose when generating the key, or an empty value if none.

## Cutting a release

1. Bump the version in `package.json`, `src-tauri/Cargo.toml` and
   `src-tauri/tauri.conf.json` in a `chore(release): vX.Y.Z` pull request.
2. After it merges, tag `main` and push the tag:

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

3. The **Release** workflow builds Windows, macOS and Linux installers, signs
   the update bundles and uploads them with `latest.json` to a draft release.
4. Review the draft and publish it. Only published releases are offered as
   updates, because the endpoint points at `releases/latest`.

> [!WARNING]
> If the private key is lost, installed apps can no longer be updated
> automatically: users would have to reinstall manually.
