# Contributing

> **Not accepting outside contributions yet.** Ostrog is open source (code: GPL-3.0-or-later,
> see [LICENSE](LICENSE); art and sound: CC BY-SA 4.0, see [LICENSE-ASSETS](LICENSE-ASSETS);
> credit and naming terms in [NOTICE](NOTICE)) and you are welcome to fork it and change it
> for yourself. Until a later version the maintainer is not taking outside issues or pull
> requests, and may close them without review. Forks should use a different name.
> The rest of this file describes the maintainers' own workflow.
>
> Contributions, when accepted, will be licensed on the same terms: code under GPL-3.0-or-later,
> art and sound under CC BY-SA 4.0.

Thanks for your interest in Ostrog. Read [docs/VISION.md](docs/VISION.md) and
[docs/design-rules.md](docs/design-rules.md) first; they hold the design rules a change must keep.

## Design rules in short

- The player watches and gives no orders. Everything built after the colonists arrive is organic:
  assembled tile by tile from materials they gather and refine, never prefab. Prefabs are only for
  structures that already exist in the world (ruins, artifacts).
- Knowledge and skill live in the individuals who learned them, and are lost when they die or leave.
- Every line of history shown comes from a recorded event. The simulation never calls a language model.

## Workflow

1. Open or pick an issue; one change per pull request, branched from `main`.
2. Every behaviour fix adds a guard test that fails without it and an entry in
   [docs/history/FIXLOG.md](docs/history/FIXLOG.md) (symptom, cause, fix).
3. Run `cargo fmt --all` before committing, then `tools/ci.sh` (fmt, clippy `-D warnings`, tests,
   asset provenance, generated-docs checks). When a code change alters the strings or flags the
   game shows, regenerate the tables with `python3 tools/docs/readme_tables.py`.
4. Every file under `godot/assets/` needs a row in `tools/art/provenance.toml`.
5. Decisions that change how the game works go in [docs/DECISIONS.md](docs/DECISIONS.md).

## Privacy and public snapshots

This repository contains source for CI, not a game or media release. Developers bootstrap a
pristine checkout with `python3 tools/ci_source_assets.py` after installing the dependencies
in `tools/requirements-ci-art.txt`. The bootstrap rejects missing inputs, unexpected media,
and output hash drift. Run `--check` to verify existing generated inputs. Never commit or
upload generated PNGs or audio. The Rust job generates PNGs before its unchanged asset tests
and docs checks; the other jobs do not need rendered inputs. Asset provenance now runs in
that job after generation, preserving its coverage without a separate generation pass.

Developers must follow the maintainer-adopted privacy baseline. Keep working history,
private references, credentials, and the review policy and its evidence private. Do not put
those materials in commits, issues, pull requests, build output, or public CI inputs.
Only the approved credit text at its exact locations is excepted; this does not permit other
identifiers, broader suppressions, or personal commit metadata.

Before requesting publication approval, prepare a fresh snapshot with one root commit and
explicit neutral author **and** committer names and email addresses. Preserve private history
separately. Use a reviewed scanner outside the snapshot, in enforced offline isolation, to
check the exact commit headers, tracked paths and file contents as data. Never execute
snapshot code during that audit or let it supply policy exceptions. Bind the result to the
exact commit, tree and artifact digest. Changed bytes require a new exact-artifact check;
unresolved matches or required coverage gaps hold approval. Keep matches and policy records
private. Privacy approval does not establish asset rights or authorize publication.

Keep the existing development repository private as upstream; never mirror it raw into the
public repository. After the clean export and exact offline review, obtain explicit owner
approval of what may cross that boundary. Make the first handoff manually, then verify real
public push and pull-request CI on that exact commit. Private validation hosts may run private
checks. Automating export and verification is an option only after that first CI proof; it does
not imply automated pushing or publication. Agreement on this workflow is not approval to
push a snapshot.

The public snapshot workflow runs Rust formatting, linting and tests, generated-document
checks, Linux/Windows simulation digest comparison, asset integrity/provenance checks, and
export-structure checks. Generic identifier checks on added lines and pull-request text run
on pull requests. Public CI receives no private policy and does not replace the offline audit,
asset-rights review, or Godot import/render and canonical game checks. Its read-only workflow
neither merges nor publishes. A local result is not evidence that hosted checks ran.
