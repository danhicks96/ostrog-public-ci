//! Lore (Phase 17c, issue #50 part 3): what people tell each other about the valley's past, as
//! against what happened (deep.rs, the chronicle).
//!
//! A tale is one version of how a hamlet of the old valley ended, held by someone, with who told
//! it to them. Tales come in three ways. Newcomers from a neighbouring place bring what their
//! families remember: a place whose last people went to Mirewell is remembered in Mirewell, and
//! told true. Settlers who stand in a ruin nobody has explained to them wonder, and a guess is
//! as good as a tale until someone knows better: the fearful guess a curse. And tales pass in
//! talk, and drift as they pass: a famine becomes a curse on the crops, a fire becomes lightning
//! sent for some wrong, the last of them going away becomes their being driven out. History
//! keeps what happened; tales keep what is said, with a teller for every version.

use crate::events::{EventKind, Severity};
use crate::ids::AgentId;
use crate::rng::{purpose, stream};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Claim {
    Famine,
    Fever,
    Fire,
    MovedTo(String),
    YoungLeft,
    // What it drifts into.
    Curse,
    Plague,
    Lightning,
    DrivenOut(String),
    // What the old valley noticed about whoever watches it (deep.rs OldAnswer): not how a
    // hamlet ended but what kept happening there. `kind` is a lean toward better, not worse.
    Pattern {
        trigger: crate::deity::Trigger,
        lever: crate::deity::Lever,
        kind: bool,
    },
    // What a pattern drifts into: only that something took notice.
    Watched,
    // A left site with no graves, scorch or worn path to read (#1816): nobody can say where
    // its people went.
    Unknown,
}

impl Claim {
    /// How it ended, in the words someone would use.
    pub fn telling(&self, site: &str) -> String {
        match self {
            Claim::Famine => format!("{site} emptied in a famine"),
            Claim::Fever => format!("fever emptied {site}"),
            Claim::Fire => format!("{site} burned and was not rebuilt"),
            Claim::MovedTo(n) => format!("the last of {site} went to {n}"),
            Claim::YoungLeft => format!("the young left {site} and the old died"),
            Claim::Curse => format!("{site} was cursed and its crops failed"),
            Claim::Plague => format!("a plague was sent on {site}"),
            Claim::Lightning => format!("lightning was sent on {site} for some wrong"),
            Claim::DrivenOut(n) => format!("the people of {n} drove {site} out"),
            Claim::Pattern {
                trigger,
                lever,
                kind,
            } => {
                use crate::deity::{Lever as L, Trigger as T};
                match (trigger, lever, kind) {
                    (T::Plenty, L::Harvest, false) => {
                        format!("whenever a hamlet grew fat, as {site} did, its next harvest failed")
                    }
                    (T::Hardship, L::Harvest, true) => {
                        format!("when {site} was at its lowest a kind year came, as one always did")
                    }
                    (T::Plenty, L::DryStorms, _) => format!(
                        "whenever a hamlet did well, as {site} did, dry storms came and fired its roofs"
                    ),
                    (T::Plenty, L::Sickness, _) => {
                        format!("whenever a hamlet did well, as {site} did, fever followed")
                    }
                    (T::LongQuiet, L::Harvest, true) => {
                        format!("{site} was quiet a long while, then had a year of plenty for no reason")
                    }
                    (T::LongQuiet, L::Harvest, false) => {
                        format!("{site} was quiet too long, and then its harvest failed")
                    }
                    (T::LongQuiet, L::DryStorms, _) => {
                        format!("{site} was quiet too long, and then dry storms came")
                    }
                    (T::LongQuiet, L::Sickness, _) => {
                        format!("{site} was quiet too long, and then fever came")
                    }
                    _ => format!("something kept leaning on {site}"),
                }
            }
            Claim::Watched => format!("something that watches the colony took notice of {site}"),
            Claim::Unknown => format!("{site}'s people left; nobody can say where"),
        }
    }

    /// A tale about what kept happening, rather than how a place ended.
    pub fn is_pattern(&self) -> bool {
        matches!(self, Claim::Pattern { .. } | Claim::Watched)
    }

    /// What a tale becomes when it is told carelessly, or by someone afraid.
    fn drift(&self) -> Claim {
        match self {
            Claim::Famine | Claim::YoungLeft => Claim::Curse,
            Claim::Fever => Claim::Plague,
            Claim::Fire => Claim::Lightning,
            Claim::MovedTo(n) => Claim::DrivenOut(n.clone()),
            Claim::Curse => Claim::Plague,
            Claim::Plague => Claim::Curse,
            Claim::Lightning => Claim::Curse,
            Claim::DrivenOut(n) => Claim::DrivenOut(n.clone()),
            Claim::Pattern { .. } | Claim::Watched => Claim::Watched,
            Claim::Unknown => Claim::Curse,
        }
    }

    /// What really happened, from the history's words.
    pub fn truth(why: &str) -> Claim {
        if let Some(n) = why.strip_prefix("the last of them went to ") {
            return Claim::MovedTo(n.to_string());
        }
        if why.contains("famine") {
            Claim::Famine
        } else if why.contains("fever") {
            Claim::Fever
        } else if why.contains("fire") {
            Claim::Fire
        } else {
            Claim::YoungLeft
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tale {
    /// The old site (deep.rs OldSite id).
    pub site: u32,
    pub claim: Claim,
    /// Who told it to them ("" for their own guess or their family's memory).
    pub teller: Option<AgentId>,
    /// How they came by it, in words: "their family's memory", "their own guess", "told by".
    pub how: String,
    pub heard: u64,
}

impl Tale {
    pub fn true_to(&self, w: &World) -> bool {
        let here = w.history.answers.iter().filter(|a| a.site == self.site);
        match &self.claim {
            Claim::Watched => return here.count() > 0,
            Claim::Pattern {
                trigger,
                lever,
                kind,
            } => {
                let seen = |a: &&crate::deep::OldAnswer| {
                    a.followed
                        && a.trigger == *trigger
                        && a.lever == *lever
                        && (a.strength > 1.0 && a.lever == crate::deity::Lever::Harvest) == *kind
                };
                return here.filter(seen).count() > 0
                    && w.history.answers.iter().filter(seen).count() >= 2;
            }
            _ => {}
        }
        w.history
            .sites
            .iter()
            .find(|s| s.id == self.site)
            .and_then(|s| s.ended.as_ref())
            .map(|(_, why)| Claim::truth(why) == self.claim)
            .unwrap_or(false)
    }
}

fn site_name(w: &World, id: u32) -> String {
    w.history
        .sites
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "a place".into())
}

/// Newcomers bring what their families remember: of a hamlet whose last people went to the
/// place they come from, the true ending; of another, what they heard, perhaps drifted.
pub fn newcomers(w: &mut World, ids: &[AgentId]) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    if w.history.sites.is_empty() {
        return;
    }
    let neighbours: Vec<String> = w
        .overworld
        .sites
        .iter()
        .filter(|s| s.people > 0)
        .map(|s| s.name.clone())
        .collect();
    let sensed = sensed(w);
    for id in ids {
        let mut r = stream(seed, &[purpose::LORE, 0x0E3, id.0 as u64]);
        let Some(from) = neighbours
            .get(r.gen_range(0..neighbours.len().max(1)))
            .cloned()
        else {
            continue;
        };
        // Their people's own: a hamlet whose last people came to their home.
        let kin = w.history.sites.iter().find(|s| {
            s.ended
                .as_ref()
                .is_some_and(|(_, why)| why == &format!("the last of them went to {from}"))
        });
        let kin_site = kin.is_some();
        let (site, claim, how) = match kin {
            Some(s) => (
                s.id,
                Claim::MovedTo(from.clone()),
                format!("their family's memory in {from}"),
            ),
            None => {
                let s = &w.history.sites[r.gen_range(0..w.history.sites.len())];
                let truth = s
                    .ended
                    .as_ref()
                    .map(|(_, why)| Claim::truth(why))
                    .unwrap_or(Claim::YoungLeft);
                let claim = if r.gen_bool(0.5) {
                    truth.drift()
                } else {
                    truth
                };
                (s.id, claim, format!("what is said in {from}"))
            }
        };
        let Some(i) = w.agents.iter().position(|a| a.id == *id) else {
            continue;
        };
        w.agents[i].tales.push(Tale {
            site,
            claim: claim.clone(),
            teller: None,
            how,
            heard: tick,
        });
        let at = w.agents[i].pos;
        let name = site_name(w, site);
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::TaleBrought {
                agent: *id,
                from: from.clone(),
                version: claim.telling(&name),
                kin: kin_site,
            },
        );
        // And, often, what the old people at home say keeps happening in this valley.
        if !sensed.is_empty() && r.gen_bool(0.5) {
            let (site, claim) = sensed[r.gen_range(0..sensed.len())].clone();
            w.agents[i].tales.push(Tale {
                site,
                claim: claim.clone(),
                teller: None,
                how: format!("what the old people in {from} say"),
                heard: tick,
            });
            let name = site_name(w, site);
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::TaleBrought {
                    agent: *id,
                    from: from.clone(),
                    version: claim.telling(&name),
                    kin: false,
                },
            );
        }
    }
}

/// What the valley's people noticed about whoever watches it: each lean that came to pass at
/// least twice, told of the last place it happened.
pub fn sensed(w: &World) -> Vec<(u32, Claim)> {
    let mut seen: std::collections::BTreeMap<(u8, u8, bool), (u32, u32)> = Default::default();
    for a in w.history.answers.iter().filter(|a| a.followed) {
        let kind = a.strength > 1.0 && a.lever == crate::deity::Lever::Harvest;
        let e = seen
            .entry((a.trigger as u8, a.lever as u8, kind))
            .or_insert((0, a.site));
        e.0 += 1;
        e.1 = a.site;
    }
    let mut out = Vec::new();
    for a in w.history.answers.iter().filter(|a| a.followed) {
        let kind = a.strength > 1.0 && a.lever == crate::deity::Lever::Harvest;
        let (n, site) = seen[&(a.trigger as u8, a.lever as u8, kind)];
        let claim = Claim::Pattern {
            trigger: a.trigger,
            lever: a.lever,
            kind,
        };
        if n >= 2 && !out.iter().any(|(_, c)| *c == claim) {
            out.push((site, claim));
        }
    }
    out
}

/// Two people talked: one may tell the other a tale they do not have, and it may change in the
/// telling. Those who hold that strangers bring harm, or fear what they cannot explain, tell it
/// darker.
pub fn on_talk(w: &mut World, i: usize, j: usize) {
    let tick = w.meta.tick;
    for (t, l) in [(i, j), (j, i)] {
        let teller = w.agents[t].id;
        let Some(tale) = w.agents[t]
            .tales
            .iter()
            .find(|x| {
                !w.agents[l]
                    .tales
                    .iter()
                    .any(|y| y.site == x.site && y.claim.is_pattern() == x.claim.is_pattern())
            })
            .cloned()
        else {
            continue;
        };
        let mut r = stream(
            w.meta.seed,
            &[
                purpose::LORE,
                0x7E11,
                teller.0 as u64,
                w.agents[l].id.0 as u64,
                tick,
            ],
        );
        if !r.gen_bool(0.5) {
            continue;
        }
        let tname = w.agents[t].name.clone();
        let (claim, changed) = tell(w, t, l, &tale, format!("told by {tname}"), &mut r);
        let at = w.agents[l].pos;
        let name = site_name(w, tale.site);
        let listener = w.agents[l].id;
        w.events.push(
            tick,
            Some(at),
            if changed {
                Severity::Notable
            } else {
                Severity::Minor
            },
            EventKind::TaleTold {
                teller,
                listener,
                version: claim.telling(&name),
                changed,
            },
        );
    }
}

/// `t` tells `l` a tale and `l` keeps it. It may change in the telling: those who hold that
/// strangers bring harm, or are cautious, tell it darker. Returns the version told and whether
/// it changed.
pub fn tell(
    w: &mut World,
    t: usize,
    l: usize,
    tale: &Tale,
    how: String,
    r: &mut impl Rng,
) -> (Claim, bool) {
    let dark = w.agents[t]
        .beliefs
        .lean(crate::mind::beliefs::Topic::Strangers)
        > 0.2
        || w.agents[t].traits.has(crate::mind::Trait::Cautious);
    let drift_chance = if dark { 0.45 } else { 0.2 };
    let changed = r.gen_bool(drift_chance);
    let claim = if changed {
        tale.claim.drift()
    } else {
        tale.claim.clone()
    };
    let changed = changed && claim != tale.claim;
    let teller = w.agents[t].id;
    let tick = w.meta.tick;
    w.agents[l].tales.push(Tale {
        site: tale.site,
        claim: claim.clone(),
        teller: Some(teller),
        how,
        heard: tick,
    });
    (claim, changed)
}

/// Whether `l` already holds a tale of this kind about this place.
pub fn has_tale(w: &World, l: usize, tale: &Tale) -> bool {
    w.agents[l]
        .tales
        .iter()
        .any(|y| y.site == tale.site && y.claim.is_pattern() == tale.claim.is_pattern())
}

/// The name of an old site, for a telling.
pub fn site_name_of(w: &World, id: u32) -> String {
    site_name(w, id)
}

/// Hourly: settlers standing in a ruin nobody has explained to them wonder how it ended; a band
/// passing the ruins of a hamlet whose people went to their home stops at its graves.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if !tick.is_multiple_of(crate::time::TICKS_PER_HOUR) || w.history.sites.is_empty() {
        return;
    }
    let sites: Vec<(u32, crate::ids::TilePos, String, String)> = w
        .history
        .sites
        .iter()
        .map(|s| {
            (
                s.id,
                s.centre,
                s.name.clone(),
                s.ended.as_ref().map(|e| e.1.clone()).unwrap_or_default(),
            )
        })
        .collect();
    let mut news = Vec::new();
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        // A child does not wonder at ruins and keep a tale of them (#358: a two-day-old did).
        if !a.alive
            || a.community.is_none()
            || a.band.is_some()
            || a.activity.is_asleep()
            || a.is_child(tick)
        {
            continue;
        }
        for (id, c, name, why) in &sites {
            if a.pos.z != c.z
                || a.pos.dist(*c) > 6
                || a.tales
                    .iter()
                    .any(|t| t.site == *id && !t.claim.is_pattern())
            {
                continue;
            }
            // A guess: the fearful see a curse, others read what they can from the place.
            let fearful = a.traits.has(crate::mind::Trait::Cautious)
                || a.beliefs.lean(crate::mind::beliefs::Topic::Death) > 0.3;
            let truth = Claim::truth(why);
            let graves = w
                .history
                .graves
                .iter()
                .filter(|g| g.pos.z == c.z && g.pos.dist(*c) <= 6)
                .count();
            // #1817: a hillside of graves reads as sickness before anything else, rightly
            // or wrongly; burned timbers are read next; otherwise the settler guesses the
            // young simply left.
            let (guess, how) = if fearful {
                (Claim::Curse, "their own guess")
            } else if graves >= 3 {
                (Claim::Fever, "the graves there")
            } else if matches!(truth, Claim::Fire) {
                (Claim::Fire, "the burned timbers")
            } else {
                (Claim::YoungLeft, "their own guess")
            };
            news.push((i, *id, name.clone(), guess, how));
        }
    }
    for (i, site, name, guess, how) in news {
        w.agents[i].tales.push(Tale {
            site,
            claim: guess.clone(),
            teller: None,
            how: how.into(),
            heard: tick,
        });
        let (who, at) = (w.agents[i].id, w.agents[i].pos);
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::RuinWondered {
                agent: who,
                site: name.clone(),
                guess: guess.telling(&name),
            },
        );
    }
    // Bands at the graves of their own people.
    let bands: Vec<(u32, String, Vec<AgentId>, bool)> = w
        .bands
        .iter()
        .map(|b| (b.id, b.site_name.clone(), b.members.clone(), b.remembered))
        .collect();
    for (bid, from, members, done) in bands {
        if done {
            continue;
        }
        let kin = sites
            .iter()
            .find(|(_, _, _, why)| why == &format!("the last of them went to {from}"));
        let Some((_, c, name, _)) = kin else { continue };
        let near = members.iter().any(|m| {
            w.agents
                .iter()
                .any(|a| a.id == *m && a.alive && a.pos.z == c.z && a.pos.dist(*c) <= 10)
        });
        if near {
            if let Some(b) = w.bands.iter_mut().find(|b| b.id == bid) {
                b.remembered = true;
            }
            w.events.push(
                tick,
                Some(*c),
                Severity::Notable,
                EventKind::BandRemembered {
                    band: bid,
                    from: from.clone(),
                    site: name.clone(),
                },
            );
        }
    }
}

/// What someone has heard of the old valley, for inspect: "Whinwick was cursed and its crops
/// failed (told by Kella)".
pub fn heard(w: &World, a: &crate::agents::Agent) -> Vec<String> {
    a.tales
        .iter()
        .map(|t| format!("{} ({})", t.claim.telling(&site_name(w, t.site)), t.how))
        .collect()
}
