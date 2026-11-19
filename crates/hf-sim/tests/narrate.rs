//! A thing from home reads as one thing with its article (#1727): "Some clothes lay by the portal
//! stone", never "A clothe", for every kind of item.

use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{EventId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

const KINDS: [ItemKind; 49] = [
    ItemKind::Log,
    ItemKind::StoneBlock,
    ItemKind::IronOre,
    ItemKind::CopperOre,
    ItemKind::Coal,
    ItemKind::Berries,
    ItemKind::GrainSheaf,
    ItemKind::Root,
    ItemKind::RawMeat,
    ItemKind::Meal,
    ItemKind::Tool,
    ItemKind::Cloth,
    ItemKind::Hide,
    ItemKind::Firewood,
    ItemKind::Plank,
    ItemKind::CopperBar,
    ItemKind::IronBar,
    ItemKind::Fibre,
    ItemKind::Clothes,
    ItemKind::Poultice,
    ItemKind::Saw,
    ItemKind::Hammer,
    ItemKind::SledgeHammer,
    ItemKind::Chisel,
    ItemKind::Needle,
    ItemKind::Bucket,
    ItemKind::Torch,
    ItemKind::Record,
    ItemKind::Gem,
    ItemKind::Ore,
    ItemKind::Bar,
    ItemKind::Weapon,
    ItemKind::Armour,
    ItemKind::Clay,
    ItemKind::Sand,
    ItemKind::Vessel,
    ItemKind::Leather,
    ItemKind::Ale,
    ItemKind::Wine,
    ItemKind::Jewel,
    ItemKind::Figurine,
    ItemKind::RoughStone,
    ItemKind::Door,
    ItemKind::BedFrame,
    ItemKind::Table,
    ItemKind::Stool,
    ItemKind::Chest,
    ItemKind::Thatch,
    ItemKind::Dye,
];

#[test]
fn a_thing_from_home_reads_naturally_for_every_item_kind() {
    let mut w = World::generate(1, GenParams::small());
    let at = TilePos::new(20, 20, 2);
    let mut lines = Vec::new();
    for (n, kind) in KINDS.into_iter().enumerate() {
        let item = w.create_lone_item(kind, 1, ItemPlace::Ground(at), 0.5, "test");
        let e = Event {
            id: EventId(n as u32 + 1),
            tick: 1000,
            at: Some(at),
            severity: Severity::Notable,
            kind: EventKind::ThingFromHome {
                item,
                maker: "Aelivane".into(),
                from: "Fernmere".into(),
            },
        };
        lines.push((kind, hf_sim::view::narrate::line(&w, &e)));
    }
    for (kind, line) in &lines {
        assert!(
            line.starts_with("A ") || line.starts_with("An ") || line.starts_with("Some "),
            "{kind:?}: {line}"
        );
        assert!(
            line.ends_with(" lay by the portal stone, made by Aelivane in Fernmere."),
            "{kind:?}: {line}"
        );
        for broken in [
            "clothe ", "torche ", "berrie ", "A iron", "A ale", "A armour",
        ] {
            assert!(!line.contains(broken), "{kind:?}: {line}");
        }
    }
    let of = |k: ItemKind| lines.iter().find(|(x, _)| *x == k).unwrap().1.clone();
    assert!(of(ItemKind::Clothes).starts_with("Some clothes lay"));
    assert!(of(ItemKind::Hammer).starts_with("A hammer lay"));
    assert!(of(ItemKind::Torch).starts_with("A torch lay"));
}
