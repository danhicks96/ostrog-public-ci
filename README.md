> **Source for CI, not a game or media release.** Rendered images and audio are not tracked.
> The Rust CI job generates verified PNG test inputs from included text/geometry recipes;
> generated media is neither committed nor uploaded. See [Contributing](CONTRIBUTING.md).

# Ostrog

> **Licence and contributions.** Ostrog's code is open source under the
> [GNU GPL v3.0 or later](LICENSE); its art and sound are under
> [CC BY-SA 4.0](LICENSE-ASSETS). You may download, use, fork, study and modify it; see
> [NOTICE](NOTICE) for the credit and naming terms. The name "Ostrog" is not licensed for forks.
> The project is not accepting outside issues or pull requests yet; see [CONTRIBUTING.md](CONTRIBUTING.md).

> **Local fallback rehearsal.** The 113 fallback sprites are unreviewed test inputs, not
> selected or cleared for publication. The general asset licence statement above is not a
> clearance claim for them; see [asset status](docs/ASSETS.md) and [NOTICE](NOTICE).

[Optional image omitted from this source snapshot.]

Ostrog is an autonomous colony simulation meant to run on a second monitor. Ten people come
through a portal into a valley; three of them land together and the other seven are scattered,
alive or dead, and nobody knows which. Each arrives with a life already lived (parents, losses, a
trade, things they came to believe), and from there they forage, build, farm, mine, hunt, cook,
quarrel, pair, raise children, form customs and send parties past the edge of the map.

You watch; you do not command. Everything built after the colonists arrive is assembled tile by
tile from their needs, from materials they gathered and worked, and every line of history the game
shows comes from a recorded event. The simulation never calls a language model.

## Status

Pre-release and under active development. The simulation, the Godot client and a Windows
launcher exist. Ongoing development and CI take place in this repository. See the
[development backlog](docs/BACKLOG.md) for current work and the
[release and development history](docs/RELEASE-HISTORY.md) for how the project got here.

## Build and run

You need Rust (version pinned in `rust-toolchain.toml`), Godot 4.6.3 and Python 3.11+.

```sh
cargo build --release -p hf-cli            # simulation and command-line tool
cargo run --release -p hf-cli -- --help
cargo build --release -p hf-godot          # the extension the Godot client loads
```

Copy the built library into `godot/bin/<platform>/` and open `godot/` in Godot 4.6.3. The full
steps (Windows, cross-builds, packaging, client flags, keyboard and mouse) are in
[docs/REFERENCE.md](docs/REFERENCE.md).

## Test

```sh
tools/ci.sh      # fmt, clippy, tests, asset provenance, docs checks
```

## Architecture map

```text
New world (seed + setup) or loaded save
        │
        ▼
Rust session → fixed tick order → updated world → next tick
        │     world → perceive → choose → act → consequences ↗
        ├─ view snapshot + events → Godot map, panels, camera and audio
        ├─ eligible events → SQLite chronicle → history and save
        ├─ world snapshot + chronicle → autosave → next load
        └─ opt-in diagnostics → decision log; not part of the save
```

| Start here | What it owns | Map |
|---|---|---|
| [Rust workspace](Cargo.toml) | Package and dependency boundaries | [Crates](crates/README.md) |
| [Simulation entry](crates/hf-sim/src/lib.rs) | Session, world state and tick order | [Simulation](crates/hf-sim/README.md) |
| [World setup](crates/hf-sim/src/setup.rs) | Terrain, local past and the wider realm | [World and ecology](crates/hf-sim/README.md#world-and-ecology) |
| [Decision system](crates/hf-sim/src/systems/decide.rs) | People, perception, needs, choices and action | [People and decisions](crates/hf-sim/src/systems/README.md) |
| [Body model](crates/hf-sim/src/body.rs) | Species, bodies, health, life and death | [Bodies and lives](crates/hf-sim/README.md#bodies-and-lives) |
| [Work planner](crates/hf-sim/src/plan.rs) | Jobs, items, buildings, routes and trade | [Work and material](crates/hf-sim/README.md#work-and-material) |
| [Community](crates/hf-sim/src/society.rs) | Groups, social ties, customs, faith and expeditions | [Societies and stories](crates/hf-sim/README.md#societies-and-stories) |
| [Rust–Godot bridge](crates/hf-godot/src/lib.rs) | Worker handoff, view snapshots and render data | [Native bridge](crates/hf-godot/README.md) |
| [Godot entry](godot/scripts/main.gd) | Screens, input, viewer controls and presentation | [Desktop client](godot/README.md) |
| [Chronicle](crates/hf-sim/src/chronicle/mod.rs) | Recorded events, learned history and summaries | [History](crates/hf-sim/src/chronicle/README.md) |
| [Save system](crates/hf-sim/src/save/mod.rs) | Versioned snapshots, migration and recovery | [Persistence](crates/hf-sim/src/save/README.md) |
| [CLI](crates/hf-cli/src/main.rs) | Headless runs, probes, profiles and soaks | [Headless and diagnostics](crates/hf-cli/README.md) |
| [Game data](data/) | Authored definitions; separate from generated assets | [Data](data/README.md) |
| [Tools](tools/) | Checks, assets, performance probes and packaging | [Tools](tools/README.md) |
| [Launcher](crates/hf-launcher/src/) | Release installation and update boundaries | [Build and release](tools/README.md#build-and-release) |
| [Design library](docs/) | Intent, decision history, backlog and reference | [Documentation index](docs/README.md) |

On a fresh source fork, start with the headless smoke in the [CLI guide](crates/hf-cli/README.md).
See [Maintaining the maps](CONTRIBUTING.md#maintaining-the-maps) when changing a boundary.

## Documentation

Start with [docs/VISION.md](docs/VISION.md), [docs/HANDBOOK.md](docs/HANDBOOK.md) and
[docs/design-rules.md](docs/design-rules.md). [CONTRIBUTING.md](CONTRIBUTING.md) explains how to
help.
