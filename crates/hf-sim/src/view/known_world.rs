//! What the valley knows of the world beyond it (#351, WP-08 slice 8.5): "every panel is a view
//! over knowledge, never over truth". The Legends world page is the realm's history; this page is
//! only what the people of the valley have seen, heard and been told, and it says what they do not
//! know. Read-only. Places come from `known_by`, strangers only once sighted, the road from the
//! caravans and journeys under way, and the lost only as a count nobody can put names to.

use crate::expedition::{CaravanState, Edge, Purpose, SiteKind};
use crate::world_state::World;

fn kind(k: SiteKind) -> &'static str {
    match k {
        SiteKind::Village => "a village",
        SiteKind::Ruin => "a ruin",
        SiteKind::Mine => "a mine",
        SiteKind::Forest => "a forest",
        SiteKind::Lakeshore => "a lakeshore",
    }
}

fn edge(e: Edge) -> &'static str {
    match e {
        Edge::North => "north",
        Edge::East => "east",
        Edge::South => "south",
        Edge::West => "west",
    }
}

/// A band's number as the watchers would tell it, not counted to the head (review of #739).
fn how_many(n: u32) -> &'static str {
    match n {
        0..=2 => "a couple",
        3..=5 => "a few",
        6..=14 => "a dozen or so",
        _ => "many",
    }
}

/// The page, as the Legends browser's BBCode.
pub fn page(w: &World) -> String {
    let ow = &w.overworld;
    let mut s = String::from(
        "[b]What the colony knows[/b]\nNot the world as it is: what the people here \
         have seen, heard and been told.\n\n[b]Places beyond the colony[/b]\n",
    );
    let known: Vec<_> = ow.sites.iter().filter(|x| !x.known_by.is_empty()).collect();
    if known.is_empty() {
        s +=
            "None. Nobody here has been past the edge of the landing site or met anyone who has.\n";
    }
    for site in &known {
        let who: Vec<String> = site
            .known_by
            .iter()
            .filter_map(|c| w.communities.iter().find(|x| x.id == *c))
            .map(|c| c.name.clone())
            .collect();
        s += &format!(
            "{}, {}, {} days out to the {}; known in {}\n",
            site.name,
            kind(site.kind),
            site.days,
            edge(site.edge),
            who.join(" and ")
        );
    }
    s += "Of anywhere else, nothing is known.\n\n[b]Strangers[/b]\n";
    let seen: Vec<_> = w.bands.iter().filter(|b| b.sighted).collect();
    if seen.is_empty() {
        s += "No strangers are about that anyone has seen.\n";
    }
    for b in seen {
        let state = match b.state {
            crate::threat::BandState::Approaching => "coming",
            crate::threat::BandState::Camped(_) => "camped near",
            crate::threat::BandState::Leaving => "leaving",
        };
        // Named by their home only when the valley knows that place (review of #739).
        let known_home = ow
            .sites
            .iter()
            .any(|x| x.id == b.site && !x.known_by.is_empty());
        let who = if known_home {
            format!("A band from {}", b.site_name)
        } else {
            "A band of strangers".to_string()
        };
        s += &format!("{who}, {}, {state}\n", how_many(b.size));
    }
    s += "\n[b]On the road[/b]\n";
    let mut road = 0;
    // A caravan the sim has marked lost is still on the road to the valley, which has no word of
    // it yet (review of #739); only one come home (Done) is not.
    for c in &ow.caravans {
        if matches!(c.state, CaravanState::Done) {
            continue;
        }
        let site = match c.purpose {
            Purpose::Trade { site, .. } | Purpose::Scout { site } | Purpose::Fetch { site, .. } => {
                site
            }
        };
        let to = ow
            .sites
            .iter()
            .find(|x| x.id == site)
            .map(|x| x.name.clone())
            .unwrap_or_else(|| "somewhere not named".into());
        let what = match c.purpose {
            Purpose::Trade { .. } => "to trade",
            Purpose::Scout { .. } => "to look",
            Purpose::Fetch { .. } => "to fetch what is wanted",
        };
        s += &format!("A caravan of {} gone to {to} {what}\n", c.members.len());
        road += 1;
    }
    // Nor does the valley know of a death on the road: they are still away (review of #739).
    for a in ow.away.iter().filter(|a| a.community.is_some()) {
        if let Some((_, to, why)) = &a.journey {
            s += &format!("{} gone to {to}, {why}\n", a.name);
            road += 1;
        }
    }
    if road == 0 {
        s += "Nobody is away.\n";
    }
    let lost = crate::view::known::unfound(w).len();
    if lost > 0 {
        s += &format!(
            "\n[b]The lost[/b]\n{lost} of those who came through have not been found. Whether \
             they live, nobody here knows.\n"
        );
    }
    s
}
