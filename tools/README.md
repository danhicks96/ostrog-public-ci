# Development tools

[Root map](../README.md#architecture-map) · [Contributing](../CONTRIBUTING.md) · [Headless tools](../crates/hf-cli/README.md)

| Path | Purpose |
|---|---|
| [docs](docs/) | Generate/check code-derived reference and data map contracts |
| [tests](tests/) and [CI](ci.sh) | Local verification; see [hosted jobs](../.github/workflows/ci.yml) for public scope |
| [perf](perf/) and [soak](soak/) | Frame and long-run probes |
| [art](art/) and [audio](audio/) | Asset generation inputs and recipes |
| [shots](shots/) and [Godot tooling](godot/) | Client verification and capture |
| [hygiene](hygiene/) and [privacy](privacy/) | Public source checks and privacy tooling |

## Build and release

[Launcher](../crates/hf-launcher/src/), [update library](../crates/hf-update/src/),
[release CLI](../crates/hf-release/src/) and [release helpers](release/) have distinct entry points.
Use [the release reference](../docs/REFERENCE.md) and [contributor policy](../CONTRIBUTING.md);
a local build, green CI job or git tag does not itself publish anything.
