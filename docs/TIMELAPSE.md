# Watched-view timelapse (#484)

The Timelapse panel (`T`, or the bottom-bar button) records a watched view of the egress point and lets
the viewer review how it changed. Recording is off by default and the choice persists in
`user://settings.cfg`. It is a viewing feature only: it gives no orders, adds no events and never
steps, loads or saves the world.

## What is watched

- **Use current view** watches the current camera position, z-level, zoom and knowledge filter
  (community-known or all visible). The live camera can then move freely.
- **Whole local map** watches the whole map at the settlement's level, fitted to the window when each
  sample is taken.

The watch is stored in every sample, so a resumed or reloaded world keeps watching the same view.

## When samples are taken

Starting recording or changing the watch takes a sample at once. After that a sample is taken at
noon of every calendar day: tick `epoch + 14,400 + k × 28,800` (`docs/TIME.md`: 1,200 ticks an
hour, the day starting at the world's epoch). The client stops stepping at the sample tick, so the
sample is taken at exactly that tick rather than wherever a frame's step of up to 400 ticks ended,
and every daily sample is lit by the same noon sun.

While a sample is taken the client holds stepping, moves the camera to the watch, and draws the
tick's state: shader clocks, sprite animation and water/fire frames come from the tick, weather and
fire particles restart from a tick-derived seed and stay frozen, lightning is suppressed, the levels
above the view and the ruins are read afresh, every entity is rebuilt, and transient effects
(contact flashes, work accents) are left out, because they age in real seconds and are not part of
the state at a tick. HUD, feed and panels are hidden. The live camera, filters and presentation are
restored afterwards.

## What a sample holds

Each sample is a PNG plus a same-named JSON record under
`user://timelapse/world-<identity>/`, where the identity (`SimHost.world_identity`,
`hf_sim::digest::world_identity`) is a hash of the world's seed, name, founding site, epoch,
generation parameters and new-game choices: the same at every tick and in every save of one world,
different between worlds. The JSON records the seed, world identity, exact tick and calendar, the
full deterministic state digest, PNG SHA-256 and size, the watch (mode, camera, z, zoom, filter),
the region in tiles, viewport and image size, whether the tick was a scheduled noon, and the
sample's event markers: the counts and up to eight lines of the record the viewer had been told of
(`SimHost.summary_between`, the same told-rows filter as the away summary) since the sample before
it. Nothing is added to the image that the renderer does not draw from the world.

## Exact replay

The simulation is deterministic and the viewer gives no orders, so re-running the same days reaches
the same states. The series uses that as a check. When the world is reloaded from an earlier save
(F9, a ring save, a later launch), the series is read back from disk and each sample tick reached
again is compared with what is stored:

- the same state digest: the sample is marked **re-run same state**, and when the PNG bytes also
  match, **re-run matched**;
- a different digest: the stored sample and every later one describe a history this world no longer
  has, so they are removed (files included) and the viewer is told.

Review never reads the live world: the panel has no `SimHost`; it shows stored images and the event
markers stored with them, and scrubbing, pinning and playback cannot change or pause the source.

## Bounds on disk and memory

| Bound | Value |
|---|---|
| Image size | at most 1280 × 720 RGB (`Timelapse.CAPTURE_MAX`): larger windows are scaled down with a fixed filter, never up |
| One PNG | at most about 2.8 MB (1280 × 720 × 3 bytes plus PNG framing); measured 0.8 to 0.9 MB for a settlement view at 1280 × 720 |
| One series | at most 120 samples (`MAX_FRAMES`, 120 game-days) and 160 MiB of PNG (`MAX_SERIES_BYTES`), oldest evicted first |
| All series | at most 6 worlds (`MAX_SERIES`): the current one and the five most recently written; older series are removed when a series is opened |
| Worst case on disk | 6 × 160 MiB of PNG plus 720 JSON records of a few KB |
| Memory | the sample records (120 small dictionaries) and the two images on screen in the panel |

The panel shows the retained PNG total against the series bound.

## Review

The panel shows the earliest (or pinned) sample beside the selected one. The slider, the arrow
buttons and the marker strip scrub; each mark on the strip is one sample, taller where more of the
record fell since the sample before. Below the images the panel lists the stored event lines between
the two compared samples. **Pin selected** chooses the comparison reference; review playback pauses
or runs at 0.5, 1, 2 or 4 samples per real second.

## Verification

- `tools/gd_test.gd`: noon scheduling and the step cut (`test_timelapse_samples_fall_on_exact_noon_ticks`),
  tick order, byte bound, replay reconcile, capture size bound and egress point zoom
  (`test_timelapse_byte_bound_order_and_replay`), series reload and pruning
  (`test_timelapse_series_reloads_and_prunes`), event markers and the bridge's identity
  (`test_timelapse_event_markers_and_world_identity`), plus the first slice's ring, panel and
  presentation-clock tests.
- `crates/hf-sim/tests/timelapse_identity.rs`: the identity survives running and a save round trip,
  differs between seeds, and reading it changes nothing.
- `tools/shots/timelapse.sh OUTPUT_DIR` (windowed, Xvfb): same-tick captures give identical PNG
  bytes; the live camera and source are independent of the watch; 121 samples keep 120 with their
  files; four days of samples land on exact noon ticks with event markers; a save taken before them
  reloads into the same series, and re-running the four days confirms every stored digest and PNG
  byte for byte; a tampered digest is reported as a divergence and later samples are removed; a
  whole-egress point sample fits the map; the bytes reported equal the PNG bytes on disk. It writes the
  day images, the egress point image, a screenshot of the panel and `report.json`.

Evidence: `docs/reviews/484/`.

## Coordination with #194

Manual photo mode, the chronicle ribbon and the event replay buffer in #194 stay separate. This
recorder is the automatic, bounded, sampled view; its event markers read the same told record the
away summary and history view use, and add no new event text.
