# Security policy

Ostrog is a single-player simulation with a launcher and an updater that verifies signed
release manifests. The most sensitive parts are the update and release code
(`crates/hf-update`, `crates/hf-release`, `crates/hf-launcher`).

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's **Report a vulnerability** button on the
repository's Security tab (private vulnerability reporting). Describe what you found, how to
reproduce it, and what you think the impact is. Do not include personal information.

The project is maintained by one person and outside contributions are not being taken yet, so
replies can be slow. Reports about the update signature check, save-file loading, or anything
that could run code from a downloaded file are the most useful.

## Supported versions

Only the latest release and the `main` branch are supported.
