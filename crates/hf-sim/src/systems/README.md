# People and the tick pipeline

[Root map](../../../../README.md#architecture-map) · [Simulation](../../README.md) · [History](../chronicle/README.md)

[The pipeline](mod.rs) is the authoritative ordering. Follow one decision:

1. [Perception](perceive.rs) updates the agent's local [knowledge](../mind/knowledge.rs) (see the [mind map](../mind/README.md)). A person is not omniscient.
2. [Needs](needs.rs) changes pressures; [decision scoring](decide.rs) considers eligibility, known targets and goal weights.
3. [Action](act.rs), [work](work.rs) and [movement](movement.rs) attempt the choice. A plan and its actual outcome are different.
4. [Flush](flush.rs) exposes recorded events to observers for effects such as [feelings](feel.rs), [beliefs](beliefs.rs) and [learning](learn.rs).
   Those updates affect later choices and can change what the person knows and does.
5. The [bridge](../../../hf-godot/README.md) reads current state for the client; the [chronicle](../chronicle/README.md) stores eligible events.

The selected-person [Why lens](../why.rs) is short-lived and not saved. The separate
[debug decision logger](../debuglog.rs) is opt-in and does not reconstruct a complete action history.
Use [decision lens tests](../../tests/why_lens.rs), [plan failure tests](../../tests/failed_plans.rs)
and the [CLI profiler](../../../hf-cli/README.md#measure-before-optimizing) before changing these paths.
