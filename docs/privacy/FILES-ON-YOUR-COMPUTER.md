# Every file Ostrog puts on your computer, and what it is

Epic #682, work package WP-09 (not included in this source snapshot), slices 9.11 and 9.12. Players do
not have the repository, so this guide ships **inside the game** (Settings, "Your files and data"),
as a plain `READ ME FIRST.txt` in the install folder, and as a section of the download page. All
three are generated from this file, and a check fails if the game creates a file that is not listed
here. The companion register of what the game may *send* is [PLAYTEST-DATA.md](PLAYTEST-DATA.md).

Status: **proposed**. Names marked (verify) are taken from the source and are confirmed against a real
clean install in slice 9.11 before this file is final. A promise that nothing exists outside this list
is only made once that audit passes. First audit, 2026-11-13 (evidence (not included in this source snapshot)):
a clean Windows profile, the release-candidate package, a first start and a four-minute play session
with an autosave and debug mode on. Every file it created is in the tables below. Update audit,
2026-11-14 (WP-02 slice 2.9 part 2, the same simulated clean profile): an update, a refused update, a
held-back update, a rolled-back update and an interrupted download (paths (not included in this source snapshot));
every path it left is in the tables below, and the update rows are confirmed by it. Still to audit: the playtest files (not in this build yet).

## Promises we want to be able to keep
- Everything Ostrog creates is in the table below, with a plain-English reason.
- Ostrog writes nothing to the Windows registry, the startup folders or any other program's files
  (the 2026-11-13 audit found none). Windows itself, as it does for every program you run, remembers
  the game's name and which sound device it used, in your user's part of the registry (the `MuiCache`
  list of program names, and an audio `PolicyConfig` entry). Those notes are written by Windows, not
  by Ostrog; they stay after you delete Ostrog and are harmless.
- Deleting the Ostrog folder removes the program. Your saved games are kept apart from the program
  so an update or reinstall never touches them, and the page below says exactly where they are.
- Only the files marked "playtest" ever leave your computer, and only if you opted in.

## The install folder (wherever you unpacked Ostrog)

| File or folder | What it is, in plain words | Sent anywhere? | Safe to delete? |
|---|---|---|---|
| `ostrog-launcher.exe` | The program you double-click. It checks our server for a newer version, downloads updates, and starts the game. It shows you what it is doing at every step. | It asks our server for the version list and the update; it sends nothing about you or your game. Our server's web log sees your IP address and the time, like any website (see "Open items") | Not unless you are uninstalling |
| `launcher.toml` | The launcher's settings: the address of our update server, the public key it uses to check that an update really came from us, your version, and whether to check for updates. You can read it in Notepad. Setting `check_enabled = false` turns update checks off. | No | No (the launcher needs it) |
| `README.md`, `VERSION` | The project's read-me (today the full developer README, about 0.6 MB) and the version of this build. | No | Yes |
| `READ ME FIRST.txt` | This guide in plain text. | No | Yes |
| `app/` | The game itself. Every update replaces this folder completely, so never keep your own files in it. Contains `ostrog.exe` (the game program), `ostrog.pck` (the game's art, sounds and scripts), `hf_godot.dll` (the game's simulation engine) and copies of `README.md` and `VERSION`. | No | Only if you are uninstalling |
| `data/` | The launcher's own folder. `logs/launcher.log` is a plain diary of what the launcher did ("checked for updates", "downloaded version X", "you cancelled the download"); it stays on your computer and grows by about half a kilobyte each time you start Ostrog. `.keep` is an empty placeholder. | No | The log, yes. Leave the rest |
| `.ostrog-update/` | Used during an update: the new version being unpacked, and a copy of your previous version so an update that fails to start can be undone. Emptied once the new version has started; the empty folder may stay. | No | Yes, when no update is running |

## Your game data
The game stores your saves and settings through Godot's standard per-user folder, which on Windows is
`%APPDATA%\Godot\app_userdata\Ostrog\` (type that into the Explorer address bar). Confirmed on a
real Windows 11 install on 2026-11-13 (WP-02 slice 2.15,
evidence (not included in this source snapshot)). The "Your files
and data" page in the game has an **Open folder** button that goes straight there.

| File or folder | What it is, in plain words | Sent anywhere? | Safe to delete? |
|---|---|---|---|
| `settings.cfg` | Your options: sound, interface scale, camera and so on. | No | Yes (options reset) |
| `announce.cfg` | Which alerts you chose to see or pause on. | No | Yes |
| `saves/` | Your saved games: the rolling autosaves (`auto.0.hfsave` to `auto.2.hfsave`) and your manual save (`manual.hfsave`). Each save is a folder holding `world.bin`, `chronicle.sqlite` and `manifest.json`; it is written first as `<name>.hfsave.tmp` so a half-written save never replaces a good one. While a save is being written the folder also holds `.ostrog-save-writing`, a tiny marker the launcher looks for so an update never starts mid-save; it disappears when the save is done. | Only if you press "Send bug report" and tick a save | **No: this is your game.** Delete only to start over |
| `logs/` | The game engine's own diary, `godot.log`, used to find crashes. The engine also keeps a few older ones from earlier launches, named `godot<date>T<time>.log`, and removes the oldest itself. | No | Yes |
| `shader_cache/` | The engine's store of graphics programs it has already prepared for your graphics card, so the next start is quicker (under 1 MB). | No | Yes (rebuilt on the next start) |
| `ostrog-debug.log` | A detailed record of why each person did what they did. **Only written when debug mode is on**. It grows quickly (about 6 MB in four minutes at the fastest speed); at 32 MB it is renamed `ostrog-debug.log.1` and a new one starts, so the two together never pass about 64 MB. | No, except a short excerpt if you press "Send bug report" and approve a preview | Yes |
| `timelapse/` | **Only if you turn Timelapse on**: one picture of the egress point at noon each game day, with a small note of what happened, kept per world so you can watch it grow. At most 120 pictures and 160 MB per world, for at most 6 worlds (about 1 GB in the worst case); the oldest are removed first. | No | Yes (you lose the time-lapse) |

The game takes no screenshots in normal play; the screenshot and footage options exist only for
testing and write where the tester tells them to.

## Temporary files (in your Windows temporary folder, `%TEMP%`)

| File or folder | What it is, in plain words | Sent anywhere? | Safe to delete? |
|---|---|---|---|
| `ostrog/` | While a game is open, the egress point's history is kept in a working file here (`chronicle-<number>.sqlite`, with `-wal` and `-shm` companions) instead of in memory; your saves copy it. The game removes the file when it closes and clears any left over from a crash on a later start. The empty folder stays. | No | Yes, when the game is not running |
| `ostrog-archive-*.zip.part` | While the launcher downloads an update: the part-downloaded file, checked before anything is installed and removed when the launcher has finished with it. If the launcher is closed in the middle of a download, the part file stays until you delete it. | No | Yes, when no update is running |

## Playtest files (only if you chose "Yes, help out")
| File or folder | What it is, in plain words | Sent anywhere? | Safe to delete? |
|---|---|---|---|
| `install_id` (verify name) | A random number that stands in for your name, so one install's reports can be grouped and deleted on request. It is not linked to you. | Yes, with each report | Yes (turn collection off and on to get a new one) |
| `reports/` (verify) | Small summaries of how each session ran, waiting to be sent. The "View report" button shows you exactly what is in them. | Yes, after you opted in | Yes |

## Open items found while writing this (to be fixed or decided in slices 2.15 and 9.1)
1. **Where saves really live** (decided in slice 2.15, 2026-11-11). Saves and settings stay in the
   standard per-user folder above, where players expect them and where they survive deleting the
   Ostrog folder; `data/` holds only the launcher's log. The "never update during a save" marker
   is written next to the saves, and the launcher now looks for it there as well as in `data/`
   (`hf_update::godot_user_dir`), so that safety works in a real install. Confirmed on Windows 11
   (2.15 evidence above: saves, settings and the marker are where the launcher looks); the launcher
   holding back a real update while the marker exists was confirmed in the 2.9 update step
   (2026-11-14: `started installed build (SaveInProgress)`, nothing downloaded).
2. **Our update server's web log.** Today the log records each visitor's IP address and the time, and
   keeps them in the server's system journal. Either switch it off for the download paths, or keep it
   for a short stated period and say so here. Decision in 9.1 (default: off, keeping only an
   anonymous count per day).
3. **The debug log's header** includes the command line, which can contain a user name and folder
   paths. It is removed from anything sent (see PLAYTEST-DATA.md) and should not be written at all
   when the log is shared (slice 9.4).

## How this stays true
Slice 9.11 audits a clean Windows install and a play session, comparing every file, folder and
registry change against this table. `tools/privacy/files_register.py` checks a list of found paths
against the tables' first columns (a name ending in `/` covers everything inside it); CI fails when a
packaged file (`tools/tests/windows_package_test.py`) or a file in either Windows audit
(`tools/tests/files_register_test.py`: `docs/reviews/2026-11-13-9.11-files-audit/audit-paths.txt`
and `docs/reviews/2026-11-14-9.11-update-audit/audit-paths.txt`) has no row. Slice 9.12 builds the in-game page, the `READ ME FIRST.txt` and the
download-page section from this file: `tools/privacy/files_guide.py` writes
`docs/privacy/READ-ME-FIRST.txt` (shipped as `READ ME FIRST.txt`), the game's copy of the rows
(`godot/data/files_register.json`) and the download page's "What it puts on your computer" block
(`tools/server/index.html`, between its generated markers), and CI fails when any differs from
this file. Players see
the four tables only, without "(verify)" notes; the playtest rows and any "Send bug report" wording
stay out until the game has those features (slices 9.2 to 9.4). A new file means a new row here, in
the same change, and `python3 tools/privacy/files_guide.py` after it.
