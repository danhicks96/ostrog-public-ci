# What Ostrog collects from playtests, and why

Epic #682, work package WP-09 (not included in this source snapshot). This file is the **register**:
the single source of truth for everything the game may send. The in-game "What we collect" page, the
download-page text and the report schema are generated from it (the companion
[FILES-ON-YOUR-COMPUTER.md](FILES-ON-YOUR-COMPUTER.md) explains every file the install creates), and a CI check fails if they disagree
(slice 9.9). If a field is not in this table, the game must not collect or send it.

Status of this register: **proposed**. The maintainer signs off the field list in slice 9.1; until then nothing
in the game may collect anything.

## Principles
1. **Opt-in.** Nothing is recorded for sending, and nothing is sent, until the player says yes on a
   first-run screen. A settings toggle turns it off immediately and discards anything unsent.
2. **Purpose-bound.** Every field answers a written question (column "Question it answers"). No
   question, no field.
3. **Minimal.** Counts and summaries, not raw behaviour. Coarse buckets instead of exact hardware.
4. **Visible.** A "View report" button shows the exact data that would be sent, before anything leaves
   the machine.
5. **No identifiers beyond a random install ID.** No name, e-mail, machine name, user name, file path,
   command line, hardware serial or IP address is stored.
6. **Explicit for anything personal.** Free text, saves and log excerpts leave the machine only when
   the player presses a button and sees a preview.
7. **Small.** The routine report is a few kilobytes, compressed. Large items are never sent in the
   background.
8. **Expiring and deletable.** Reports are deleted after 90 days; "delete my data" removes everything
   under an install ID.
9. **Never affects the game.** Collection reads the simulation and never writes to it; world digests
   are identical with it on or off.
10. **Separate from updates.** The launcher and the update path contain no telemetry. Reports are sent
   by a separate small program.

## The fields

"Routine" means sent automatically after opt-in (in a session report, at game exit or with an update).
"On request" means only when the player presses a button and previews it.

| Field | Example | Question it answers | When sent | Notes |
|---|---|---|---|---|
| `install_id` | random 128-bit number | Which reports come from the same install, and which data to delete on request | Routine | Generated when the player opts in; not linked to a person unless they tell us. Reset by turning collection off and on |
| `game_version`, `commit` | `0.4.1`, `c176` | Which build had the problem | Routine | |
| `os_family` | `Windows` | Is a problem platform-specific | Routine | Family only, no version string |
| `cpu_cores_bucket`, `ram_gb_bucket` | `8-15`, `16-31` | Performance by machine class | Routine | Buckets, not exact values |
| `gpu_vendor` | `NVIDIA` | Frame-time problems by GPU family | Routine | Vendor only. A model string is not collected unless the maintainer approves it in 9.1 |
| `seed`, `region`, `history_years`, `settings` | `3`, `valley`, `150`, in-game options | Reproduce what the player saw (the simulation is deterministic) | Routine | In-game options only; no paths |
| `session_minutes`, `days_played`, `speed_use` | `47`, `12`, share of time at 1x/3x/10x/paused | How the game is watched; whether long unattended sessions work | Routine | Totals only, no timeline |
| `frame_ms_p50/p95/p99/max`, `tick_ms_mean/worst`, `rss_mb_peak` | `9.1`, `14`, `2400` | Do real machines meet the performance gates | Routine | |
| `people_end`, `stalled_person_hours`, `starving_person_hours`, `ledger_drift_days`, `deaths_by_cause` (counts) | `41`, `0`, `3`, `0`, `{cold: 2}` | Does the simulation misbehave on seeds and settings we have not soaked | Routine | Counts only. No names of in-game people |
| `ui_use_counts` | `{panel_inspect: 31, bar_hide: 2}` | Is the mouse-first interface used as designed; which controls are never touched | Routine | Counts per control, no per-click timestamps |
| `crash` | panic message, `file:line`, symbol names, game and Godot error line | What is crashing, and where | Routine | Absolute paths and user names stripped before it is written (guard test) |
| `feedback_text` | what the player types | What the player thinks | On request | Free text; the player chose to write it |
| `save_file` | one `.hfsave` | Reproduce a bug exactly | On request | Preview shows size and what it contains; world and in-game people only. Treated as untrusted data on our side |
| `decision_log_excerpt` | recent lines of the #565 decision log | Why people behaved as they did in a reported bug | On request | The log header's `command line` field is **removed** (it can contain a user name and paths); settings are limited to the in-game options above |

## Never collected
Real name, e-mail, IP address (not stored; see below), machine or user name, file paths, command line,
keystrokes, mouse movement, screenshots (unless the player attaches one), microphone or camera, other
programs or processes, location, hardware serials, anything outside the game window.

## How the server treats it
- A write-only upload endpoint behind the secret path prefix; reports are never served back out.
- Size limit per upload and a per-IP rate limit. The rate limiter keeps IP addresses in memory for a
  minute to count connections and does not write them to disk; the upload route's access log is
  switched off.
- Stored outside the served directory, with a disk quota and automatic deletion at 90 days.
- Uploaded saves are untrusted files and are opened only in a throwaway environment, never on the
  machine that holds the release signing key.

## Rules for changing this register
A new field needs: a question it answers; a row here; the schema and the generated pages updated; and
The maintainer's approval if it is free text, a file, an identifier, or anything not a count or bucket. Removing a
field needs only the PR. The consistency check (9.9) must pass.

## Draft of the text players see (first-run screen)
> **Help improve Ostrog?** If you say yes, the game keeps a small summary of each session (how it
> ran, any crashes, which buttons you used) and sends it to the Ostrog developers. It contains no
> name, e-mail or file paths. You can see exactly what would be sent at any time with "View report",
> turn this off whenever you like in Settings, and ask us to delete your data. Saved games and anything
> you type are only sent if you press "Send bug report" and have looked at them first.
> **[Yes, help out]  [No thanks]  [What exactly is collected?]**
