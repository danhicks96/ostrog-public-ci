# Peoples and anatomy definitions

[Data map](../README.md) · [Body model](../../crates/hf-sim/src/body.rs) · [Root map](../../README.md#architecture-map)

[species](species/) contains authored species definitions; [plans](plans/) describes body plans.
[The Rust loader](../../crates/hf-sim/src/species.rs) reads these definitions and provides the
library used during setup and generation. Refer to [peoples design](../../docs/design-peoples.md)
for intent. Changing a definition is different from changing a living person's state; check how
references survive the [save boundary](../../crates/hf-sim/src/save/README.md).
