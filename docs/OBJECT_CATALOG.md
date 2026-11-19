# Ostrog object catalog

This is the breadth target for the object and prop system. The goal is the feeling of a
settlement, ruin or dungeon assembled from understandable things rather than a short list of
gameplay tokens. An object is a combination of a family, form, material, quality, condition,
provenance and optional decoration; those axes should be rendered with overlays and palette
swaps wherever possible instead of multiplying sprite sheets.

The catalog is deliberately broader than the current simulation enums. It is the art and data
contract for the next object pass.

Status: this is an adopted design contract, not an inventory of everything already implemented.
The current slice is the generic object vocabulary, existing storied-item history, and a guarded
person-inspection view for equipped, worn and carried items (PR #459). Universal item identity,
complete custody coverage, assembly/disassembly links and the long-tail families below remain
planned work and must be delivered with tests and truthful history.

## Object record

Every placeable or discoverable object should be able to carry:

| Field | Examples |
| --- | --- |
| family | tool, furniture, gate, relic, rubble |
| form | axe, chair, portcullis, crown, broken beam |
| material | oak, granite, bronze, iron, glass, bone, hide |
| quality | crude, plain, fine, excellent, masterwork, legendary |
| condition | new, worn, damaged, broken, ruined, repaired |
| provenance | local, abandoned, ancient, royal, dwarven, fey, unknown |
| function | cuts, stores, cooks, blocks, illuminates, records, decorates |
| placement | carried, ground, furniture, wall, floor, ceiling, stack, fixture |
| decoration | inlay, engraving, banner, trim, paint, moss, rust, blood, ivy |
| assembly | part links, parent object, construction stage, repair/disassembly links |
| origin | construction era, site, builder or maker event when known |
| knowledge | observed condition, confidence, source and last observation per person |

Quality, condition and provenance are not just adjectives. They should affect silhouette,
highlight, edge wear, missing pieces, material overlays and the inspection text.

`intact` or `complete` means that no required part is missing or damaged. `perfect` is a visual
condition meaning complete and unblemished; it is not the same as masterwork quality. Quality is
workmanship, condition is wear or damage, and provenance is history. A person may know only an
observed or inferred condition, while the world record retains the authoritative condition.

When a player clicks a person, the inspect panel should expose their equipment, worn items and
inventory as individual records. For each record, show the readable name, material, quality,
condition, maker or origin, creation time, current location and owner chain. Stacks can remain
aggregated for ordinary materials, but named, crafted, worn, broken or otherwise storied things
must remain individual items. This is the bridge between the object catalog and a DF-like chain
of custody: the item is not just “a spear”, it is the same spear that was made, carried, used,
repaired, inherited and eventually found in a ruin.

## 1. Natural resources and loose materials

Logs, branches, sticks, bark, roots, fibre, reeds, grass bundles, thatch, straw, seeds,
flowers, herbs, mushrooms, berries, fruit, nuts, grain sheaves, clay, sand, mud, peat, salt,
ash, charcoal, snow, ice, water, clean water, dirty water, rough stone, rubble, gravel, flint,
coal, peat fuel, bone, teeth, horn, shell, feathers, hides, leather, sinew, wool, cotton, silk,
amber, resin, wax, sap, dyes, pigments, glass shards, slag, mortar, lime, bricks and tiles.

Material variants should cover common woods, stones, clays, metals and organic sources already
present in the world rather than inventing a new art family for each recipe.

## 2. Parts, fittings and crafting components

Planks, beams, poles, dowels, handles, shafts, hafts, bow limbs, bowstrings, grips, blades,
heads, teeth, hooks, nails, rivets, bolts, hinges, latches, locks, keys, chains, wire, plates,
rings, bands, springs, gears, axles, wheels, rope, cord, thread, cloth squares, leather
straps, buckles, padding, glass panes, bottles, jars, pots, lids, lids with handles, mortar,
bricks, stone blocks, cut stone, carved stone, brackets, stairs, railings, shutters and doors.

These are the cobble-together layer: a completed object is assembled from a form plus a set of
parts, and a damaged object can expose or drop those parts.

## 3. Tools and implements

Axes, hatchets, adzes, saws, bowsaws, handsaws, sledgehammers, hammers, mallets, chisels,
gouges, files, rasps, drills, augers, knives, sickles, scythes, hoes, rakes, shovels, spades,
pickaxes, mattocks, crowbars, trowels, needles, awls, shears, scissors, pincers, pliers,
tongs, ladles, buckets, baskets, sacks, crates, wheelbarrows, carts, fishing rods, nets,
trap kits, measuring rods, levels, compasses, torches, lanterns, braziers, quills, inkpots,
brushes, looms, spinning tools and repair kits.

Tools need handle, head and condition overlays so a worn axe and a broken axe can share the same
base silhouette.

## 4. Weapons, armour and battlefield objects

### Weapons

Clubs, cudgels, quarterstaves, spears, pikes, halberds, tridents, daggers, knives, swords,
shortswords, longswords, sabres, rapiers, axes, battleaxes, warhammers, maces, flails,
morningstars, whips, slings, bows, shortbows, longbows, crossbows, bolts, arrows, javelins,
throwing knives, nets, shields, bucklers, tower shields, siege stones, fire pots, caltrops,
smoke bombs, signal horns and banners.

### Protection

Helmets, caps, hoods, mail shirts, scale, plate, breastplates, gorgets, gauntlets, bracers,
greaves, boots, shields, cloaks, padded armour, bone armour, hide armour, mining helmets,
work aprons and ceremonial armour.

### Battlefield remains

Weapon racks, armour stands, quiver racks, arrow bundles, spent arrows, broken shafts, dropped
shields, weapon piles, bloodstains, scorch marks, barricades, stakes, caltrop fields, siege
debris, fallen standards and abandoned camps.

The current generic `weapon` and `armour` kinds are useful foundations, but the catalog should
eventually distinguish weapon silhouettes and ammunition families.

## 5. Clothing, personal gear and containers

Tunics, shirts, workshirts, dresses, skirts, trousers, trousers with belts, aprons, coats,
cloaks, robes, hoods, hats, caps, gloves, mitts, scarves, belts, pouches, backpacks, satchels,
bedrolls, blankets, towels, boots, sandals, masks, goggles, spectacles, jewellery, amulets,
badges, signet rings, wedding rings, backpacks, quivers, tool belts, canteens and travel packs.

Clothing should use the existing species/body/age/direction layering. Wear, mud, blood, rain,
patches, missing fasteners and dye should be overlays or palette swaps.

## 6. Food, drink, medicine and domestic goods

Raw meat, fish, eggs, milk, cheese, butter, bread, flour, porridge, stew, roast meat, dried
meat, smoked fish, preserves, jam, pickles, cakes, pies, fruit, vegetables, mushrooms, meals,
ale, wine, cider, mead, spirits, tea, coffee, juice, water, poison, antidote, poultice, salve,
bandages, splints, medicine bundles, herbs, syringes, surgical tools, soap, candles, oil,
perfume, ink, paper, parchment, books, scrolls, maps and recipe bundles.

## 7. Furniture, storage and domestic fixtures

Beds, bedframes, bedrolls, cribs, mattresses, blankets, chairs, stools, benches, tables,
desks, counters, shelves, cabinets, wardrobes, chests, crates, barrels, bins, baskets, sacks,
stockpiles, cupboards, dressers, lecterns, mirrors, washbasins, tubs, toilets, wells, sinks,
fireplaces, hearths, ovens, stoves, braziers, lamps, lantern hooks, candleholders, chandeliers,
clocks, musical instruments, looms, spinning wheels, drying racks, butcher blocks, tables with
tools, writing desks and display cases.

## 8. Workshops and production stations

Carpenter benches, workbenches, stonecutting benches, masonry tables, anvils, forges, smelters,
blast furnaces, kilns, pottery wheels, glass furnaces, cooking pots, stoves, breweries,
tanneries, spinning wheels, looms, dye vats, sewing tables, leatherworking benches, kitchens,
butcher tables, herbalist tables, alchemy tables, enchanting tables, gemcutting benches,
mechanist benches, sawpits, waterwheels, windmills, presses, grinders, mills and repair stands.

Each workstation should have idle, active, damaged and abandoned states, with fuel, heat,
smoke, sparks, steam, water, ash and residue as optional attachments.

## 9. Architecture, infrastructure and fixtures

Walls, wall corners, pillars, columns, beams, floors, ceilings, roofs, rafters, stairs, ladders,
ramps, bridges, railings, fences, gates, doors, shutters, windows, portcullises, drawbridges,
trapdoors, hatches, wells, drains, channels, aqueducts, roads, paths, bridges, docks, mooring
posts, signs, posts, scaffolds, construction frames, rubble piles, foundations, ruins and
collapsed sections.

Architectural objects need material, edge, damage and construction-progress variants. A wall
should be able to become cracked, mossy, burned, breached, collapsed or repaired without losing
its original material identity.

## 10. Lighting, signals and decoration

Candles, sconces, lamps, lanterns, braziers, torches, firepits, hearth fires, altar flames,
signal fires, beacons, glow fungus, crystals, moonstones, banners, flags, tapestries, rugs,
paintings, statues, busts, carvings, idols, shrines, standing stones, fountains, planters,
flowerpots, garden ornaments, signs, heraldry, trophies, bones, skulls, antlers, cobwebs, ivy,
moss, vines and seasonal decorations.

## 11. Books, records and knowledge objects

Books, ledgers, letters, journals, maps, charts, scrolls, tablets, bark records, stone tablets,
inscriptions, gravestones, seals, stamps, contracts, recipe books, technique manuals, relic
catalogs, genealogies, royal decrees, expedition logs, coded notes and incomplete fragments.

Written objects should have blank, readable, damaged, sealed, burned and ancient visual states.

## 12. Ruins, castles and dungeons

### Ruined settlement remains

Collapsed houses, broken roofs, wall stubs, door fragments, shattered furniture, abandoned
workshops, cold hearths, cracked wells, empty stockpiles, old fields, overgrown paths, graves,
cemeteries, memorial stones, burned foundations, carts, broken tools, rusted weapons, lost
clothing, old cooking gear and buried storage jars.

### Castle and fortress remains

Battlements, crenellations, murder holes, arrow slits, portcullises, drawbridges, moat pieces,
guard posts, barracks, armouries, weapon racks, throne rooms, chapels, crypts, kitchens,
pantries, cellars, towers, spiral stairs, oubliettes, prison cells, torture-room remnants,
banner poles, royal seals, treasure rooms, vault doors and collapsed courtyards.

### Dungeon remains

Prison doors, barred gates, cages, chains, shackles, sarcophagi, coffins, tombs, altars,
ritual circles, secret doors, pressure plates, traps, levers, gears, chains, spikes, pits,
poison vents, arrow traps, dart traps, swinging blades, flood gates, ancient lamps, strange
fungus, bones, webs, monster nests, discarded adventurer gear, map fragments and sealed rooms.

### Ancient and exceptional objects

Relic weapons, named armour, crowns, sceptres, seals, keys, tablets, masks, idols, crystal
devices, star charts, musical instruments, memory stones, cursed objects, blessed objects,
unfinished artifacts, broken relics, counterfeit relics, dormant machines and objects whose
function must be inferred from parts or inscriptions.

## 13. Traps, machines and interactive mechanisms

Levers, pull chains, buttons, switches, pressure plates, locks, keys, keyholes, gears, wheels,
cranks, pistons, pumps, valves, doors, secret doors, trapdoors, shutters, bells, alarms,
tripwires, snares, cages, spike traps, dart traps, arrow traps, fire traps, gas vents, water
channels, lifts, elevators, bridges, counterweights, puzzle mechanisms and broken mechanisms.

These should be assembled from visible parts where possible: a missing gear or snapped chain is
more interesting than a disabled boolean.

## 14. Quality and condition language

The minimum visual state set is:

`rough -> plain -> fine -> excellent -> masterwork -> legendary`

and:

`new -> used -> worn -> damaged -> broken -> ruined -> repaired`

Useful overlays include chipped corners, bent edges, missing boards, exposed stuffing, rust,
soot, moss, dust, blood, frost, cobwebs, stains, patched cloth, replacement parts, maker marks,
engraving, inlay and a small quality glint. A legendary object should be recognisable by its
silhouette and provenance before the player reads its name.

## 15. Implementation order

1. Freeze this catalog as data, including stable IDs and aliases for the current enums.
2. Map every existing item and structure to a catalog record; report unmapped simulation kinds.
3. Make provenance universal for item records, with stable creation events, custody transfers,
   assembly/disassembly links and a player-facing history view.
4. Add component parts and state overlays: broken, worn, ruined, repaired and construction stage.
5. Expand the high-frequency gameplay families: tools, weapons, clothing, food, containers and
   workstations.
6. Add castle, dungeon and ruin prop kits built from the same parts and materials.
7. Add named/ancient objects and procedural decoration only after the ordinary object language
   is coherent.

The art pipeline should generate one base silhouette per form, then compose material, wear,
quality, provenance and decoration. That is the route to a large RimWorld/DF-scale catalog
without creating thousands of hand-maintained sprite sheets.
