<!--
Release notes for the first friend release (WP-02 slices 2.9 and 2.10). Passed as the NOTES argument of
tools/release/publish_windows.sh. Corrected 2026-11-13 by the local AI (Windows) against the real Windows
install (slice 2.15, docs/reviews/626-release-2026-11-13/2.15.md), the files audit
(docs/reviews/2026-11-13-9.11-files-audit/) and the code at c230.
Checked in the 2.9 clean-install run (2026-11-14, docs/reviews/626-release-2026-11-13/2.9/, #794):
- the download page and checksum, the download/verify/install stages, an update to a newer release,
  an update held back during a save, an interrupted update, rollback, bad-signature rejection and
  uninstall all match this text;
- the version line under "Reporting a problem" was corrected (app\VERSION);
- 0.1.0's launcher could update only once (fixed for everyone who runs 0.1.1, #1541);
- not checked: SmartScreen, because the run downloaded with curl, not a browser.
The release after this one has its own notes: docs/release-notes-0.1.1.md. Never write the private
download prefix here.
-->
# Ostrog 0.1.0 (build c230)

Ostrog for Windows 64-bit: an autonomous colony simulation to leave running on a second monitor.
Ten people come through a portal into a valley that was lived in long before them; you choose the
three who land together, and what they build, fear, believe and remember is their own doing. You
watch; you do not command.

## Install

1. Open the download page from the link you were sent and press **Download Ostrog (Windows)**.
   You get `ostrog-windows-x64-portable-c230.zip`. The page also shows its SHA-256 checksum if
   you want to check the download (`Get-FileHash <file> -Algorithm SHA256` in PowerShell).
2. Right-click the zip, choose **Extract All**, and pick a folder you own (for example Documents or
   `D:\Games`, not Program Files). Do not run it from inside the zip window.
3. Open the `ostrog-windows-x64-portable` folder and double-click `ostrog-launcher.exe`.
4. A small window says what the launcher is doing at each step ("Checking for a newer
   version...", "Starting the game. A window will open shortly."), then the game opens. The first
   time, it asks a few questions about your valley (Who watches, The valley, The peoples, Who came
   through, What they hold) and then begins.

**Your saves and settings** are in `%APPDATA%\Godot\app_userdata\Ostrog\` (paste that into the
Explorer address bar). They are not inside the Ostrog folder, so updating or deleting the game
does not remove them.

## Updates

- Each time you start it, the launcher checks our server once for a newer version. It installs one
  only if the update's digital signature and checksum both prove it came from us unchanged; anything
  else is refused and the game you have starts as it was.
- With no internet, it starts the version you already have.
- An update waits if the game is in the middle of saving. The new version is unpacked beside the old
  one and swapped in whole, so an interrupted update leaves the previous version working, and a new
  version that fails to start is rolled back.
- An update replaces only the `app` folder. It never touches the launcher, its `data` folder, or your
  saves.
- The launcher and the game send nothing about you or your games. Our server sees your internet
  address and the time when the launcher checks for an update, like any website.

## Uninstall

Delete the `ostrog-windows-x64-portable` folder. To remove your saves and settings too, also
delete `%APPDATA%\Godot\app_userdata\Ostrog\`. The game also leaves an empty `ostrog` folder
in your temporary folder (`%TEMP%`), which you can delete.

## Known limits (stated, not hidden)

- Windows 64-bit only; no Linux or macOS package is published.
- Updates are checked when you start the launcher; a game left running for days does not update
  itself.
- With 500 people a busy moment can take longer than one frame: about 12 ms on the project's test
  machine and 35 to 45 ms on a slower server CPU. Large settlements may stutter on weaker computers,
  and the busiest scenes do not yet hold 60 frames a second on every graphics card.
- The window shows what the simulation does; it is not tuned as a game to be won.

## Reporting a problem

Tell whoever sent you the link what you were doing, what you saw, and your version (it is in the
`app\VERSION` file in the Ostrog folder; the `VERSION` file beside the launcher keeps the version
you first downloaded). If they ask for more, zip and send the newest
`auto.N.hfsave` folder from `%APPDATA%\Godot\app_userdata\Ostrog\saves\` and the file
`data\logs\launcher.log` from the Ostrog folder. Nothing is sent from your computer unless you
send it.
