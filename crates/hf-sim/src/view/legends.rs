//! The Legends browser (M3, C28): pages about the world's peoples, settlements, figures and
//! realms, written as BBCode with `[url=kind:id]` links between them, and the world map at any
//! year. Everything comes from the realm's state and from chronicle rows that reference the
//! thing (entity_ref); nothing here makes anything up.

use crate::chronicle::Chronicle;
use crate::realm::sites::{Realm, SiteKind};
use crate::realm::RealmMap;
use crate::world_state::World;

/// How a realm year reads: years before the portal (the realm's last year is the one before).
pub fn when(r: &Realm, year: i64) -> String {
    let back = r.year - year + 1;
    match back {
        1 => "the year before the portal".to_string(),
        n if n <= 0 => "after the portal".to_string(),
        n => format!("{n} years before the portal"),
    }
}

fn link(kind: &str, id: u32, text: &str) -> String {
    format!("[url={kind}:{id}]{text}[/url]")
}

fn site_link(r: &Realm, id: u32) -> String {
    r.sites
        .get(id as usize)
        .map(|s| link("site", id, &s.name))
        .unwrap_or_else(|| "a place".into())
}

fn figure_link(r: &Realm, id: u32) -> String {
    r.figures
        .get(id as usize)
        .map(|f| link("figure", id, &f.name))
        .unwrap_or_else(|| "someone".into())
}

/// The kind of a people, as it reads before what they do ("Grisk hunters", "Human farmers").
fn species_link(w: &World, id: crate::species::SpeciesId) -> String {
    link("species", id.0 as u32, &w.kind(id).name)
}

fn people_link(r: &Realm, id: u32) -> String {
    r.peoples
        .get(id as usize)
        .map(|p| link("people", id, &p.name))
        .unwrap_or_else(|| "a people".into())
}

fn polity_link(r: &Realm, id: u32) -> String {
    r.polities
        .get(id as usize)
        .map(|p| link("polity", id, &p.name))
        .unwrap_or_else(|| "a realm".into())
}

/// The chronicle's lines about one thing, oldest first (at most `limit`).
fn record(w: &World, c: &Chronicle, kind: &str, id: u32, limit: usize) -> Vec<String> {
    let Ok(mut st) = c.conn.prepare_cached(
        "SELECT r.id, r.tick, r.detail FROM real_history r JOIN entity_ref e ON e.history_id = r.id \
         WHERE e.entity_kind = ?1 AND e.entity_id = ?2 ORDER BY r.tick, r.id LIMIT ?3",
    ) else {
        return Vec::new();
    };
    let rows = st.query_map(rusqlite::params![kind, id as i64, limit as i64], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    });
    let Ok(rows) = rows else { return Vec::new() };
    let year_of = |tick: u64| -> i64 {
        // Ticks before the arrival map back to realm years (realm/chronicle.rs tick_of).
        let epoch = w.meta.epoch;
        let back = epoch
            .saturating_sub(tick)
            .div_ceil(crate::time::TICKS_PER_YEAR) as i64;
        w.realm.year - back + 1
    };
    rows.filter_map(|r| r.ok())
        .filter_map(|(hid, tick, detail)| {
            let e = crate::view::history::event_from_row(hid, tick as u64, &detail)?;
            Some(format!(
                "[color=#9a9a9a]{}[/color]  {}",
                when(&w.realm, year_of(tick as u64)),
                crate::view::narrate::line(w, &e)
            ))
        })
        .collect()
}

/// One page: "world", "people:N", "site:N", "figure:N" or "polity:N".
pub fn page(w: &World, c: &Chronicle, target: &str) -> String {
    // Why it happened: the valley's own record, whatever the realm holds.
    if let Some(id) = target.strip_prefix("why:") {
        return crate::view::causes::page(w, c, id.parse().unwrap_or(0));
    }
    let r = &w.realm;
    if r.peoples.is_empty() {
        return "[b]Legends[/b]\n\nThis world has no recorded history beyond it.".into();
    }
    let (kind, id) = target.split_once(':').unwrap_or((target, "0"));
    let id: u32 = id.parse().unwrap_or(0);
    match kind {
        "people" => people_page(w, c, id),
        "site" => site_page(w, c, id),
        "figure" => figure_page(w, c, id),
        "polity" => polity_page(w, c, id),
        "war" => war_page(w, c, id),
        "species" => species_page(w, id),
        "culture" => culture_page(w, id),
        "beast" => beast_page(w, c, id),
        // What the valley knows now, not the world's history (#351, WP-08 8.5).
        "known" => crate::view::known_world::page(w),
        _ => world_page(w),
    }
}

/// A beast of the deep (P10): what it is, when it woke, and every settlement it fell on.
fn beast_page(w: &World, c: &Chronicle, id: u32) -> String {
    let r = &w.realm;
    let Some(b) = r.beasts.get(id as usize) else {
        return world_page(w);
    };
    let plan = crate::plan::plan(&b.plan)
        .map(|p| {
            p.parts
                .iter()
                .map(|d| d.id.label())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let mut s = format!(
        "[b]{}[/b]\nA huge thing of {}, of {} parts ({plan}). It woke {} {}.\n",
        b.name,
        b.made_of,
        crate::plan::plan(&b.plan)
            .map(|p| p.parts.len())
            .unwrap_or(0),
        if b.layer == "underworld" {
            "in the underworld"
        } else {
            "in the deep caverns"
        },
        when(r, b.woke)
    );
    if !b.ruined.is_empty() {
        let places: Vec<String> = b.ruined.iter().map(|x| site_link(r, *x)).collect();
        s += &format!("It left {} empty.\n", places.join(", "));
    }
    let lines = record(w, c, "beast", id, 60);
    if !lines.is_empty() {
        s += "\n[b]The record[/b]\n";
        s += &lines.join("\n");
        s += "\n";
    }
    s += "\n[url=world:0]The world[/url]";
    s
}

/// A culture (P7): whose it is, where it grew, how its dead are kept, and what it holds firmly.
fn culture_page(w: &World, id: u32) -> String {
    let r = &w.realm;
    let Some(c) = r.cultures.get(id as usize) else {
        return world_page(w);
    };
    let mut s = format!(
        "[b]{}[/b], a culture of {}.\n",
        c.name,
        people_link(r, c.people)
    );
    match (c.from, c.site) {
        (Some(f), Some(site)) => {
            s += &format!(
                "It grew at {}, far from the homeland, {} (from {}).\n",
                site_link(r, site),
                when(r, c.founded),
                link("culture", f, &r.cultures[f as usize].name)
            )
        }
        _ => s += "The culture of their homeland.\n",
    }
    s += &format!("They {}.\n\n[b]What they hold[/b]\n", c.rite.name());
    let mut firm: Vec<_> = c.stances.iter().filter(|(_, v)| v.abs() >= 0.4).collect();
    firm.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()).then(a.0.cmp(&b.0)));
    if firm.is_empty() {
        s += "Nothing more firmly than their neighbours.\n";
    }
    for (t, v) in firm.iter().take(8) {
        s += &format!("That {}.\n", crate::mind::beliefs::phrase(*t, *v));
    }
    let sites = r
        .sites
        .iter()
        .filter(|x| x.culture == c.id && x.alive())
        .count();
    s += &format!("\nHeld in {sites} settlements now.\n");
    // P8: the gods it named, its grudges, and the stories it tells most lately.
    for g in &c.gods {
        s += &format!("They name {} for {}.\n", g.name, g.domain);
    }
    for g in &c.grudges {
        s += &format!(
            "They hold a grudge against {}.\n",
            people_link(r, g.against)
        );
    }
    // P9: the cultures they have traded and married with most.
    let mut rel: Vec<&crate::realm::relations::Relation> = r
        .relations
        .iter()
        .filter(|x| (x.a == c.id || x.b == c.id) && (x.trade > 0 || x.marriages > 0))
        .collect();
    rel.sort_by_key(|x| (std::cmp::Reverse(x.trade + 20 * x.marriages), x.a, x.b));
    if !rel.is_empty() {
        s += "\n[b]Who they deal with[/b]\n";
        for x in rel.iter().take(5) {
            let other = if x.a == c.id { x.b } else { x.a };
            let married = match x.marriages {
                0 => String::new(),
                1 => ", and one marriage".to_string(),
                n => format!(", and {n} marriages"),
            };
            s += &format!(
                "{}: {} caravans{married}, the last {}.\n",
                link("culture", other, &r.cultures[other as usize].name),
                x.trade,
                when(r, x.last_trade)
            );
        }
    }
    let told: Vec<_> = r
        .stories
        .iter()
        .rev()
        .filter(|x| x.culture == c.id)
        .take(6)
        .collect();
    if !told.is_empty() {
        s += "\n[b]What they tell[/b]\n";
        for st in told {
            let later = if st.version > 0 {
                " (a later telling)"
            } else {
                ""
            };
            s += &format!(
                "{}{later}, {}.\n",
                crate::realm::stories::telling(r, st),
                when(r, st.year)
            );
        }
    }
    s
}

/// A kind of people (P3): its body, senses, life and the world's peoples of that kind.
fn species_page(w: &World, id: u32) -> String {
    let Some(sp) = w
        .kinds()
        .into_iter()
        .find(|(k, _)| k.0 as u32 == id)
        .map(|(_, d)| d)
    else {
        return world_page(w);
    };
    let plan = sp.body_plan();
    let mut s = format!(
        "[b]{}[/b]\n{}.\n\nA body of {} parts ({}), {:.0} to {:.0} cm tall and {:.0} to {:.0} kg; \
         comfortable from {:.0} to {:.0} degrees, {}. They live in {}, build {}, and pair {}.\n",
        sp.plural,
        sp.describe(),
        plan.parts.len(),
        plan.parts
            .iter()
            .map(|d| d.id.label())
            .collect::<Vec<_>>()
            .join(", "),
        sp.size.height_cm[0],
        sp.size.height_cm[1],
        sp.size.mass_kg[0],
        sp.size.mass_kg[1],
        sp.climate.comfort_c[0],
        sp.climate.comfort_c[1],
        match sp.climate.coat.as_str() {
            "bare" => "bare-skinned".to_string(),
            c => format!("in {c}"),
        },
        match sp.social.structure.as_str() {
            "household" => "households".to_string(),
            o => format!("{o}s"),
        },
        sp.habitat.builds,
        match sp.social.bonding.as_str() {
            "pair" => "for life",
            "group" => "in groups",
            _ => "rarely",
        }
    );
    let peoples: Vec<String> = w
        .realm
        .peoples
        .iter()
        .filter(|p| p.species.0 as u32 == id)
        .map(|p| people_link(&w.realm, p.id))
        .collect();
    s += "\n[b]Peoples of this kind[/b]\n";
    if peoples.is_empty() {
        s += "None in this world.\n";
    } else {
        s += &(peoples.join(", ") + "\n");
    }
    s
}

fn world_page(w: &World) -> String {
    let r = &w.realm;
    let alive = r.sites.iter().filter(|s| s.alive()).count();
    let mut s = format!(
        "{}\n\n[b]The world[/b]\n{} years remembered before the portal. {} peoples, {} settlements \
         ({} still lived in), {} named figures, {} realms.\n\n[b]Peoples[/b]\n",
        link("known", 0, "What the colony knows (not the world as it is)"),
        r.year + 1,
        r.peoples.len(),
        r.sites.len(),
        alive,
        r.figures.len(),
        r.polities.len()
    );
    for p in &r.peoples {
        let n = r
            .sites
            .iter()
            .filter(|x| x.people == p.id && x.alive())
            .count();
        s += &format!(
            "{}, {} {}: {n} settlements\n",
            people_link(r, p.id),
            species_link(w, p.species),
            p.living.label()
        );
    }
    s += "\n[b]Realms[/b]\n";
    for p in &r.polities {
        let state = match &p.ended {
            Some((y, cause)) => format!("ended {} ({cause})", when(r, *y)),
            None => format!("since {}", when(r, p.founded)),
        };
        s += &format!(
            "{} of the {}, seat {}: {state}\n",
            polity_link(r, p.id),
            people_link(r, p.people),
            site_link(r, p.seat)
        );
    }
    if !r.wars.is_empty() {
        s += "\n[b]Wars[/b] (the latest)\n";
        for x in r.wars.iter().rev().take(10) {
            let state = match &x.ended {
                Some((y, cause)) => format!("ended {} ({cause})", when(r, *y)),
                None => "still fought".to_string(),
            };
            s += &format!(
                "{}: {state}\n",
                link(
                    "war",
                    x.id,
                    &format!(
                        "{} against {}",
                        r.polities[x.a as usize].name, r.polities[x.b as usize].name
                    )
                )
            );
        }
    }
    if !r.beasts.is_empty() {
        s += "\n[b]Beasts of the deep[/b]\n";
        for b in &r.beasts {
            s += &format!(
                "{}, woke {}\n",
                link("beast", b.id, &b.name),
                when(r, b.woke)
            );
        }
    }
    if let Some(region) = w.meta.setup.as_ref().and_then(|x| x.region) {
        if let Some(p) = r.nearest_people(region) {
            s += &format!(
                "\nThe egress point lies among the {}.\n",
                people_link(r, p.id)
            );
        }
    }
    s
}

fn people_page(w: &World, c: &Chronicle, id: u32) -> String {
    let r = &w.realm;
    let Some(p) = r.peoples.get(id as usize) else {
        return world_page(w);
    };
    let _ = c;
    let mine: Vec<_> = r.sites.iter().filter(|s| s.people == id).collect();
    let alive = mine.iter().filter(|s| s.alive()).count();
    let home = match mine.first() {
        // A people of the deep (P10) lives in the caverns under the land.
        Some(first) if p.deep => format!(
            "They live in the caverns under the land. Their first home was {}.",
            site_link(r, first.id)
        ),
        Some(first) => format!("Their first home was {}.", site_link(r, first.id)),
        None => {
            // A rare people (P5) never had a land of its own.
            let among: Vec<String> = r
                .sites
                .iter()
                .filter(|x| x.minority == Some(id))
                .take(4)
                .map(|x| site_link(r, x.id))
                .collect();
            if among.is_empty() {
                "They have no land of their own.".to_string()
            } else {
                format!(
                    "They have no land of their own; households of them live in {}.",
                    among.join(", ")
                )
            }
        }
    };
    let mut s = format!(
        "[b]The {}[/b], {} {}. {home}\n\n[b]What they hold[/b]\n",
        p.name,
        species_link(w, p.species),
        p.living.label(),
    );
    let mut conv = p.convictions.clone();
    conv.sort_by(|a, b| {
        b.1.abs()
            .partial_cmp(&a.1.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (t, v) in conv.iter().take(4).filter(|c| c.1.abs() >= 0.2) {
        s += &format!("That {}.\n", crate::setup::held_words(*t, *v));
    }
    // Their cultures (P7): the first, and any grown at far settlements.
    let cultures: Vec<String> = r
        .cultures
        .iter()
        .filter(|c| c.people == id)
        .map(|c| link("culture", c.id, &c.name))
        .collect();
    if !cultures.is_empty() {
        s += &format!("\n[b]Cultures[/b] {}\n", cultures.join(", "));
    }
    s += &format!(
        "\n[b]Settlements[/b] {} founded, {alive} still lived in. The largest:\n",
        mine.len()
    );
    let mut big: Vec<_> = mine.iter().filter(|s| s.alive()).collect();
    big.sort_by(|a, b| {
        b.population
            .partial_cmp(&a.population)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for x in big.iter().take(8) {
        s += &format!(
            "{}, a {} of {:.0}\n",
            site_link(r, x.id),
            x.kind.label(),
            x.population
        );
    }
    let realms: Vec<_> = r.polities.iter().filter(|q| q.people == id).collect();
    if !realms.is_empty() {
        s += "\n[b]Their realms[/b]\n";
        for q in realms {
            s += &format!("{}\n", polity_link(r, q.id));
        }
    }
    s += "\n[b]Remembered among them[/b]\n";
    let mut figs: Vec<_> = r.figures.iter().filter(|f| f.people == id).collect();
    figs.sort_by_key(|f| {
        (
            std::cmp::Reverse(
                f.died
                    .as_ref()
                    .map(|d| d.0 - f.born)
                    .unwrap_or(r.year - f.born),
            ),
            f.id,
        )
    });
    for f in figs.iter().take(8) {
        s += &format!("{}, {}\n", figure_link(r, f.id), f.role.label());
    }
    s += "\n[url=world:0]The world[/url]";
    s
}

fn site_page(w: &World, c: &Chronicle, id: u32) -> String {
    let r = &w.realm;
    let Some(x) = r.sites.get(id as usize) else {
        return world_page(w);
    };
    let mut s = format!(
        "[b]{}[/b], a {} of the {}.\nFounded {}",
        x.name,
        if x.kind == SiteKind::Mine {
            "mine"
        } else if x.alive() {
            x.kind.label()
        } else {
            "ruin"
        },
        people_link(r, x.people),
        when(r, x.founded)
    );
    if let Some(p) = x.parent {
        s += &format!(" by settlers from {}", site_link(r, p));
    }
    s += ".\n";
    match &x.ended {
        Some((y, cause)) => {
            s += &format!(
                "Left empty {}: {cause}. At its height {:.0} lived there.\n",
                when(r, *y),
                x.peak
            )
        }
        None => s += &format!("{:.0} live there (at most {:.0}).\n", x.population, x.peak),
    }
    // What it trades with the valley (C36), if the valley knows it: what it holds and the
    // last exchanges.
    if let Some(site) = w
        .overworld
        .sites
        .iter()
        .find(|o| o.realm_site == Some(id) && !o.known_by.is_empty())
    {
        let held: Vec<String> = site
            .stores
            .iter()
            .filter(|(_, v)| **v >= 1.0)
            .map(|(k, v)| format!("{:.0} {}", v, k.label()))
            .collect();
        if !held.is_empty() {
            s += &format!("They hold for trade: {}.\n", held.join(", "));
        }
        for t in site.trades.iter().rev().take(3) {
            let side = |v: &Vec<(crate::items::ItemKind, u32)>| {
                v.iter()
                    .map(|(k, n)| format!("{n} {}", k.label()))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            s += &format!(
                "A caravan from the colony traded {} for {} ({}).\n",
                side(&t.gave),
                if t.got.is_empty() {
                    "nothing".to_string()
                } else {
                    side(&t.got)
                },
                crate::legend::when(w, t.tick)
            );
        }
        // With other settlements beyond the valley (P9).
        for x in site.exchanges.iter().rev().take(3) {
            let with = w
                .overworld
                .sites
                .iter()
                .find(|o| o.id == x.with)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "a neighbour".into());
            s += &format!(
                "Traded {} {} to {with} for {} {} ({}).\n",
                x.gave.1,
                x.gave.0.label(),
                x.got.1,
                x.got.0.label(),
                crate::legend::when(w, x.tick)
            );
        }
    }
    if let Some(q) = r
        .polities
        .iter()
        .find(|q| q.members.contains(&id) || q.seat == id)
    {
        let role = if q.seat == id {
            "the seat of"
        } else {
            "part of"
        };
        s += &format!("It is {role} the realm of {}.\n", polity_link(r, q.id));
    }
    // What each culture tells of what happened here (P8): the same battle told two ways.
    let here: Vec<_> = r.stories.iter().filter(|x| x.site == Some(id)).collect();
    if !here.is_empty() {
        s += "\n[b]What is told of it[/b]\n";
        // Each with its year, and never the same sentence twice (#222).
        let mut told: Vec<String> = Vec::new();
        for st in here.iter().rev() {
            if told.len() >= 6 {
                break;
            }
            let who = r
                .cultures
                .get(st.culture as usize)
                .map(|c| link("culture", c.id, &c.name))
                .unwrap_or_default();
            let later = if st.version > 0 {
                " (a later telling)"
            } else {
                ""
            };
            let line = format!(
                "{who} say: {}{later}, {}.\n",
                crate::realm::stories::telling(r, st),
                when(r, st.year)
            );
            if !told.contains(&line) {
                told.push(line);
            }
        }
        for line in told {
            s += &line;
        }
    }
    let born: Vec<_> = r.figures.iter().filter(|f| f.born_at == id).collect();
    if !born.is_empty() {
        s += &format!("\n[b]Born here[/b] ({})\n", born.len());
        for f in born.iter().take(10) {
            s += &format!("{}, {}\n", figure_link(r, f.id), f.role.label());
        }
    }
    let lines = record(w, c, "site", id, 40);
    if !lines.is_empty() {
        s += "\n[b]The record[/b]\n";
        s += &lines.join("\n");
        s += "\n";
    }
    s += &format!(
        "\n{}  [url=world:0]The world[/url]",
        people_link(r, x.people)
    );
    s
}

fn figure_page(w: &World, c: &Chronicle, id: u32) -> String {
    let r = &w.realm;
    let Some(f) = r.figures.get(id as usize) else {
        return world_page(w);
    };
    let mut s = format!(
        "[b]{}[/b], {} of the {}.\nBorn {} at {}",
        f.name,
        f.role.label(),
        people_link(r, f.people),
        when(r, f.born),
        site_link(r, f.born_at)
    );
    if let Some(p) = f.parent {
        s += &format!(", child of {}", figure_link(r, p));
    }
    s += ".\n";
    match &f.died {
        Some((y, cause)) => s += &format!("Died {} ({cause}), aged {}.\n", when(r, *y), y - f.born),
        None => {
            s += &format!(
                "Lives at {}, aged {}.\n",
                site_link(r, f.site),
                r.year - f.born
            )
        }
    }
    let children: Vec<_> = r.figures.iter().filter(|k| k.parent == Some(id)).collect();
    if !children.is_empty() {
        s += "\n[b]Children[/b]\n";
        for k in children.iter().take(10) {
            s += &format!("{}\n", figure_link(r, k.id));
        }
    }
    let lines = record(w, c, "figure", id, 40);
    if !lines.is_empty() {
        s += "\n[b]The record[/b]\n";
        s += &lines.join("\n");
        s += "\n";
    }
    s += &format!(
        "\n{}  [url=world:0]The world[/url]",
        people_link(r, f.people)
    );
    s
}

fn polity_page(w: &World, c: &Chronicle, id: u32) -> String {
    let r = &w.realm;
    let Some(q) = r.polities.get(id as usize) else {
        return world_page(w);
    };
    let mut s = format!(
        "[b]The realm of {}[/b], of the {}.\nIts seat is {}, since {}.\n",
        q.name,
        people_link(r, q.people),
        site_link(r, q.seat),
        when(r, q.founded)
    );
    match (&q.ended, q.ruler) {
        (Some((y, cause)), _) => s += &format!("It came to an end {}: {cause}.\n", when(r, *y)),
        (None, Some(f)) => s += &format!("Ruled now by {}.\n", figure_link(r, f)),
        (None, None) => s += "Nobody rules it now.\n",
    }
    s += &format!("{} settlements belong to it.\n", q.members.len());
    // P9: common cause made with other realms.
    for x in r.alliances.iter().filter(|x| x.a == id || x.b == id) {
        let other = if x.a == id { x.b } else { x.a };
        let ended = match &x.ended {
            Some((y, why)) => format!(", until {} ({why})", when(r, *y)),
            None => String::new(),
        };
        s += &format!(
            "Allied with {} against {}, {}{ended}.\n",
            polity_link(r, other),
            polity_link(r, x.against),
            when(r, x.since)
        );
    }
    let lines = record(w, c, "polity", id, 60);
    if !lines.is_empty() {
        s += "\n[b]The record[/b]\n";
        s += &lines.join("\n");
        s += "\n";
    }
    s += &format!(
        "\n{}  [url=world:0]The world[/url]",
        people_link(r, q.people)
    );
    s
}

/// The world map as RGB bytes (size x size x 3): country by biome, rivers, and the settlements
/// living in `year` coloured by their people (ruins of that year dark); the valley in white.
pub fn map_rgb(w: &World, m: &RealmMap, year: i64) -> Vec<u8> {
    let r = &w.realm;
    let n = m.size;
    let mut out = crate::realm::map::rgb(m);
    // The deep's settlements (P10) first, darker, so a settlement on the land above wins.
    for x in r
        .sites
        .iter()
        .filter(|x| x.deep)
        .chain(r.sites.iter().filter(|x| !x.deep))
    {
        if x.founded > year {
            continue;
        }
        let gone = x.ended.as_ref().is_some_and(|(y, _)| *y <= year);
        let col = if gone {
            [20, 18, 24]
        } else {
            let c = r
                .peoples
                .get(x.people as usize)
                .map(|p| p.colour)
                .unwrap_or([255, 255, 255]);
            if x.deep {
                [c[0] / 2, c[1] / 2, c[2] / 2]
            } else {
                c
            }
        };
        let k = x.region * 3;
        out[k..k + 3].copy_from_slice(&col);
    }
    if let Some(region) = w.meta.setup.as_ref().and_then(|s| s.region) {
        if region < n * n {
            // Trade routes (C36): a line from the valley to every settlement its caravans
            // have traded with, in pale gold, drawn under the valley's mark. Only on the map
            // of the present: every trade is after the portal.
            for site in w
                .overworld
                .sites
                .iter()
                .filter(|s| !s.trades.is_empty() && year >= r.year)
            {
                let Some(to) = site
                    .realm_site
                    .and_then(|rs| r.sites.get(rs as usize))
                    .map(|x| x.region)
                else {
                    continue;
                };
                let (x0, y0) = ((region % n) as i32, (region / n) as i32);
                let (x1, y1) = ((to % n) as i32, (to / n) as i32);
                let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
                for t in 1..steps {
                    let x = x0 + (x1 - x0) * t / steps;
                    let y = y0 + (y1 - y0) * t / steps;
                    let k = (y as usize * n + x as usize) * 3;
                    out[k..k + 3].copy_from_slice(&[232, 196, 120]);
                }
            }
            out[region * 3..region * 3 + 3].copy_from_slice(&[255, 255, 255]);
        }
    }
    out
}

fn war_page(w: &World, c: &Chronicle, id: u32) -> String {
    let r = &w.realm;
    let Some(x) = r.wars.get(id as usize) else {
        return world_page(w);
    };
    let mut s = format!(
        "[b]The war between {} and {}[/b]\nBegun {}: {}.\n",
        polity_link(r, x.a),
        polity_link(r, x.b),
        when(r, x.began),
        x.cause
    );
    match &x.ended {
        Some((y, cause)) => {
            s += &format!(
                "Ended {}: {cause}. {} battles were fought.\n",
                when(r, *y),
                x.battles
            )
        }
        None => s += &format!("Still fought; {} battles so far.\n", x.battles),
    }
    let lines = record(w, c, "war", id, 60);
    if !lines.is_empty() {
        s += "\n[b]The record[/b]\n";
        s += &lines.join("\n");
        s += "\n";
    }
    s += "\n[url=world:0]The world[/url]";
    s
}
