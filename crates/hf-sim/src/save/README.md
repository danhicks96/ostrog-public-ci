# Save and load

[Root map](../../../../README.md#architecture-map) · [Chronicle](../chronicle/README.md) · [Bridge](../../../hf-godot/README.md)

[Save module](mod.rs) owns the compatibility boundary. A save package contains a manifest plus the world
snapshot and a separate chronicle. [Snapshot](snapshot.rs) defines durable world state; transient caches
are rebuilt rather than treated as saved history. [Migration](migrate.rs) documents the supported version
transition. Do not infer a format mismatch from a file's name alone.

<!-- BEGIN GENERATED: save-contract -->
<!-- Written by tools/docs/readme_tables.py; edit the code, then rerun it. -->

Current writer format: **2** (see [FORMAT_VERSION](mod.rs)).

<!-- END GENERATED: save-contract -->

On desktop the bridge settles the sim, captures the snapshot and pins the chronicle; background work
commits the package. Time the capture separately from database backup and directory writes. The [client](../../../../godot/README.md)
chooses a valid autosave or explicit save to load. Test with a disposable copy; never run compatibility probes
against the sole copy of an active world. See the [save-memory guard](../../tests/save_memory.rs).
