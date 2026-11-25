# Rust workspace

[Root map](../README.md#architecture-map) · [Workspace manifest](../Cargo.toml)

| Package | Responsibility |
|---|---|
| [hf-sim](hf-sim/README.md) | Authoritative simulation, save format and chronicle |
| [hf-cli](hf-cli/README.md) | Headless runner, profilers, probes and save inspection |
| [hf-godot](hf-godot/README.md) | Native client bridge and worker handoff |
| [hf-launcher](hf-launcher/src/) | Desktop launcher |
| [hf-update](hf-update/src/) | Shared update and save locking |
| [hf-release](hf-release/src/) | Release-side command-line tools |

Start at the package's manifest before changing a dependency. UI projections and build tools
do not own simulation state. See [tools](../tools/README.md) for packaging and validation.
