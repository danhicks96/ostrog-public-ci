# A person's mind

[Simulation](../../README.md) · [Decision loop](../systems/README.md) · [Root map](../../../../README.md#architecture-map)

Start at [mind](mod.rs), [knowledge](knowledge.rs) and [agent state](../agents.rs). The mind represents
what this individual knows and experiences, not everything that exists in the world.
[Perception](../systems/perceive.rs) updates known observations; [decision scoring](../systems/decide.rs)
uses those observations along with needs and traits. [Belief](../systems/beliefs.rs) and [learning](../systems/learn.rs)
consume simulation outcomes. Their future effects flow back through the tick loop.

[Save-memory tests](../../tests/save_memory.rs) cover continuity at the persistence boundary. The
[selected-person Why lens](../why.rs) is diagnostic state, not a substitute for the person's knowledge
or the [durable chronicle](../chronicle/README.md).
