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
through a portal into the world. Three emerge together at the egress point of the chosen landing site. The other seven are scattered,
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

## Layout

| Path | What |
|---|---|
| `crates/` | Deterministic Rust simulation, CLI, Godot bridge, launcher |
| `godot/` | The Godot 4 client (GDScript, scenes, generated assets) |
| `tools/` | CI, packaging, and the scripts that generate art, audio and docs tables |
| `data/` | Game data |
| `docs/` | Vision, design, decisions, reference; see [docs/README.md](docs/README.md) |

## Documentation

Start with [docs/VISION.md](docs/VISION.md), [docs/HANDBOOK.md](docs/HANDBOOK.md) and
[docs/design-rules.md](docs/design-rules.md). [CONTRIBUTING.md](CONTRIBUTING.md) explains how to
help.
