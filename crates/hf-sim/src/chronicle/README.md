# Chronicle

[Root map](../../../../README.md#architecture-map) · [Simulation](../../README.md) · [Saves](../save/README.md)

[Session](../lib.rs) forwards new [events](../events.rs) by increasing event ID; [the chronicle](mod.rs) records
eligible events in SQLite and indexes entity references. Trace-severity events are excluded. The database distinguishes
real history, taught accounts, and derived summaries. [Viewer read models](../view/) and the [native bridge](../../../hf-godot/README.md)
use those records; they do not change what originally happened.

A save carries the chronicle alongside its world snapshot. World [digest](../digest.rs) does not hash the
chronicle. When optimizing or instrumenting its writer, compare ordered durable event records separately;
measure how much time and storage are spent writing rows, building references and backing up the database.
