# Ostrog 0.1.1 (build c252)

Ostrog for Windows 64-bit: an autonomous colony simulation to leave running on a second monitor.
Ten people come through a portal into a valley that was lived in long before them; you choose the
three who land together, and what they build, fear, believe and remember is their own doing. You
watch; you do not command.

## New since 0.1.0

- The launcher window no longer says your saves are in its `data` folder; it says they are kept in
  AppData, which is where they are (`%APPDATA%\Godot\app_userdata\Ostrog\`).
- Some of the people who wake from the stone carry one fragment of the old world: a symbol, one
  step of a rite, a place that feels wrong, a material that seems to belong in a machine. They do
  not know what it means, and what they tell others changes as it passes on.
- Large settlements draw with less work each frame (the world is redrawn only where it changed).
- Your 0.1.0 saves load in 0.1.1; updating keeps them.
- Closing the game window now saves your valley first, so a short first visit is not lost
  (before, the first save came about a game day in, roughly 24 minutes at normal speed).
- The Mouse & Keys list opens at the left edge, clear of the settlement, and fits without scrolling.
- The camera director follows what is happening again; in 0.1.0 it stayed still.
- The zip has a `READ ME FIRST.txt`, and Settings has a "Your files and data" page: both list every
  file Ostrog writes on your computer and what it is for. The download page lists them too.
- The news feed at the bottom left and the sounds tied to events work again while the game runs (in
  0.1.0 they only came through when you opened a panel). Important lines now stay longer than small
  ones.
- Updates keep working after the first one: the copy of your previous version that an update keeps
  in case the new one fails to start is now removed once the new version runs, so it no longer
  blocks the next update or fills your disk.
- People only say they saw something strange if they could actually see the spot.
- The Settings sliders show their value, and the autosave setting says when it is off.
- Smaller fixes to the screen: Esc closes the panel on top; only one reading panel is open at a
  time; announcements appear under the top bar and wait while you are reading; Load says when
  nothing is saved yet and asks before replacing the running valley; the top bar counts people who
  are away the same way the People panel does.

## Install

1. Open the download page from the link you were sent and press **Download Ostrog (Windows)**.
   You get `ostrog-windows-x64-portable-c252.zip`. The page also shows its SHA-256 checksum if
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
