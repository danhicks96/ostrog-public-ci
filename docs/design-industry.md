# M4 design: industry, materials and economy

Current continuation note: the M4 material and craft rules remain the foundation, but the later
object breadth is now governed by D57 and `docs/OBJECT_CATALOG.md`. Existing generic item kinds
are not the final catalog; future forms, parts, condition states and custody records must grow
from the real material chains described here and remain traceable in the chronicle.

Programme row M4 in docs/PLAN-FINISH.md. Written 2026-10-18 on top of what exists: 29 item kinds
(items.rs), 9 data recipes at stations (work/recipes.rs: planks, copper and iron bars, tools,
cloth, clothes, poultices), tool wear and storied items (legend.rs), a ledger that balances every
item's creation and use, and caravans to the world's real settlements (C27).

## Findings it answers

- C21: a harvest nobody hauls piles up for years (84,756 sheaves on one seed); food never spoils
  and fields are sown for the peak population.
- M3: the world's settlements have peoples, ways of living and land, but trade carries the same
  fixed goods everywhere.

## The pieces

1. Materials (`materials.rs`). A material table with density, hardness, melting point, value
   and a palette ramp (palette v2 has gold and copper; gems have their own): stones from the
   site's strata (granite, basalt, limestone, sandstone, slate, marble ...), ores and metals
   (iron, copper, tin, silver, gold; bronze as an alloy), woods by tree species, fibres and
   hides by source. Items gain `material: Option<MaterialId>` (serde default): the old kinds
   keep working; new generic kinds (Bar, Block, Weapon, Armour, Furniture items, Vessel) take
   their nature from the material.
2. Chains as recipes people take up (data, as now): smelting and alloying; smithing (tools,
   weapons, armour); carpentry and masonry (furniture by material); pottery and a kiln; glass;
   weaving; tanning and leatherwork; brewing (grain and fruit to drink, meeting the Drink
   conviction people already hold); cooking that remembers ingredients; jewellery from gems.
   Each recipe has a station, a skill and a technique people learn by trying and watching
   (the existing techniques model), never an order.
3. Quality. Skill, tool, material and mood give a quality tier; a masterwork gets a name and a
   story (legend.rs) and shows on its sprite. An artisan possessed by a work (DF's strange mood)
   happens only from a cause in their own past (a formative event about making or loss), and the
   record says which.
4. Engravings that show recorded history. A mason engraving a wall or a carver a table chooses a
   real chronicle row the community remembers (a hearth tale, a death, a raid, an echo) and the
   piece carries its history id; hovering it tells the scene and links to the row. DF's
   signature, and a natural fit for the never-invent rule.
5. Food that spoils and stores that matter. Food lying in the open rots by days and weather
   (sheaves, meat, fruit at their own rates), recorded in the ledger as rotted; granaries,
   cellars, barrels and pots keep it; people judge sowing by what the store holds and how many
   eat, not by the peak population. Preservation is not a binary indoor flag: temperature,
   humidity, pests, container, airflow, salt/smoke/drying technique and the maker's learned
   knowledge matter when those systems exist. A person can remember that an exposed pile spoiled
   and change their practice, while someone else may not know why.
6. Trade with the world. Each real settlement keeps stores from its people's living and land
   (farmers grain, herders hides and wool, miners ore and stone, fisherfolk fish), wants what it
   lacks, and prices from scarcity on both sides; caravans carry real goods and come back with
   what was traded; routes appear on the world map; the ledger balances through every trade.

## Gates (tests)

- `ore_becomes_a_weapon_that_is_worn_and_used` (ore to bar to weapon, carried and worn down).
- `prices_follow_scarcity` (a settlement short of grain pays more for it; a glut pays less).
- `ledger_holds_through_trade_and_rot` (zero drift with spoilage and trade). Built in C36 as
  `barter_moves_goods_and_makes_none` and `stores_are_made_and_rot` (tests/trade.rs), with the
  soak's ledger-drift check on the colony side.
- `sowing_follows_the_store` (fields shrink when the store is full, grow when it runs low).
- `an_engraving_shows_a_real_row` (its history id narrates back to a real event).
- `a_mood_has_a_cause` (every possessed artisan has a formative event behind it).

## Checkpoints

| # | Work |
|---|---|
| C31 | Spoilage and storage; sowing by stores (the C21 finding first) |
| C32 | Materials table, the material field, strata and woods feeding it (done 2026-10-18: docs/reviews/materials-2026-10-18; hides from hunting moved to C34 with tanning) |
| C33 | Chains: smelting and alloys, smithing with weapons and armour, carpentry, masonry (done 2026-10-18: docs/reviews/arms-2026-10-18; walls of each stone moved to M7 with wall art) |
| C34 | Pottery, glass, weaving, tanning, brewing, cooking with ingredients, jewellery (done 2026-10-18: docs/reviews/crafts-2026-10-18; prospecting for tin, silver and gold added) |
| C35 | Quality, masterworks, moods with causes, engravings of recorded history (done 2026-10-18: docs/reviews/masterworks-2026-10-18) |
| C36 | Trade with the world's settlements, prices, routes on the map (done 2026-10-18: docs/reviews/trade-2026-10-18); M4 review and soaks follow as their own step |

Art alongside (M2 track): item icons for every new kind and material (derived sheets with metal
keyed in pure cyan so one sheet gives every metal), workshop sprites, quality marks.
