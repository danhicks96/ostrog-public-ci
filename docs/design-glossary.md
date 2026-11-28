# Glossary — what we call things

The owner chose the vocabulary below. Put it in the interface and in what the game says; keep
internal type names as they are.

| We say | Meaning | We do **not** say |
| --- | --- | --- |
| **survivor** | One of the ten who came through the portal. In code: an `Agent` of `Species::Human`. | pawn, colonist, settler, dwarf, duplicant |
| **starting survivors** | The three who begin together at the egress point. | starting party (ambiguous: all ten are the party) |
| **the missing** | The other seven, scattered alive or dead, unknown until someone goes and looks. | the lost ones (sounds metaphysical) |
| **Oakfolk** | The community's own name, as shown in the HUD. | the colony, the settlement (fine in code comments, not in the HUD) |
| **Engine** | The pacing model chosen at setup. It leans on chance; it fires no events. | storyteller, director (the director is the camera) |
| **the arrival** | The setup screen: reroll ten, choose three, choose the Engine. | character creator, party select |
| **object record** | The stable description of a physical thing: family/form, material, quality, condition, parts, placement and true history. | token, generic item |
| **custody** | A recorded change of who holds, owns, carries, wears or discovers an object. | invented ownership, inferred history |

## Why these words

"Survivor" says what is true of all ten without saying where they are or what they owe anyone. It
is also nobody else's word, which keeps us clear of RimWorld and Dwarf Fortress not just legally
but in the reader's head.

## Rules that follow

- No user-facing string may use the forbidden column. `grep -rniE 'pawn|colonist|settler|\bdwarf' godot/scripts/ godot/scenes/` must come back empty.
- Where the game says *why* something happened, say it in a survivor's terms (what they saw, remembered, or were told), never in system terms ("grabEnabled", "pressure 0.4").
- The Engine never gets credited in the world's voice. In-world, nobody knows there is one. It appears in the HUD because the viewer is outside the world.
- Unknown object history is shown as unknown. The interface may summarize a recorded chain, but it
  must not fill in a maker, age or owner just to make a tooltip sound complete.
