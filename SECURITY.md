# Security Policy

Chainfolio is a watch-only app: it never handles private keys or signs transactions. Still, it stores wallet addresses, balances and optional API keys on your machine, so we take security seriously.

## Reporting a vulnerability

Please **do not open a public issue**. Report it privately through
[GitHub Security Advisories](https://github.com/Raggzinn/chainfolio/security/advisories/new).

Include a description, steps to reproduce and the affected version. You should get a response within 7 days.

## Supported versions

Only the latest release receives security fixes.

## Scope

- Exposure of locally stored data (database, API keys, backups)
- Unsafe IPC commands or capability configuration
- Remote content able to execute code in the app
- Supply-chain issues in the release pipeline
