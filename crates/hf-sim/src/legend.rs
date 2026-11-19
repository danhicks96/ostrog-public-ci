//! Things with a life (issue #49): who made a tool, of what and when, what it did, who held
//! it, how worn it is, and how it ended.
//!
//! A tool or garment that is made, or taken up for the first time, gets a story. A held tool
//! wears with use and breaks, and the feed tells its history as it goes. Use by one pair of
//! hands makes it cherished: losing it is a grief. Work worth remembering (the walls of houses,
//! the trees for them, the way down to the cellar, other tools made with it) becomes its deeds.
//! A piece made exceptionally well is named when it is made; a cherished tool with deeds enough
//! earns a name; a named tool with many deeds is spoken of. When its keeper dies, a cherished
//! or named tool passes to their kin as an heirloom; anything else stays where they fell.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, ItemId, TilePos};
use crate::items::{ItemKind, ItemPlace};
use crate::rng::{purpose, stream};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub maker: Option<AgentId>,
    /// "Halgar", or how it came to be here ("brought through the portal").
    pub maker_name: String,
    pub made: u64,
    pub uses: u32,
    /// 1 new .. 0 broken.
    pub wear: f32,
    pub name: Option<String>,
    /// Who holds it now, and everyone who has: (who, name, since).
    pub owners: Vec<(AgentId, String, u64)>,
    /// Uses by the present keeper.
    pub keeper_uses: u32,
    pub cherished: bool,
    pub heirloom: bool,
    pub spoken_of: bool,
    /// Who it was cherished by before the present heir took it up, and their name, when an
    /// heirloom kept its sentiment because the heir was the keeper's kin (#1121).
    #[serde(default)]
    pub keepsake_of: Option<(AgentId, String)>,
    /// Counts behind the deeds.
    pub walls: u32,
    pub trees: u32,
    pub stone: u32,
    pub roofs: u32,
    pub made_things: u32,
    /// What it is remembered for, oldest first: (tick, words).
    pub deeds: Vec<(u64, String)>,
    /// How it came to lie where it was found (deep.rs: "left at Ashton when ..."), or "".
    #[serde(default)]
    pub origin: String,
    /// What someone could learn by studying it (C13), if anything.
    #[serde(default)]
    pub teaches: Option<crate::mind::techniques::Technique>,
    /// A weapon's blows struck at strangers, armour's blows taken (C33).
    #[serde(default)]
    pub fights: u32,
    /// Game a weapon brought down (C33).
    #[serde(default)]
    pub hunts: u32,
}

/// Kinds that get a story.
pub fn storied(k: ItemKind) -> bool {
    matches!(
        k,
        ItemKind::Tool
            | ItemKind::Saw
            | ItemKind::Hammer
            | ItemKind::SledgeHammer
            | ItemKind::Chisel
            | ItemKind::Needle
            | ItemKind::Bucket
            | ItemKind::Clothes
            | ItemKind::Record
            | ItemKind::Weapon
            | ItemKind::Armour
            | ItemKind::Jewel
            | ItemKind::Figurine
    )
}

/// Uses by one keeper before a tool is cherished. Measured (C12): the busiest keeper reaches
/// 47 to 59 uses by day five and plateaus at 67 to 89 by day twenty, so seventy is a tool
/// cherished somewhere in the second or third week, on most seeds.
pub const CHERISH_USES: u32 = 70;
/// Deeds before a cherished tool earns a name, and before a named one is spoken of.
pub const NAME_DEEDS: usize = 3;
// Three families of deeds, not five (#233: with five, nothing was spoken of in 1,500 soak
// days; tools break before they do five kinds of work).
pub const LEGEND_DEEDS: usize = 3;
/// Days a tool must have been in this valley before people speak of it.
pub const LEGEND_DAYS: u64 = 10;
/// Quality at which a piece is named when it is made.
pub const FINE: f32 = 0.85;

pub fn when(w: &World, tick: u64) -> String {
    let c = crate::time::Calendar::from_tick(tick.saturating_sub(w.meta.epoch));
    // People count years from the arrival, starting at one (as the HUD does).
    format!("{:?} of year {}", c.season, c.year + 1).to_lowercase()
}

/// The words for what it is: "copper tool", "hammer", "clothes".
pub fn noun(k: ItemKind, m: crate::world::Material) -> String {
    use crate::world::Material as M;
    match k {
        // Worlds saved before C32 made tools of the ore's material; newer ones of the metal or
        // the stone itself.
        ItemKind::Tool => match m {
            M::IRON_ORE | M::IRON => "iron tool".into(),
            M::COPPER_ORE | M::COPPER => "copper tool".into(),
            M::BRONZE => "bronze tool".into(),
            M::GRANITE
            | M::LIMESTONE
            | M::SANDSTONE
            | M::SLATE
            | M::BASALT
            | M::GABBRO
            | M::GNEISS
            | M::MARBLE => "stone tool".into(),
            _ => "tool".into(),
        },
        ItemKind::SledgeHammer => "sledgehammer".into(),
        ItemKind::Weapon => format!("{} spear", metal(m)),
        ItemKind::Jewel => format!("{} ring", metal(m)),
        ItemKind::Figurine => {
            let name = crate::world::MaterialTable::default().get(m).name.clone();
            if m == crate::world::Material::AIR {
                "carved figure".into()
            } else {
                format!("{name} figure")
            }
        }
        ItemKind::Armour => match m {
            M::BRONZE => "bronze scale coat".into(),
            _ => format!("coat of {} mail", metal(m)),
        },
        // Kinds whose label is plural stay so (#233: "took up Saewen's clothe").
        ItemKind::Clothes => "clothes".to_string(),
        other => other.label().trim_end_matches('s').to_string(),
    }
}

/// A metal's plain name for a thing's words ("iron"); "metal" for one with none recorded.
fn metal(m: crate::world::Material) -> &'static str {
    use crate::world::Material as M;
    match m {
        M::IRON | M::IRON_ORE => "iron",
        M::COPPER | M::COPPER_ORE => "copper",
        M::BRONZE => "bronze",
        M::TIN => "tin",
        M::SILVER => "silver",
        M::GOLD => "gold",
        _ => "metal",
    }
}

/// A name of the kind people give a tool they are proud of, not one the valley has given
/// already (#227: two oak figures by one maker were both "Ironmourner"; the pool is fifty a
/// kind, so by the sixth a repeat was likelier than not).
pub fn tool_name(seed: u64, id: ItemId, k: ItemKind, taken: &[String]) -> String {
    let mut r = stream(seed, &[purpose::LEGEND, id.0 as u64]);
    let a = [
        "Stone", "Oak", "Iron", "Long", "True", "Old", "Bright", "Grey", "Red", "Black",
    ];
    let b: &[&str] = match k {
        ItemKind::Hammer | ItemKind::SledgeHammer => &["biter", "song", "fist", "heart", "knock"],
        ItemKind::Saw | ItemKind::Chisel => &["tooth", "edge", "bite", "line", "grain"],
        ItemKind::Needle => &["thread", "eye", "stitch", "point"],
        ItemKind::Clothes => &["cloak", "weave", "warden", "mantle"],
        ItemKind::Weapon => &["thorn", "sting", "reach", "fang", "warden"],
        ItemKind::Armour => &["shell", "ward", "hide", "wall", "keeper"],
        ItemKind::Jewel => &["star", "promise", "ember", "tear", "vow"],
        ItemKind::Figurine => &["watcher", "sleeper", "keeper", "mourner", "dreamer"],
        _ => &["biter", "edge", "hand", "heart", "tooth", "wright"],
    };
    let mut name = String::new();
    for _ in 0..(a.len() * b.len()) {
        name = format!(
            "{}{}",
            a[r.gen_range(0..a.len())],
            b[r.gen_range(0..b.len())]
        );
        if !taken.iter().any(|t| t == &name) {
            return name;
        }
    }
    // Every pair given: the younger of the two.
    format!("{name} the younger")
}

/// The names the valley's things already carry.
pub fn names_given(w: &World) -> Vec<String> {
    w.items
        .iter()
        .filter_map(|it| it.story.as_ref().and_then(|s| s.name.clone()))
        .collect()
}

/// Give an item a story if it should have one and does not.
pub fn ensure(w: &mut World, id: ItemId, maker: Option<usize>, how: &str) {
    let tick = w.meta.tick;
    let (maker_id, maker_name) = match maker {
        Some(i) => (Some(w.agents[i].id), w.agents[i].name.clone()),
        None => (None, how.to_string()),
    };
    if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
        if it.story.is_none() && storied(it.kind) {
            it.story = Some(Box::new(Story {
                maker: maker_id,
                maker_name,
                made: tick,
                wear: 1.0,
                ..Default::default()
            }));
        }
    }
}

/// A storied piece was just made by agent `i`: a fine one is named, and the feed says so.
pub fn made(w: &mut World, id: ItemId, i: usize) {
    ensure(w, id, Some(i), "");
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let taken = names_given(w);
    let Some(it) = w.items.iter_mut().find(|it| it.id == id) else {
        return;
    };
    if it.quality < FINE {
        return;
    }
    let name = tool_name(seed, id, it.kind, &taken);
    let what = noun(it.kind, it.material);
    if let Some(s) = it.story.as_mut() {
        s.name = Some(name.clone());
    }
    let who = w.agents[i].id;
    let at = w.agents[i].pos;
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::ItemNamed {
            agent: who,
            name,
            what,
            because: "made".into(),
        },
    );
}

/// Agent `i` takes up item `id` (a tool from the store, or an heirloom).
///
/// FIXLOG 2026-11-15 #1121: an heirloom passed to kin or a partner stays cherished; it is dear
/// for whose it was, not for how long the heir has used it. A stranger picking up an ordinary
/// tool still starts it fresh.
pub fn take_up(w: &mut World, i: usize, id: ItemId) {
    ensure(w, id, None, "of unknown make");
    let tick = w.meta.tick;
    let who = w.agents[i].id;
    let name = w.agents[i].name.clone();
    let agent = &w.agents[i];
    let mut kin: Vec<AgentId> = agent.children.clone();
    kin.extend(agent.partner);
    kin.extend(agent.mother);
    kin.extend(agent.father);
    if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
        it.place = ItemPlace::Held(who);
        if let Some(s) = it.story.as_mut() {
            if s.owners.last().map(|o| o.0) != Some(who) {
                let keepsake = s
                    .heirloom
                    .then(|| s.owners.last().cloned())
                    .flatten()
                    .filter(|o| kin.contains(&o.0));
                s.owners.push((who, name, tick));
                s.keeper_uses = 0;
                if let Some((pid, pname, _)) = keepsake {
                    // Stays cherished: the heir's own hands didn't earn this, but their kinship
                    // to the last keeper did.
                    s.keepsake_of = Some((pid, pname));
                } else {
                    s.cherished = false;
                    s.keepsake_of = None;
                }
            }
        }
    }
    w.agents[i].tool = Some(id);
}

/// One piece of work done by agent `i` with the tool in hand.
pub fn used(w: &mut World, i: usize, kind: crate::work::DesignationKind) {
    let taken = names_given(w);
    use crate::work::DesignationKind as K;
    let Some(id) = w.agents[i].tool else { return };
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let who = w.agents[i].id;
    let at = w.agents[i].pos;
    let mut news: Vec<EventKind> = Vec::new();
    {
        let Some(it) = w.items.iter_mut().find(|it| it.id == id) else {
            w.agents[i].tool = None;
            return;
        };
        let what = noun(it.kind, it.material);
        let item_kind = it.kind;
        let Some(s) = it.story.as_mut() else { return };
        s.uses += 1;
        s.keeper_uses += 1;
        let deed = |n: u32, steps: [u32; 3], words: [&str; 3]| -> Option<String> {
            steps
                .iter()
                .position(|x| *x == n)
                .map(|k| words[k].to_string())
        };
        let new_deed = match kind {
            K::BuildWall(_) | K::BuildDoor => {
                s.walls += 1;
                deed(
                    s.walls,
                    [22, 66, 150],
                    [
                        "raised the walls of a house",
                        "raised the walls of three houses",
                        "raised half the walls in the valley",
                    ],
                )
            }
            K::BuildRoof(_) => {
                s.roofs += 1;
                deed(
                    s.roofs,
                    [20, 60, 140],
                    [
                        "roofed a house",
                        "roofed three houses",
                        "roofed most of the valley",
                    ],
                )
            }
            K::Chop => {
                s.trees += 1;
                deed(
                    s.trees,
                    [15, 50, 120],
                    [
                        "felled the timber for a house",
                        "felled a stand of trees",
                        "felled a forest's worth",
                    ],
                )
            }
            K::Mine | K::DigStairs => {
                s.stone += 1;
                if matches!(kind, K::DigStairs) && !s.deeds.iter().any(|d| d.1.contains("way down"))
                {
                    Some("dug the way down".to_string())
                } else {
                    deed(
                        s.stone,
                        [20, 60, 150],
                        [
                            "cut a cellar",
                            "cut halls in the rock",
                            "cut deep into the hill",
                        ],
                    )
                }
            }
            K::Craft(_) => {
                s.made_things += 1;
                deed(
                    s.made_things,
                    [10, 40, 100],
                    [
                        "made a store of goods",
                        "made the valley's tools",
                        "made more than anyone remembers",
                    ],
                )
            }
            _ => None,
        };
        if let Some(d) = new_deed {
            // A greater deed of the same kind replaces the lesser ("three houses" for "a house").
            let family: String = d.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
            s.deeds.retain(|x| !x.1.starts_with(&family));
            s.deeds.push((tick, d));
        }
        if !s.cherished && s.keeper_uses >= CHERISH_USES {
            s.cherished = true;
            news.push(EventKind::Cherished {
                agent: who,
                what: what.clone(),
                name: s.name.clone(),
            });
        }
        if s.name.is_none() && s.cherished && s.deeds.len() >= NAME_DEEDS {
            let n = tool_name(seed, id, item_kind, &taken);
            s.name = Some(n.clone());
            news.push(EventKind::ItemNamed {
                agent: who,
                name: n,
                what: what.clone(),
                because: s
                    .deeds
                    .last()
                    .map(|d| d.1.clone())
                    .unwrap_or_else(|| "has done much".into()),
            });
        }
        if !s.spoken_of
            && s.name.is_some()
            && s.deeds.len() >= LEGEND_DEEDS
            && tick.saturating_sub(s.made) >= LEGEND_DAYS * crate::time::TICKS_PER_DAY
        {
            s.spoken_of = true;
            news.push(EventKind::SpokenOf {
                name: s.name.clone().unwrap_or_default(),
                what,
                deeds: s.deeds.iter().map(|d| d.1.clone()).collect(),
            });
        }
    }
    for k in news {
        let sev = if matches!(k, EventKind::SpokenOf { .. }) {
            Severity::Major
        } else {
            Severity::Notable
        };
        w.events.push(tick, Some(at), sev, k);
    }
}

/// The tool in agent `i`'s hand wears by `by`; returns false if it broke.
pub fn wear(w: &mut World, i: usize, by: f32) -> bool {
    let Some(id) = w.agents[i].tool else {
        return true;
    };
    let tick = w.meta.tick;
    let Some(ix) = w.items.iter().position(|it| it.id == id) else {
        w.agents[i].tool = None;
        return true;
    };
    let left = {
        let s = w.items[ix].story.get_or_insert_with(|| {
            Box::new(Story {
                wear: 1.0,
                maker_name: "of unknown make".into(),
                made: tick,
                ..Default::default()
            })
        });
        s.wear -= by;
        s.wear
    };
    w.agents[i].tool_wear = left.max(0.0);
    if left > 0.0 {
        return true;
    }
    // Broken: the feed tells what it was.
    let it = w.items[ix].clone();
    let s = it.story.clone().unwrap_or_default();
    let who = w.agents[i].id;
    let at = w.agents[i].pos;
    w.consume_item(id, 1, "worn out");
    w.agents[i].tool = None;
    w.agents[i].tool_quality = 0.0;
    w.agents[i].tool_wear = 0.0;
    if s.cherished || s.name.is_some() {
        w.agents[i]
            .thoughts
            .add(crate::mind::ThoughtKind::LostTreasure, tick, None);
    }
    w.events.push(
        tick,
        Some(at),
        if s.cherished || s.name.is_some() {
            Severity::Notable
        } else {
            Severity::Minor
        },
        EventKind::ToolBroke {
            agent: who,
            tier: ItemKind::tier(it.quality),
            what: s.name.clone().unwrap_or_else(|| noun(it.kind, it.material)),
            history: history(w, &it.kind, &s),
        },
    );
    false
}

/// An item's history for the feed (": Halgar made it ..."), or "" without a story.
pub fn history_of(w: &World, it: &crate::items::Item) -> String {
    it.story
        .as_ref()
        .map(|s| history(w, &it.kind, s))
        .unwrap_or_default()
}

/// ": Halgar made it in spring of year 1, and it raised the walls of a house" (or "").
fn history(w: &World, _k: &ItemKind, s: &Story) -> String {
    let made = match s.maker {
        Some(_) => format!("{} made it in {}", s.maker_name, when(w, s.made)),
        None => String::new(),
    };
    let deeds: Vec<&str> = s.deeds.iter().map(|d| d.1.as_str()).collect();
    let did = match deeds.len() {
        0 => String::new(),
        1 => format!("it {}", deeds[0]),
        n => format!("it {} and {}", deeds[..n - 1].join(", "), deeds[n - 1]),
    };
    match (made.is_empty(), did.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!(": {made}"),
        (true, false) => format!(": {did}"),
        (false, false) => format!(": {made}, and {did}"),
    }
}

/// A garment with a story worn through (life.rs): its name or "clothes", and its history for
/// the feed (": Halgar made it in spring of year 1", or who made it and how it came here).
pub fn worn_through(w: &World, it: &crate::items::Item, s: &Story) -> (String, String) {
    let what = s.name.clone().unwrap_or_else(|| "clothes".into());
    let mut h = history(w, &it.kind, s);
    if s.maker.is_none() && !s.origin.is_empty() {
        let from = format!("made by {}, {}", s.maker_name, s.origin);
        h = match h.strip_prefix(": ") {
            Some(rest) => format!(": {from}; {rest}"),
            None => format!(": {from}"),
        };
    }
    (what, h)
}

/// One line about a storied item, for inspect and hover.
pub fn describe(w: &World, id: ItemId) -> Option<String> {
    let it = w.items.iter().find(|it| it.id == id)?;
    let s = it.story.as_ref()?;
    let what = noun(it.kind, it.material);
    let head = match &s.name {
        Some(n) => format!("{n}, {}", crate::view::narrate::a_or_an(&what)),
        None => crate::view::narrate::a_or_an(&what),
    };
    let maker = match s.maker {
        // A trophy says how it was taken, not who made it.
        _ if it.kind == ItemKind::Hide => String::new(),
        Some(_) => format!(" {} made in {}", s.maker_name, when(w, s.made)),
        // Found from the valley's past: who made it, and roughly when.
        None if !s.origin.is_empty() => format!(
            " made by {}, {}",
            s.maker_name,
            crate::deep::ago(w.meta.epoch.saturating_sub(s.made) / crate::time::TICKS_PER_YEAR)
        ),
        None => format!(", {}", s.maker_name),
    };
    let worn = match s.wear {
        x if x > 0.8 => "nearly new",
        x if x > 0.5 => "well used",
        x if x > 0.2 => "worn",
        _ => "nearly worn through",
    };
    let mut out = format!("{head}{maker}; {worn}");
    if !s.origin.is_empty() {
        out.push_str(&format!("; {}", s.origin));
    }
    if s.heirloom {
        if let Some((_, pname)) = &s.keepsake_of {
            out.push_str(&format!("; an heirloom, {}'s", pname));
        } else if let Some(first) = s.owners.first() {
            out.push_str(&format!("; an heirloom, once {}'s", first.1));
        }
    }
    if s.cherished {
        out.push_str("; cherished");
    }
    if !s.deeds.is_empty() {
        let d: Vec<&str> = s.deeds.iter().map(|d| d.1.as_str()).collect();
        out.push_str(&format!("; it {}", d.join(", ")));
    }
    out.push('.');
    Some(out)
}

/// A storied item lying at `p`, for hover: "Here lies Stonebiter, a hammer ...".
pub fn ground_note(w: &World, p: TilePos) -> Option<String> {
    let it = w.items.iter().find(|it| {
        it.place == ItemPlace::Ground(p)
            && it.story.is_some()
            && crate::view::known::item_told(w, it)
    })?;
    describe(w, it.id).map(|d| format!("Lying here: {d}"))
}

/// Every tick: whoever died this tick leaves their tool. A cherished or named one goes to their
/// kin as an heirloom; anything else stays where they fell.
pub fn step(w: &mut World) {
    let dead: Vec<(AgentId, Option<TilePos>)> = w
        .last_events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Died { agent, .. } => Some((agent, e.at)),
            _ => None,
        })
        .collect();
    for (who, at) in &dead {
        pieces_of_the_dead(w, *who, *at);
    }
    for (who, at) in dead {
        let Some(di) = w.agents.iter().position(|a| a.id == who) else {
            continue;
        };
        let Some(id) = w.agents[di].tool.take() else {
            continue;
        };
        let pos = at.unwrap_or(w.agents[di].pos);
        let keep = w
            .items
            .iter()
            .find(|it| it.id == id)
            .and_then(|it| it.story.as_ref())
            .map(|s| s.cherished || s.name.is_some())
            .unwrap_or(false);
        let a = &w.agents[di];
        let mut kin: Vec<AgentId> = a.children.clone();
        kin.extend(a.partner);
        kin.extend(a.mother);
        kin.extend(a.father);
        let heir = if keep {
            kin.into_iter().find(|k| {
                w.agents
                    .iter()
                    .any(|x| x.id == *k && x.alive && x.community.is_some())
            })
        } else {
            None
        };
        match heir {
            Some(h) => {
                let hi = w.agents.iter().position(|x| x.id == h).unwrap();
                // Whatever the heir held goes back on the ground beside them.
                if let Some(old) = w.agents[hi].tool.take() {
                    let hp = w.agents[hi].pos;
                    if let Some(it) = w.items.iter_mut().find(|it| it.id == old) {
                        it.place = ItemPlace::Ground(hp);
                    }
                }
                if let Some(s) = w
                    .items
                    .iter_mut()
                    .find(|it| it.id == id)
                    .and_then(|it| it.story.as_mut())
                {
                    s.heirloom = true;
                }
                take_up(w, hi, id);
                let q = w
                    .items
                    .iter()
                    .find(|it| it.id == id)
                    .map(|it| it.quality)
                    .unwrap_or(0.5);
                w.agents[hi].tool_quality = q.max(0.05);
                let wear = w
                    .items
                    .iter()
                    .find(|it| it.id == id)
                    .and_then(|it| it.story.as_ref())
                    .map(|s| s.wear)
                    .unwrap_or(1.0);
                w.agents[hi].tool_wear = wear;
                let what = w
                    .items
                    .iter()
                    .find(|it| it.id == id)
                    .map(|it| {
                        it.story
                            .as_ref()
                            .and_then(|s| s.name.clone())
                            .unwrap_or_else(|| noun(it.kind, it.material))
                    })
                    .unwrap_or_default();
                let tick = w.meta.tick;
                let hp = w.agents[hi].pos;
                w.events.push(
                    tick,
                    Some(hp),
                    Severity::Notable,
                    EventKind::Inherited {
                        agent: h,
                        from: who,
                        what,
                    },
                );
            }
            None => {
                if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
                    it.place = ItemPlace::Ground(pos);
                }
            }
        }
        w.agents[di].tool_quality = 0.0;
    }
}

/// What the dead wore that has a story (a ring given at a pairing, clothes with a name) goes to
/// their nearest living kin as an heirloom, a ring on the hand, clothes set down beside them; with
/// no kin it is left where they fell. The tool is `step`'s, weapons and armour arms.rs's.
/// FIXLOG 2026-10-20 #168, C62: such pieces stayed held by the dead for good, out of reach.
fn pieces_of_the_dead(w: &mut World, who: AgentId, at: Option<TilePos>) {
    let Some(di) = w.agents.iter().position(|a| a.id == who) else {
        return;
    };
    let a = &w.agents[di];
    let pos = at.unwrap_or(a.pos);
    let others = [a.tool, a.weapon, a.armour];
    let pieces: Vec<ItemId> = w
        .items
        .iter()
        .filter(|it| {
            it.place == ItemPlace::Held(who) && it.story.is_some() && !others.contains(&Some(it.id))
        })
        .map(|it| it.id)
        .collect();
    if pieces.is_empty() {
        return;
    }
    let mut kin: Vec<AgentId> = a.children.clone();
    kin.extend(a.partner);
    kin.extend(a.mother);
    kin.extend(a.father);
    let heir = kin.into_iter().find_map(|k| {
        w.agents
            .iter()
            .position(|x| x.id == k && x.alive && x.community.is_some())
    });
    let tick = w.meta.tick;
    for id in pieces {
        let Some(it) = w.items.iter_mut().find(|it| it.id == id) else {
            continue;
        };
        let Some(hi) = heir else {
            it.place = ItemPlace::Ground(pos);
            continue;
        };
        let (h, hname, hpos) = (w.agents[hi].id, w.agents[hi].name.clone(), w.agents[hi].pos);
        it.place = if it.kind == ItemKind::Jewel {
            ItemPlace::Held(h)
        } else {
            ItemPlace::Ground(hpos)
        };
        let what = it
            .story
            .as_ref()
            .and_then(|s| s.name.clone())
            .unwrap_or_else(|| noun(it.kind, it.material));
        if let Some(s) = it.story.as_mut() {
            s.heirloom = true;
            s.owners.push((h, hname, tick));
        }
        w.events.push(
            tick,
            Some(hpos),
            Severity::Notable,
            EventKind::Inherited {
                agent: h,
                from: who,
                what,
            },
        );
    }
}
