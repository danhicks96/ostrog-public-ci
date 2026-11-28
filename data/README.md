# Authored simulation data

[Root map](../README.md#architecture-map) · [Simulation](../crates/hf-sim/README.md) · [Content design](../docs/design-content-data.md)

[Peoples](peoples/README.md) includes [species definitions](peoples/species/) and [body plans](peoples/plans/).
Start at [Rust species ingestion and generation](../crates/hf-sim/src/species.rs) and
[world setup](../crates/hf-sim/src/setup.rs). These definitions shape a new simulation;
they are not the live state of an existing person. Check [save compatibility](../crates/hf-sim/src/save/README.md)
before changing identifiers or meanings consumed by persistent state.
Generated art and sound follow a [different pipeline](../tools/README.md).
