//! Event text. Every sentence is filled only from the recorded event's fields and the
//! names of the entities it references. No embellishment.

use crate::events::{BreakExplanation, Event, EventKind};
use crate::world_state::World;

fn name(w: &World, id: crate::ids::AgentId) -> String {
    w.agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|a| a.id == id)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| format!("#{}", id.0))
}

/// Whoever an event names, as the line says it at `tick`: a person by name; an animal as
/// `beast` says it, by an earned name only if it had it then. A beast named in a fight was
/// renamed in the lines before it, so a re-told line differed from the one shown (#486,
/// FIXLOG 2026-11-03; the c82 rule of `beast` carried to every line).
fn named_as(w: &World, tick: u64, id: crate::ids::AgentId) -> String {
    let animal = w
        .agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|a| a.id == id)
        .is_some_and(|a| a.species != crate::agents::Species::Person);
    if animal {
        beast(w, tick, id)
    } else {
        name(w, id)
    }
}

/// "a, b and c".
fn join(xs: &[String]) -> String {
    match xs.len() {
        0 => String::new(),
        1 => xs[0].clone(),
        n => format!("{} and {}", xs[..n - 1].join(", "), xs[n - 1]),
    }
}

/// An animal as a sentence's subject: its name if people gave it one, else "A wolf".
/// A beast as the line names it: by its earned name only if it had it at `tick`.
fn beast(w: &World, tick: u64, id: crate::ids::AgentId) -> String {
    let a = w
        .agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|a| a.id == id);
    let n = match a {
        Some(a) if a.named_at.is_some_and(|t| tick < t) => {
            format!("{:?}", a.species).to_lowercase()
        }
        _ => name(w, id),
    };
    match n.chars().next() {
        Some(c) if c.is_lowercase() => {
            let article = if "aeiou".contains(c) { "An" } else { "A" };
            format!("{article} {n}")
        }
        _ => n,
    }
}

fn cname(w: &World, id: crate::ids::CommunityId) -> String {
    w.communities
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| format!("community {}", id.0))
}

pub fn line(w: &World, e: &Event) -> String {
    use EventKind::*;
    match &e.kind {
        WorldCreated { seed } => format!("{} was founded (seed {}).", w.meta.name, seed),
        BeatOut { agent, .. } => format!("{} beat out the flames.", named_as(w, e.tick, *agent)),
        SiteHungry { site } => format!("The stores ran out at {site}."),
        BandSetOut { site, size, .. } => format!("{size} hungry people set out from {site}."),
        StrangersSighted {
            agent, band, size, ..
        } => {
            // Strangers of another kind are named for it (P3).
            let kind = w
                .agents
                .iter()
                .find(|a| a.band == Some(*band))
                .map(|a| a.people)
                .filter(|p| *p != crate::species::SpeciesId::HUMAN)
                .map(|p| w.kind(p).plural.clone())
                .unwrap_or_else(|| "strangers".into());
            format!(
                "{} saw {kind} camped at the edge of the colony, {size} of them.",
                named_as(w, e.tick, *agent)
            )
        }
        Raided {
            what,
            count,
            open: false,
            ..
        } => format!("Strangers took {count} {what} from the stores in the night."),
        Raided { what, count, .. } => {
            format!("The strangers took {count} {what} from the stores, as they had been told they could.")
        }
        Welcomed { agent, .. } => format!(
            "{} went out to the strangers and told them they could eat.",
            named_as(w, e.tick, *agent)
        ),
        BandLeft {
            site,
            took,
            lost,
            peaceful,
            ..
        } => {
            if *peaceful {
                format!(
                    "The strangers went home to {site} with {took} portions of food, and no harm done."
                )
            } else if *lost > 0 {
                format!("The strangers went back toward {site}, {lost} fewer than came, with {took} portions of food.")
            } else {
                format!("The strangers went back toward {site} with {took} portions of food.")
            }
        }
        Attacked {
            animal,
            agent,
            driven_off,
        } => {
            if *driven_off {
                format!(
                    "{} went for {}, and was driven off.",
                    beast(w, e.tick, *animal),
                    named_as(w, e.tick, *agent)
                )
            } else {
                format!("{} went for {}.", beast(w, e.tick, *animal), named_as(w, e.tick, *agent))
            }
        }
        DeityAnswered {
            deity,
            trigger,
            lever,
            place,
            days,
            ..
        } => format!(
            "{deity} answered {trigger} by leaning on {lever} over {place} for {days} days."
        ),
        ArcTurn {
            now,
            during,
            because,
        } => {
            let mut c = now.chars();
            let head = c
                .next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default();
            let mut s = format!("{head} now, on top of {}", join(during));
            if let Some(b) = because {
                s.push_str(", because ");
                s.push_str(b);
            }
            s.push('.');
            s
        }
        ArcEased { days, what, lost } => {
            let d = if *days == 1 {
                "a day".to_string()
            } else {
                format!("{days} days")
            };
            let mut s = format!("After {d} of {}, things eased.", join(what));
            match lost {
                0 => {}
                1 => s.push_str(" One did not live to see it."),
                n => s.push_str(&format!(" {n} did not live to see it.")),
            }
            s
        }
        Studied {
            agent,
            what,
            site,
            learned,
            success,
        } => {
            if *success {
                format!(
                    "{} studied the {what} from {site} and worked out {learned}.",
                    named_as(w, e.tick, *agent)
                )
            } else {
                format!(
                    "{} puzzled over the {what} from {site} and could make nothing of it.",
                    named_as(w, e.tick, *agent)
                )
            }
        }
        CraftTried {
            agent,
            craft,
            success,
            spoiled,
        } => {
            if *success {
                format!("{} worked out {craft} by trying.", named_as(w, e.tick, *agent))
            } else {
                format!(
                    "{} tried {craft} without knowing how and spoiled {spoiled}.",
                    named_as(w, e.tick, *agent)
                )
            }
        }
        CraftLost {
            community,
            craft,
            last,
            how,
        } => format!(
            "With {last} {how}, nobody in {} knows {craft} any more.",
            cname(w, *community)
        ),
        Walled {
            community, side, ..
        } => format!(
            "{} have raised a wall round their houses, with a gate to the {side}.",
            cname(w, *community)
        ),
        CraftRegained {
            community,
            craft,
            by,
        } => format!("{} knows {craft} again: {by} brought it.", cname(w, *community)),
        Taught { teacher, pupil, craft } => format!(
            "{} taught {} {craft} at the bench.",
            named_as(w, e.tick, *teacher),
            named_as(w, e.tick, *pupil)
        ),
        LearnedByWatching {
            agent,
            from,
            technique,
        } => format!(
            "{} picked up {technique}, watching {}.",
            named_as(w, e.tick, *agent),
            named_as(w, e.tick, *from)
        ),
        CrowsGathered { .. } => "Crows came down where something died.".to_string(),
        HartSighted { agent } => format!(
            "{} saw the White Hart at the edge of the trees.",
            named_as(w, e.tick, *agent)
        ),
        BearDenned { bear } => format!("{} has gone to its den for the winter.", beast(w, e.tick, *bear)),
        BearWoke { bear } => format!("{} has come out of its den.", beast(w, e.tick, *bear)),
        DucksLeft { .. } => "The ducks flew south.".to_string(),
        DucksReturned { .. } => "The ducks came back to the water.".to_string(),
        CameOver { agent } => format!(
            "{} came over the hills into the area, where there was food for it again.",
            beast(w, e.tick, *agent)
        ),
        BeastSlain { agent, beast: b, hurt } => match hurt {
            0 => format!("{} brought down {b}.", named_as(w, e.tick, *agent)),
            1 => format!("{} brought down {b}, who had hurt someone.", named_as(w, e.tick, *agent)),
            n => format!("{} brought down {b} at last; it had hurt {n} people.", named_as(w, e.tick, *agent)),
        },
        TaleBrought {
            agent,
            from,
            version,
            kin,
        } => {
            if *kin {
                format!(
                    "{} came from {from}, where their family remembers that {version}.",
                    named_as(w, e.tick, *agent)
                )
            } else {
                format!(
                    "{} came from {from} with a tale: {version}.",
                    named_as(w, e.tick, *agent)
                )
            }
        }
        TaleTold {
            teller,
            listener,
            version,
            changed,
        } => {
            if *changed {
                format!(
                    "In {}'s telling to {}, {version}.",
                    named_as(w, e.tick, *teller),
                    named_as(w, e.tick, *listener)
                )
            } else {
                format!("{} told {} that {version}.", named_as(w, e.tick, *teller), named_as(w, e.tick, *listener))
            }
        }
        RuinWondered { agent, site, guess } => format!(
            "{} stood in the ruins of {site} and wondered; perhaps {guess}.",
            named_as(w, e.tick, *agent)
        ),
        BandRemembered { from, site, .. } => format!(
            "The strangers from {from} stopped at the graves of {site}; their people came from there."
        ),
        OldFounded { site, by, people } if people.is_empty() => format!("{by} founded {site}."),
        OldFounded { site, by, people } => format!("{by}, of the {people}, founded {site}."),
        OldTrouble { what, .. } => what.clone(),
        OldAnswered {
            site,
            deity,
            trigger,
            lever,
        } => format!("{deity} answered {trigger} at {site} by leaning on {lever}."),
        SetupChosen {
            deity,
            history_years,
            valley,
            held,
            peoples,
        } => {
            let held = if held.is_empty() {
                "nothing in common beyond what each had lived".to_string()
            } else {
                format!("in common that {held}")
            };
            let past = match history_years {
                0 => "no past to find".to_string(),
                n => format!("{n} years of local history behind it"),
            };
            let world = if peoples.is_empty() {
                String::new()
            } else {
                format!(" In this world: {peoples}.")
            };
            format!(
                "The portal opened on {valley}, watched by {deity}, with {past}. The three \
                 held {held}.{world}"
            )
        }
        OldDied { who, site, cause } => match cause.as_str() {
            "old age" => format!("{who} of {site} died old."),
            "the raid" => format!("{who} of {site} was killed in the raid."),
            "the fire" => format!("{who} of {site} died in the fire."),
            c => format!("{who} of {site} died of {c}."),
        },
        OldEnded { site, why } => format!("{site} stood empty: {why}."),
        RoofFell {
            tiles,
            hurt,
            because,
            ..
        } => {
            let what = if *tiles > 3 {
                "The roof"
            } else {
                "Part of a roof"
            };
            let mut s = format!("{what} came down {because}.");
            match hurt.len() {
                0 => s.push_str(" Nobody was under it."),
                _ => {
                    let names: Vec<String> = hurt.iter().map(|h| named_as(w, e.tick, *h)).collect();
                    s.push_str(&format!(" It caught {}.", join(&names)));
                }
            }
            s
        }
        UpperFloorFell {
            tiles,
            hurt,
            because,
            ..
        } => {
            let what = if *tiles > 3 {
                "The upper room"
            } else {
                "Part of the upper room"
            };
            let mut s = format!("{what} came down {because}.");
            match hurt.len() {
                0 => s.push_str(" Nobody was under it."),
                _ => {
                    let names: Vec<String> = hurt.iter().map(|h| named_as(w, e.tick, *h)).collect();
                    s.push_str(&format!(" It caught {}.", join(&names)));
                }
            }
            s
        }
        SaplingTrampled { .. } => "A young tree was trampled underfoot.".to_string(),
        HouseStage { stage, by, .. } => match (stage.as_str(), by) {
            ("marked out", Some(b)) => format!("{} marked out a new house.", named_as(w, e.tick, *b)),
            ("marked out", None) => "A new house was marked out.".to_string(),
            ("walls up", _) => {
                "The walls of the new house are up; it is open to the sky.".to_string()
            }
            ("roofed", _) => "The new house has its roof.".to_string(),
            ("mended", _) => "The roof is mended.".to_string(),
            (s, _) => format!("The house: {s}."),
        },
        Named {
            name: n, because, ..
        } => format!("They call it {n} now, {because}."),
        Lived {
            name, age, what, ..
        } => {
            let years = w.meta.epoch.saturating_sub(e.tick) / crate::time::TICKS_PER_YEAR;
            let when = match years {
                0 => "At the portal".to_string(),
                1 => "A year before the arrival".to_string(),
                n => format!("{n} years before the arrival"),
            };
            format!("{when}: {}", crate::past::narrate(name, *age, what))
        }
        Arrived { agent, .. } => format!("{} arrived.", named_as(w, e.tick, *agent)),
        Born {
            agent,
            mother,
            father,
        } => {
            // A child of two kinds (P9): the record says which.
            let kind = |id: &crate::ids::AgentId| {
                w.agents
                    .iter()
                    .find(|a| a.id == *id)
                    .map(|a| (a.people, a.species_def().name.clone()))
            };
            match (kind(mother), father.as_ref().and_then(kind)) {
                (Some((pm, km)), Some((pf, kf))) if pm != pf => format!(
                    "{} was born to {}, of a {km} mother and a {kf} father.",
                    named_as(w, e.tick, *agent),
                    named_as(w, e.tick, *mother)
                ),
                _ => format!("{} was born to {}.", named_as(w, e.tick, *agent), named_as(w, e.tick, *mother)),
            }
        }
        // (An animal: "A deer died: brought down by wolves."; a person by name.)
        Died { agent, cause } => format!("{} died: {}.", beast(w, e.tick, *agent), cause),
        Ate { agent, quality, .. } => {
            let how = if *quality > 0.75 {
                "a good meal"
            } else if *quality > 0.4 {
                "a meal"
            } else {
                "a poor meal"
            };
            format!("{} ate {}.", named_as(w, e.tick, *agent), how)
        }
        Slept { agent, hours, .. } => format!("{} slept {:.0} hours.", named_as(w, e.tick, *agent), hours),
        Foraged { agent, .. } => format!("{} gathered food.", named_as(w, e.tick, *agent)),
        Harvested { agent, .. } => format!("{} harvested a crop.", named_as(w, e.tick, *agent)),
        Hauled { agent, .. } => format!("{} carried goods to the stockpile.", named_as(w, e.tick, *agent)),
        Built { agent, at, .. } => match w.structures.iter().find(|s| s.pos == *at) {
            Some(s) => {
                let what = match s.kind {
                    crate::structures::StructureKind::CookingPot => "cooking pot".to_string(),
                    crate::structures::StructureKind::CarpenterBench => {
                        "carpenter's bench".to_string()
                    }
                    k => format!("{k:?}").to_lowercase(),
                };
                format!("{} finished the {what}.", named_as(w, e.tick, *agent))
            }
            None => format!("{} finished building.", named_as(w, e.tick, *agent)),
        },
        Mined { agent, found, .. } => match found {
            Some(f) => format!("{} struck {} while mining.", named_as(w, e.tick, *agent), f),
            None => format!("{} mined rock.", named_as(w, e.tick, *agent)),
        },
        Cooked { agent, .. } => format!("{} cooked a meal.", named_as(w, e.tick, *agent)),
        Talked { a, b, warmth } => {
            let how = if *warmth > 0.7 { "warmly" } else { "a while" };
            format!("{} and {} talked {}.", named_as(w, e.tick, *a), named_as(w, e.tick, *b), how)
        }
        Quarrelled { a, b } => format!("{} and {} quarrelled.", named_as(w, e.tick, *a), named_as(w, e.tick, *b)),
        PlanFailed {
            agent,
            goal,
            reason,
        } => format!("{} gave up {}: {}.", named_as(w, e.tick, *agent), goal, reason),
        Discovered { agent, what, .. } => format!("{} discovered {}.", named_as(w, e.tick, *agent), what),
        Starving { agent } => format!("{} is starving.", named_as(w, e.tick, *agent)),
        Parched { agent } => format!("{} has had nothing to drink and is parched.", named_as(w, e.tick, *agent)),
        TrueDream { agent, what, .. } => format!(
            "{} cried out in their sleep, of {}.",
            named_as(w, e.tick, *agent),
            what
        ),
        Drenched { what, .. } => format!("The flood water spoiled {what} where it stood."),
        VerminDrawn { .. } => "Vermin came to the rotted food.".to_string(),
        VerminBite { agent, .. } => format!("{} was bitten in the night by vermin.", named_as(w, e.tick, *agent)),
        Festered { agent, .. } => format!("{}'s bite festered.", named_as(w, e.tick, *agent)),
        Exhausted { agent } => format!("{} collapsed from exhaustion.", named_as(w, e.tick, *agent)),
        StoresFailed { community, .. } => format!(
            "{}'s stores ran short with the cold months on them.",
            cname(w, *community)
        ),
        Weakened { agent, .. } => format!("{} grew weak from hunger.", named_as(w, e.tick, *agent)),
        SickenedWeak { agent, what, .. } => format!(
            "{}, weak from hunger, took {what}.",
            named_as(w, e.tick, *agent)
        ),
        RainSpoiled { what, .. } => format!(
            "The rain came in where the roof had gone and spoiled {what}."
        ),
        HealerLost {
            community, healer, ..
        } => format!(
            "With {} dead, {} had no healer left.",
            named_as(w, e.tick, *healer),
            cname(w, *community)
        ),
        WoundFestered { agent, .. } => format!(
            "{}'s wound festered, with no healer left to dress it.",
            named_as(w, e.tick, *agent)
        ),
        Weather { kind } => match kind.as_str() {
            "rain" => "It began to rain.".to_string(),
            "snow" => "It began to snow.".to_string(),
            other => format!("{}{}.", other[..1].to_uppercase(), &other[1..]),
        },
        SeasonChanged { season, year } => format!("{} began, year {}.", season, year + 1),
        Dreamt {
            agent,
            consolidated,
        } => format!(
            "{} dreamt ({} memories settled).",
            named_as(w, e.tick, *agent),
            consolidated
        ),
        Joined { agent, community } => {
            format!("{} joined {}.", named_as(w, e.tick, *agent), cname(w, *community))
        }
        Refugees {
            agents,
            from,
            cause,
        } => {
            let who = join(&agents.iter().map(|a| named_as(w, e.tick, *a)).collect::<Vec<_>>());
            format!("{who} came, fleeing {from}: {cause}.")
        }
        WouldNotStay {
            agent,
            community,
            because,
        } => format!(
            "{} would not stay at {}: {because}.",
            named_as(w, e.tick, *agent),
            cname(w, *community)
        ),
        TurnedAway {
            agent,
            community,
            reason,
        } => format!(
            "{} turned {} away: {}.",
            cname(w, *community),
            named_as(w, e.tick, *agent),
            reason
        ),
        Answered {
            leader,
            went,
            stayed,
            reasons,
        } => {
            let who = named_as(w, e.tick, *leader);
            let tail = if reasons.is_empty() {
                String::new()
            } else {
                format!(": {}", reasons.join("; "))
            };
            if *went == 0 {
                format!("Nobody went with {who}; {stayed} stayed{tail}.")
            } else {
                format!("{went} went with {who}, {stayed} stayed{tail}.")
            }
        }
        Split {
            from,
            to,
            leader,
            members,
            cause,
        } => format!(
            "{} led {} people away from {} to found {}: {}.",
            named_as(w, e.tick, *leader),
            members,
            cname(w, *from),
            cname(w, *to),
            cause
        ),
        MovedCamp {
            community,
            leader,
            cause,
            ..
        } => format!(
            "{} moved camp, led by {}: {}.",
            cname(w, *community),
            named_as(w, e.tick, *leader),
            cause
        ),
        LeftTheValley {
            community,
            leader,
            people,
            to,
            cause,
        } => format!(
            "{} left the area, {} of them led by {}, to settle at {}: {}.",
            cname(w, *community),
            people,
            named_as(w, e.tick, *leader),
            to,
            cause
        ),
        CommunityEnded { name: n, how, .. } => format!("{n} is no more in the colony: they {how}."),
        Traded {
            from,
            to,
            carrier,
            gave,
            received,
        } => {
            if received == "nothing" {
                // A gift is called one (#260): not a hundred failed bargains.
                format!(
                    "{} brought {} from {} to {}.",
                    named_as(w, e.tick, *carrier),
                    gave,
                    cname(w, *from),
                    cname(w, *to)
                )
            } else {
                format!(
                    "{} carried {} from {} to {} and came back with {}.",
                    named_as(w, e.tick, *carrier),
                    gave,
                    cname(w, *from),
                    cname(w, *to),
                    received
                )
            }
        }
        Chose {
            agent,
            what,
            because,
        } => format!("{} {}, {}.", named_as(w, e.tick, *agent), what, because),
        RoleTaken { agent, role } => format!("{} took up {} work.", named_as(w, e.tick, *agent), role),
        Left { agent } => format!("{} left the area.", named_as(w, e.tick, *agent)),
        Paired { a, b } => format!("{} and {} became partners.", named_as(w, e.tick, *a), named_as(w, e.tick, *b)),
        CustomFormed {
            community,
            label,
            narrative,
            ..
        } => format!(
            "{} now hold that we {}: \"{}\"",
            cname(w, *community),
            label,
            narrative
        ),
        CustomAdopted {
            agent,
            community,
            narrative,
            ..
        } => format!(
            "{} learned the way of {}: \"{}\"",
            named_as(w, e.tick, *agent),
            cname(w, *community),
            narrative
        ),
        HearthTale {
            teller,
            listeners,
            what,
        } => {
            // Three names and a count (#228: a line of forty names ran to six rows).
            let mut names: Vec<String> = listeners.iter().map(|l| named_as(w, e.tick, *l)).collect();
            let who = if names.len() > 4 {
                let rest = names.len() - 3;
                names.truncate(3);
                format!("{} and {rest} others", names.join(", "))
            } else {
                join(&names)
            };
            format!(
                "By the hearth, {} told {}. {who} listened.",
                named_as(w, e.tick, *teller),
                what
            )
        }
        IdentityFormed { community, line } => {
            format!("{} have become a people who {}.", cname(w, *community), line)
        }
        IdentityLapsed { community, line } => format!(
            "{} are no longer a people who {}.",
            cname(w, *community),
            line
        ),
        CustomLapsed {
            community, label, ..
        } => format!(
            "{} no longer keep the custom to {}.",
            cname(w, *community),
            label
        ),
        RestDay {
            community,
            remembered,
            ..
        } => format!(
            "{} rest today, remembering {}.",
            cname(w, *community),
            named_as(w, e.tick, *remembered)
        ),
        Hunted { agent, prey } => format!("{} brought down a {}.", named_as(w, e.tick, *agent), prey),
        Collapse {
            by,
            tiles,
            rock,
            sank,
            hurt,
            ..
        } => {
            let mut s = format!(
                "The {rock} roof came down where {} was cutting: {} {} of gallery filled with rubble.",
                named_as(w, e.tick, *by),
                tiles,
                if *tiles == 1 { "tile" } else { "tiles" }
            );
            if !hurt.is_empty() {
                let names: Vec<String> = hurt.iter().map(|h| named_as(w, e.tick, *h)).collect();
                s.push_str(&format!(" It caught {}.", join(&names)));
            }
            if *sank {
                s.push_str(" The ground above sank into it.");
            }
            s
        }
        ThinStirred { .. } => {
            "The air about the stone where they came through has gone thin.".to_string()
        }
        SharedDream { dreamers, of } => {
            let names: Vec<String> = dreamers.iter().map(|d| named_as(w, e.tick, *d)).collect();
            format!("{} dreamt the same dream in the night: {of}.", join(&names))
        }
        ThingFromHome { item, maker, from } => {
            let what = w
                .items
                .iter()
                .find(|it| it.id == *item)
                .map(|it| upper_first(&a_thing(it.kind, it.material)))
                .unwrap_or_else(|| "Something".into());
            format!("{what} lay by the portal stone, made by {maker} in {from}.")
        }
        Recognised { agent, maker, .. } => format!(
            "{} knew it at once: {maker} made it, at home.",
            named_as(w, e.tick, *agent)
        ),
        EchoSeen {
            agent,
            of,
            cause,
            known,
        } => {
            if *known {
                format!(
                    "At dusk {} saw the echo of {of} by the portal stone.",
                    named_as(w, e.tick, *agent)
                )
            } else {
                format!(
                    "At dusk {} saw {of} standing by the portal stone, and knew then that {of} had died alone, {}.",
                    named_as(w, e.tick, *agent),
                    crate::opening::died_how(cause)
                )
            }
        }
        WrongWeather { cold, .. } => {
            if *cold {
                "Snow fell on a patch of ground by the portal stone and nowhere else.".to_string()
            } else {
                "The frost went from a patch of ground by the portal stone, and only there."
                    .to_string()
            }
        }
        ThinStudied { agent, learned } => {
            if *learned {
                format!("{} sat by the portal stone and began to understand it.", named_as(w, e.tick, *agent))
            } else {
                format!("{} sat by the portal stone a long while.", named_as(w, e.tick, *agent))
            }
        }
        ThinQuieted { .. } => {
            "The air about the portal stone is still again; the warding stones hold it.".to_string()
        }
        Realm { what } => crate::realm::chronicle::line(&w.realm, what),
        Rotted { what } => format!("Food went off where it lay: {what}."),
        NewsHeard { teller, from, what } => {
            format!("{}, back from {from}, brings news: {what}", named_as(w, e.tick, *teller))
        }
        Restless { agent } => format!(
            "{} lies unremembered: no rite was given, nothing carved, and nobody remembers.",
            named_as(w, e.tick, *agent)
        ),
        GhostSeen { of, by, cause } => format!(
            "{} saw {} at dusk, where they died ({cause}).",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *of)
        ),
        AtRest { agent, how } => format!("{} is at rest: {how}.", named_as(w, e.tick, *agent)),
        Cursed {
            agent,
            condition,
            because,
        } => format!("{} was taken by {condition}, {because}.", named_as(w, e.tick, *agent)),
        Bitten { by, of } => format!("{} was bitten in the night by {}.", named_as(w, e.tick, *of), named_as(w, e.tick, *by)),
        Missing { agent } => format!(
            "{} was not where they sleep on the night of the full moon.",
            named_as(w, e.tick, *agent)
        ),
        AteDead { agent, of } => format!(
            "{}, starving, ate of the dead {}.",
            named_as(w, e.tick, *agent),
            named_as(w, e.tick, *of)
        ),
        TurnedToStone { agent, cavern } => format!(
            "{} died standing, turned to stone, since they touched the underworld's rock in {cavern}.",
            named_as(w, e.tick, *agent)
        ),
        Unbound {
            agent,
            by,
            condition,
        } => format!(
            "{} lifted {condition} from {}.",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *agent)
        ),
        Suspected { by, of, because } => format!(
            "{} has come to suspect {}: {because}.",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *of)
        ),
        WidelySuspected { of, because, count } => format!(
            "{} is whispered about: {count} now say {because}.",
            named_as(w, e.tick, *of)
        ),
        Breached { agent, cavern } => format!(
            "{} broke through into {cavern}, a cavern of the deep.",
            named_as(w, e.tick, *agent)
        ),
        FromBelow { agents, from } => {
            let who = join(&agents.iter().map(|a| named_as(w, e.tick, *a)).collect::<Vec<_>>());
            format!("{who} came up from below, from {from}.")
        }
        BeastStirred { beast, cavern } => {
            format!("Something lairs in the deep beyond {cavern}: {beast}, by the world's record.")
        }
        BatsStirred {
            agent,
            cavern,
            bats,
        } => format!(
            "{} came into {cavern} and {} went up off the rock.",
            named_as(w, e.tick, *agent),
            if *bats > 6 {
                "a cloud of bats".to_string()
            } else {
                format!("{bats} bats")
            }
        ),
        BurrowedIn { .. } => {
            "Something has dug its way into the gallery from inside the rock.".to_string()
        }
        Propped { agent, .. } => format!(
            "{} stopped cutting to prop the roof first.",
            named_as(w, e.tick, *agent)
        ),
        CaveIn { agent, hurt, .. } => {
            if *hurt {
                format!("The rock came down on {}; they are hurt.", named_as(w, e.tick, *agent))
            } else {
                format!(
                    "The rock came down near {}; they got clear.",
                    named_as(w, e.tick, *agent)
                )
            }
        }
        CaravanFormed {
            community,
            leader,
            members,
            purpose,
            ..
        } => format!(
            "{} gathers {} of {} {}.",
            named_as(w, e.tick, *leader),
            members,
            cname(w, *community),
            purpose
        ),
        CaravanLeft {
            leader, members, ..
        } => format!(
            "{}'s party of {} walks out past the edge of the egress point.",
            named_as(w, e.tick, *leader),
            members
        ),
        CaravanReturned {
            leader,
            members,
            days,
            site,
            received,
            ..
        } => {
            let goods = if received.is_empty() {
                "nothing to show for it".to_string()
            } else {
                format!("bringing {received}")
            };
            format!(
                "{}'s party of {} is back from {} after {} days, {}.",
                named_as(w, e.tick, *leader),
                members,
                site,
                days,
                goods
            )
        }
        Crafted { agent, kind, tier } => format!(
            "{} made {} {}.",
            named_as(w, e.tick, *agent),
            crate::items::ItemKind::tier_name(*tier),
            kind.label()
        ),
        ToolBroke {
            agent,
            tier,
            what,
            history,
        } => {
            if what.is_empty() {
                format!(
                    "{}'s {} tool broke.",
                    named_as(w, e.tick, *agent),
                    crate::items::ItemKind::tier_name(*tier)
                )
            } else {
                format!("{}'s {what} broke{history}.", named_as(w, e.tick, *agent))
            }
        }
        DrankTogether { drinkers, what } => {
            let names: Vec<String> = drinkers.iter().map(|a| named_as(w, e.tick, *a)).collect();
            let who = match names.len() {
                0 => String::new(),
                1 => names[0].clone(),
                n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
            };
            format!("Over {what} at the hearth: {who}.")
        }
        GaveRing { giver, to, what } => {
            format!("{} gave {} {what}.", named_as(w, e.tick, *giver), named_as(w, e.tick, *to))
        }
        Possessed { agent, because, .. } => {
            format!("{} is taken by a fey mood, {because}.", named_as(w, e.tick, *agent))
        }
        Proposed {
            agent,
            what,
            because,
        } => format!("{} wants {what}: {because}.", named_as(w, e.tick, *agent)),
        LaidToRest {
            agent,
            rite,
            by,
            in_absence,
        } => {
            if *in_absence {
                format!(
                    "{} was mourned, far from where they fell; {} saw to it.",
                    named_as(w, e.tick, *agent),
                    named_as(w, e.tick, *by)
                )
            } else {
                format!(
                    "{} was {}; {} saw to it.",
                    named_as(w, e.tick, *agent),
                    rite.words(),
                    named_as(w, e.tick, *by)
                )
            }
        }
        TreeFelled { agent, .. } => format!("{} felled a living tree.", named_as(w, e.tick, *agent)),
        Engraved { agent, what, .. } => {
            format!(
                "{} carved it into the wall: {}",
                named_as(w, e.tick, *agent),
                what.trim_end_matches('.')
            )
        }
        TookUpArms { agent, what } => {
            format!("{} took up the {what} against the strangers.", named_as(w, e.tick, *agent))
        }
        ItemNamed {
            agent,
            name: n,
            what,
            because,
        } => {
            if because == "made" {
                format!(
                    "{} made {} so fine it was given a name: {n}.",
                    named_as(w, e.tick, *agent),
                    a_or_an(what)
                )
            } else {
                format!(
                    "They call {}'s {what} {n} now; it {because}.",
                    named_as(w, e.tick, *agent)
                )
            }
        }
        Cherished {
            agent,
            what,
            name: n,
        } => match n {
            Some(n) => format!("{} has come to love {n}.", named_as(w, e.tick, *agent)),
            None => format!("{} has come to love their {what}.", named_as(w, e.tick, *agent)),
        },
        SpokenOf {
            name: n,
            what,
            deeds,
        } => format!("People speak of {n} now, the {what} that {}.", join(deeds)),
        Inherited { agent, from, what } => {
            format!("{} took up {}'s {what}.", named_as(w, e.tick, *agent), named_as(w, e.tick, *from))
        }
        Wounded {
            agent,
            part,
            kind,
            cause,
        } => format!(
            "{} took a {} to the {} {}.",
            named_as(w, e.tick, *agent),
            kind,
            part,
            cause
        ),
        Healed { agent, part } => format!("{}'s {} has healed.", named_as(w, e.tick, *agent), part),
        FellIll { agent, what } => {
            let what = if what.is_empty() { "a fever" } else { what };
            format!("{} has taken {what}.", named_as(w, e.tick, *agent))
        }
        Recovered { agent, what } => {
            let what = if what.is_empty() { "the fever" } else { what };
            let what = what.strip_prefix("a ").map(|x| format!("the {x}")).unwrap_or(what.to_string());
            format!("{} is over {what}.", named_as(w, e.tick, *agent))
        }
        Ailing { agent, what } => format!("{} now has {}.", named_as(w, e.tick, *agent), what),
        FireStarted { cause, .. } => format!("Fire {cause}!"),
        FireOut { burned, .. } => {
            if *burned == 1 {
                "The fire is out; one tile burned.".to_string()
            } else {
                format!("The fire is out; {burned} tiles burned.")
            }
        }
        Burned { what, .. } => format!("The {what} burned."),
        Doused { agent, .. } => format!("{} threw water on the fire.", named_as(w, e.tick, *agent)),
        Flooded { tiles } => format!("The water has risen over {tiles} tiles."),
        Receded => "The flood has drawn back.".to_string(),
        Broke { agent, kind, .. } => {
            format!("{} broke down: {}.", named_as(w, e.tick, *agent), kind)
        }
        FurnitureBroken { agent, what } => {
            // #1791: an owned piece is named theirs ("Moreya's bed"), not "the Moreya's bed".
            let article = if what.contains('\'') { "" } else { "the " };
            format!("{} smashed {article}{what}.", named_as(w, e.tick, *agent))
        }
        Fought { a, b } => format!("{} and {} came to blows.", named_as(w, e.tick, *a), named_as(w, e.tick, *b)),
        StoodAgainst { beast: b, by, felled } => {
            let who = match by.len() {
                1 => named_as(w, e.tick, by[0]),
                2 => format!("{} and {}", named_as(w, e.tick, by[0]), named_as(w, e.tick, by[1])),
                n => format!("{}, {} and {} others", named_as(w, e.tick, by[0]), named_as(w, e.tick, by[1]), n - 2),
            };
            if *felled {
                format!("{who} stood against {} and killed it.", beast(w, e.tick, *b))
            } else {
                format!("{who} stood against {} and drove it off.", beast(w, e.tick, *b))
            }
        }
        Routed { site, lost } => format!(
            "The strangers from {site} lost their nerve after {lost} of them were hurt, and ran for home."
        ),
        Shifted { agent } => format!(
            "Under the full moon {} went out in a beast's shape.",
            named_as(w, e.tick, *agent)
        ),
        SpellBegan { how, .. } => how.clone(),
        SpellLifted { what, days } => format!("After {days} days, {what} lifted from the area."),
        Faring {
            community,
            now,
            why,
        } => format!("{community} is {now}: {why}."),
        LastOne { agent, community } => format!(
            "{} is the last of {community} left in the colony.",
            named_as(w, e.tick, *agent)
        ),
        WentAway { agent, to, why } => {
            format!("{} set out alone for {to}, {why}.", named_as(w, e.tick, *agent))
        }
        CameBack { agent, from } => format!("{} came back from {from}.", named_as(w, e.tick, *agent)),
        RiteOutcome {
            agent,
            by,
            outcome,
            how,
        } => {
            if outcome == "marred" {
                format!(
                    "The rite {} gave {} was marred: {how}. They do not rest.",
                    named_as(w, e.tick, *by),
                    named_as(w, e.tick, *agent)
                )
            } else {
                format!(
                    "{} was laid to rest as they should be: {how}.",
                    named_as(w, e.tick, *agent)
                )
            }
        }
        LimbTaken { agent, by, part } => format!(
            "{} took {}'s {part}, gone bad, to save them.",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *agent)
        ),
        LimbMade { agent, by, what } => {
            format!("{} made {} {what}.", named_as(w, e.tick, *by), named_as(w, e.tick, *agent))
        }
        MadeUp { a, b } => format!("{} and {} made it up.", named_as(w, e.tick, *a), named_as(w, e.tick, *b)),
        Repaid { by, to } => format!(
            "{} shared their food with {}, who once did them a good turn.",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *to)
        ),
        RivalMade { name: n, foe } => format!(
            "{n} fought {} and lived, and will remember them.",
            named_as(w, e.tick, *foe)
        ),
        RivalReturned { name: n, title, foe } => {
            let n = if title.is_empty() { n.clone() } else { format!("{n} {title}") };
            format!("{n} has come back, looking for {}.", named_as(w, e.tick, *foe))
        }
        RivalMet {
            name: n,
            title,
            foe,
            meetings,
        } => {
            let n = if title.is_empty() { n.clone() } else { format!("{n} {title}") };
            format!("{n} and {} met again, the {} time.", named_as(w, e.tick, *foe), crate::view::narrate::nth(*meetings))
        }
        RivalFell {
            name: n,
            title,
            foe,
            meetings,
        } => {
            let n = if title.is_empty() { n.clone() } else { format!("{n} {title}") };
            format!(
                "{n} is dead, after {meetings} meetings with {}.",
                named_as(w, e.tick, *foe)
            )
        }
        SnareSet { agent } => format!("{} set a snare.", named_as(w, e.tick, *agent)),
        Snared { agent } => {
            let a = w.agents.iter().find(|a| a.id == *agent);
            if a.is_some_and(|a| a.species == crate::agents::Species::Person) {
                format!("{} was caught by the leg in a snare.", named_as(w, e.tick, *agent))
            } else {
                format!("A snare took {}.", beast(w, e.tick, *agent))
            }
        }
        Tamed { agent, animal } => format!(
            "{} tamed {} into the herd.",
            named_as(w, e.tick, *agent),
            beast(w, e.tick, *animal)
        ),
        Fished { agent, count, ice } => {
            let at = if *ice { "through the ice" } else { "at the water" };
            match count {
                0 => format!("{} caught nothing {at}.", named_as(w, e.tick, *agent)),
                1 => format!("{} caught a fish {at}.", named_as(w, e.tick, *agent)),
                n => format!("{} caught {n} fish {at}.", named_as(w, e.tick, *agent)),
            }
        }
        Drawn { animal } => format!(
            "{} has caught the smell of the stores and is coming in.",
            beast(w, e.tick, *animal)
        ),
        StoresRaided {
            animal,
            took,
            broke,
        } => {
            let mut t = format!("{} got into the stores and ate {took}", beast(w, e.tick, *animal));
            if !broke.is_empty() {
                t.push_str(&format!(", and broke the {broke}"));
            }
            t + "."
        }
        CameOfAge { agent, best } => format!(
            "{} came of age, best at {best}.",
            named_as(w, e.tick, *agent)
        ),
        MineSunk { community, why } => format!(
            "{} set about sinking their shaft deeper, for {why}.",
            cname(w, *community)
        ),
        ReachedDeep { community, rock } => format!(
            "{}'s shaft came down into the {rock}, the deep beneath the egress point.",
            cname(w, *community)
        ),
        LeftUnrested { agent, why } => {
            format!("{} was given no rite: {why}.", named_as(w, e.tick, *agent))
        }
        DeepChosen {
            community,
            cavern,
            walled,
        } => {
            if *walled {
                format!(
                    "{} would go no further into {cavern}, and set about walling the way.",
                    cname(w, *community)
                )
            } else {
                format!(
                    "{} left the way into {cavern} open and dug on, for what the deep holds.",
                    cname(w, *community)
                )
            }
        }
        AccountTold {
            teller,
            listener,
            version,
            changed,
            ..
        } => {
            if *changed {
                format!(
                    "In {}'s telling to {}, {version}.",
                    named_as(w, e.tick, *teller),
                    named_as(w, e.tick, *listener)
                )
            } else {
                format!("{} told {} that {version}.", named_as(w, e.tick, *teller), named_as(w, e.tick, *listener))
            }
        }
        GraveRead { agent, guess, .. } => format!(
            "{} stood at a grave and read it: {guess}.",
            named_as(w, e.tick, *agent)
        ),
        RoleLeft { agent, role } => {
            format!("{} has stopped doing {} work.", named_as(w, e.tick, *agent), role)
        }
        StirBelow { cavern, heard } => {
            let names: Vec<String> = heard.iter().map(|a| named_as(w, e.tick, *a)).collect();
            format!(
                "Something moved below, in {cavern}, where the work was loud; {} heard it.",
                join(&names)
            )
        }
        Thinned {
            agent,
            part,
            lost,
            left,
            cause,
        } => {
            let who = name(w, *agent);
            match part.as_str() {
                "body" => format!(
                    "{who} lost a {} part of itself {cause}.",
                    if *lost >= 25 { "great" } else { "small" }
                ),
                "members" => format!("{who} lost {lost} of their number {cause}; {left} are left."),
                _ => format!("{who} lost {lost} of its {part} {cause}, {left} left."),
            }
        }
        Divided { agent, .. } => format!(
            "{} was cut in two, and both halves crept on.",
            name(w, *agent)
        ),
        Crept { agent, how, .. } => {
            if how == "squeezed through" {
                format!("{} squeezed through the gaps of a wall.", name(w, *agent))
            } else {
                format!("{} {how} the rock.", name(w, *agent))
            }
        }
        CameBackUnsettled { agent } => format!(
            "{} came back from the portal stone unsettled and could not say why.",
            name(w, *agent)
        ),
        WouldNotCut { agent, .. } => format!(
            "{} stopped at the face and would not cut it, and could not say why.",
            name(w, *agent)
        ),
        Frosted { community, plants } => format!(
            "A killing frost took {plants} of {}'s growing crop.",
            cname(w, *community)
        ),
        Fostered { child, by, kin } => format!(
            "{} took in {}, whose parents are gone{}.",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *child),
            match kin {
                1 => ", their grandparent",
                2 => ", their aunt or uncle",
                3 => ", their sibling",
                _ => "",
            }
        ),
        NewStone { agent, stone } => format!(
            "{} cut {stone}, harder than any stone the colony has worked.",
            named_as(w, e.tick, *agent)
        ),
        SocietyFormed { against, members } => format!(
            "{members} who have no love for {} have begun to meet by night, apart.",
            named_as(w, e.tick, *against)
        ),
        Betrayed { by, to, members } => format!(
            "{} told {} of the {members} who meet against them by night.",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *to)
        ),
        StruckWater { agent, tiles } => format!(
            "{} cut into wet rock and the water came in, {tiles} tiles of the workings under it.",
            named_as(w, e.tick, *agent)
        ),
        FirstWord { community, people } => format!(
            "{} know now that there are people of {people} beyond the colony; where they live, nobody knows.",
            cname(w, *community)
        ),
        Torn { agent, with, and } => format!(
            "{} went with {}, but their kin stayed with {}.",
            named_as(w, e.tick, *agent),
            cname(w, *with),
            cname(w, *and)
        ),
        Omen { reader, of, good } => {
            let reading = if *good { "a blessing" } else { "a warning" };
            format!("{} read {of} as {reading}.", named_as(w, e.tick, *reader))
        }
        CarriedOff { things, .. } => {
            format!("The flood carried off what lay in its way, {things} things of it.")
        }
        RanDry { water, .. } => {
            format!("No rain for days: {water} has run dry, its bed bare but for pools.")
        }
        RanAgain { water, .. } => format!("The rain came, and {water} runs again."),
        HighWater { water, spring } => {
            if *spring {
                format!("The spring flood: the winter's snow has melted and {water} is over its banks, too deep to ford.")
            } else {
                format!("In spate after the rain, {water} is over its banks and too deep to ford.")
            }
        }
        PondLeft { tiles, .. } => {
            format!("The flood fell and left a pond in the low ground, {tiles} tiles of it.")
        }
        WaterBrokeIn { agent, tiles } => format!(
            "The water broke in where {} was digging and poured down the shaft, {tiles} tiles of it.",
            named_as(w, e.tick, *agent)
        ),
        DeityRead { reader, words, .. } => format!("{} said {words}.", named_as(w, e.tick, *reader)),
        Scoffed { by, at } => format!(
            "{} said it was only weather, whatever {} made of it.",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *at)
        ),
        SignSettled {
            reader, good, dead, ..
        } => {
            if *good {
                format!(
                    "{} died, and {}'s blessing was doubted.",
                    named_as(w, e.tick, *dead),
                    named_as(w, e.tick, *reader)
                )
            } else {
                format!(
                    "When {} died they remembered {}'s warning.",
                    named_as(w, e.tick, *dead),
                    named_as(w, e.tick, *reader)
                )
            }
        }
        FaithTurned { agent, heeds: true } => {
            format!("{} has come to heed the signs.", named_as(w, e.tick, *agent))
        }
        FaithTurned { agent, heeds: false } => {
            format!("{} no longer heeds the signs.", named_as(w, e.tick, *agent))
        }
        Observance {
            by,
            community,
            season,
            came,
            at_shrine,
        } => {
            if *at_shrine {
                format!(
                    "{} kept the {season} observance at the shrine, led by {}, {came} with them.",
                    cname(w, *community),
                    named_as(w, e.tick, *by)
                )
            } else {
                format!(
                    "{} kept the {season} observance, led by {}, {came} with them.",
                    cname(w, *community),
                    named_as(w, e.tick, *by)
                )
            }
        }
        ObservanceLapsed {
            community,
            season,
            why,
        } => format!(
            "{} let the {season} observance lapse: {why}.",
            cname(w, *community)
        ),
        Rose { beast: b, cavern } => {
            format!("{} came up out of the deep through {cavern}.", beast(w, e.tick, *b))
        }
        Besieged { site, size } => format!(
            "{size} from {site}, come over a grudge, camped and laid siege to the colony."
        ),
        SiegeLifted {
            site,
            days,
            taken,
            captives,
            why,
        } => {
            let took = match (taken, captives) {
                (0, 0) => "with nothing".to_string(),
                (t, 0) => format!("with {t} portions of food"),
                (0, c) => format!("with {c} of the colony's people"),
                (t, c) => format!("with {t} portions of food and {c} of the colony's people"),
            };
            // Why it was lifted (#402); older records have none.
            let why = match why.as_str() {
                "" => String::new(),
                w => format!(", {w}"),
            };
            format!(
                "After {days} days the siege from {site} was lifted{why}; they went home {took}."
            )
        }
        HostSated {
            site,
            days,
            taken,
            captives,
        } => {
            let took = match (taken, captives) {
                (0, 0) => "with nothing".to_string(),
                (t, 0) => format!("with {t} portions of food"),
                (0, c) => format!("with {c} of the colony's people"),
                (t, c) => format!("with {t} portions of food and {c} of the colony's people"),
            };
            format!(
                "After {days} days the host from {site} had what it came for and went home {took}."
            )
        }
        Captured { agent, site } => {
            format!("{} was brought down and carried off to {site}.", named_as(w, e.tick, *agent))
        }
        Held { agent } => format!(
            "{}, hurt in the fight, was taken and kept at the houses.",
            named_as(w, e.tick, *agent)
        ),
        Ransomed { agent, site, food } => format!(
            "{site} paid {food} meals for {}, who went home to them.",
            named_as(w, e.tick, *agent)
        ),
        TookIn { agent } => format!(
            "Nobody came for {}, and the colony took them in as one of its own.",
            named_as(w, e.tick, *agent)
        ),
        Returned { agent, site } => {
            format!("{} came home, let go by {site}.", named_as(w, e.tick, *agent))
        }
        Followed { agent, domain } => format!("The colony looked to {} {domain}.", named_as(w, e.tick, *agent)),
        Accused { by, of, what } => {
            format!("{} accused {} of {what}.", named_as(w, e.tick, *by), named_as(w, e.tick, *of))
        }
        Judged {
            judge,
            of,
            what,
            verdict,
            wrongly,
        } => {
            // #526 (2026-10-31, c137): keep a recorded wrongful judgement distinct in feed/history.
            if *wrongly {
                format!(
                    "{} wrongly judged {} for {what}: {verdict}.",
                    named_as(w, e.tick, *judge),
                    named_as(w, e.tick, *of)
                )
            } else {
                format!(
                    "{} judged {} for {what}: {verdict}.",
                    named_as(w, e.tick, *judge),
                    named_as(w, e.tick, *of)
                )
            }
        }
        Exiled { agent, to } => format!(
            "{} was sent out of the colony, and went to {to}.",
            named_as(w, e.tick, *agent)
        ),
        Feud { a, b } => format!(
            "The families of {} and {} are in a feud now.",
            named_as(w, e.tick, *a),
            named_as(w, e.tick, *b)
        ),
        FeudInherited {
            agent,
            from,
            against,
        } => format!(
            "{} has taken up {}'s feud with the family of {}.",
            named_as(w, e.tick, *agent),
            named_as(w, e.tick, *from),
            named_as(w, e.tick, *against)
        ),
        FeudEnded { a, b, how, by } => {
            let by = by.map(|j| format!(" {}", named_as(w, e.tick, j))).unwrap_or_default();
            format!(
                "The feud between the families of {} and {} is over: {how}{by}.",
                named_as(w, e.tick, *a),
                named_as(w, e.tick, *b)
            )
        }
        Killing { agent, by } => format!(
            "{} died of the blows {} struck in a fight.",
            named_as(w, e.tick, *agent),
            named_as(w, e.tick, *by)
        ),
        Maiming { agent, by, part } => format!(
            "{} lost their {part} to a wound {} gave them in a fight.",
            named_as(w, e.tick, *agent),
            named_as(w, e.tick, *by)
        ),
        CavernFound {
            cavern,
            paces,
            pool,
            underworld,
        } => {
            let mut t = format!("{cavern} is a chamber of {paces} paces");
            if *pool {
                t.push_str(", with a still pool");
            }
            if *underworld {
                t.push_str(", its walls of the underworld's stone");
            }
            t + "."
        }
        Sealed { cavern } => format!("The way into {cavern} was walled up."),
        GateOpened { agent, cavern } => format!(
            "{} cut out the wall over the way down to {cavern}, and the deep was open again.",
            named_as(w, e.tick, *agent)
        ),
        DeepPact {
            against,
            site,
            members,
            ..
        } => format!(
            "The {members} who meet against {} by night have had word with the people of {site}, below.",
            named_as(w, e.tick, *against)
        ),
        SocietyExposed {
            against,
            members,
            from,
        } => format!(
            "It came out that {members} had met by night against {} and bargained with {from}; the colony turned from them.",
            named_as(w, e.tick, *against)
        ),
        LetIn {
            by, from, against, ..
        } => format!(
            "{} let the people of {from} up out of the deep, for the grudge held against {}.",
            named_as(w, e.tick, *by),
            named_as(w, e.tick, *against)
        ),
        TakenBelow { agent, from } => format!(
            "{} was taken in the night and carried down into the deep, to {from}.",
            named_as(w, e.tick, *agent)
        ),
        GateFoundOpen { cavern, child } => match child {
            Some(c) => format!(
                "In the morning the way down to {cavern} stood open, and {} was gone.",
                named_as(w, e.tick, *c)
            ),
            None => format!("In the morning the way down to {cavern} stood open."),
        },
        WentDownAfter { agents, child, to } => {
            let who: Vec<String> = agents.iter().map(|a| named_as(w, e.tick, *a)).collect();
            format!(
                "{} went down into the deep after {}, toward {to}.",
                join(&who),
                named_as(w, e.tick, *child)
            )
        }
        CameUpFromBelow {
            child,
            by,
            outcome,
            lost,
            from,
        } => match outcome.as_str() {
            "brought back" => format!(
                "{} came up out of the deep with {}, brought back from {from}.",
                named_as(w, e.tick, *by),
                named_as(w, e.tick, *child)
            ),
            "lost" => format!(
                "Those who went down after {} came up without them; {} was lost in the deep.",
                named_as(w, e.tick, *child),
                lost.map(|l| named_as(w, e.tick, l)).unwrap_or_default()
            ),
            "hurt" => format!(
                "{} and the others came up out of the deep hurt, and without {}.",
                named_as(w, e.tick, *by),
                named_as(w, e.tick, *child)
            ),
            _ => format!(
                "{} and the others came up out of the deep with no sign of {}.",
                named_as(w, e.tick, *by),
                named_as(w, e.tick, *child)
            ),
        },
        BrokeIn { beast, of, door } => {
            let what = if *door { "door" } else { "wall" };
            format!(
                "{} broke down a {of} {what} and came in after the people inside.",
                named_as(w, e.tick, *beast)
            )
        }
        RelicMade {
            community,
            of,
            what,
            why,
        } => format!(
            "The {what} of {}, {why}, became a relic of {} and was set down to be kept.",
            named_as(w, e.tick, *of),
            cname(w, *community)
        ),
        RelicLost {
            community,
            what,
            why,
            by,
        } => match by {
            Some(by) => format!(
                "{} lost its relic, the {what}: {why}, by {}.",
                cname(w, *community),
                named_as(w, e.tick, *by)
            ),
            None => format!("{} lost its relic, the {what}: {why}.", cname(w, *community)),
        },
        Pilgrimage { agent, to, why } => {
            format!("{}, {why}, went to {to} to pray.", named_as(w, e.tick, *agent))
        }
        FactionFormed {
            leader,
            keeper,
            community,
            members,
        } => format!(
            "{} and {} more of {} hold the signs apart from {}, their keeper.",
            named_as(w, e.tick, *leader),
            members.saturating_sub(1),
            cname(w, *community),
            named_as(w, e.tick, *keeper)
        ),
        FactionObservance { by, came, .. } => format!(
            "{} kept the season's observance apart, {came} of those who hold with them there.",
            named_as(w, e.tick, *by)
        ),
        SoughtKin { agent, from, years } => format!(
            "{}, taken below {years} years ago and grown among the people of {from}, came up to look for their family.",
            named_as(w, e.tick, *agent)
        ),
        KinMet { agent, by, outcome } => match by {
            Some(by) => format!(
                "{} met {}, grown below: {outcome}.",
                named_as(w, e.tick, *by),
                named_as(w, e.tick, *agent)
            ),
            None => format!(
                "{} found none of their family: {outcome}.",
                named_as(w, e.tick, *agent)
            ),
        },
        MeetingSeen { by, members, .. } => format!(
            "{} saw {members} of their own gathered apart in the night, away from the houses, and wondered why.",
            named_as(w, e.tick, *by)
        ),
        FactionEnded {
            leader,
            community,
            how,
        } => format!(
            "Those of {} who held the signs apart with {} are one with the rest again: they {how}.",
            cname(w, *community),
            named_as(w, e.tick, *leader)
        ),
        WallBroken { site, of, days } => format!(
            "After {days} days of battering, the host from {site} brought down a stretch of the {of} wall."
        ),
        GrownBelow { agent, from, years } => format!(
            "Among the people of {from} was {}, taken below {years} years ago, grown now and one of them.",
            named_as(w, e.tick, *agent)
        ),
        ShrineLost { community, why } => {
            format!("{} lost its shrine: {why}.", cname(w, *community))
        }
        KeeperNamed { agent, community } => format!(
            "{} was named keeper of the observances of {}.",
            named_as(w, e.tick, *agent),
            cname(w, *community)
        ),
        KeeperChallenged {
            by,
            keeper,
            won,
            with_by,
            with_keeper,
            ..
        } => {
            if *won {
                format!(
                    "{} challenged {} for the keeping and took it, {with_by} standing with them and {with_keeper} against.",
                    named_as(w, e.tick, *by),
                    named_as(w, e.tick, *keeper)
                )
            } else {
                format!(
                    "{} challenged {} for the keeping and was turned back, {with_keeper} standing with the keeper and {with_by} with them.",
                    named_as(w, e.tick, *by),
                    named_as(w, e.tick, *keeper)
                )
            }
        }
        Barred { beast, cavern } => {
            format!("{beast} came up through {cavern} and found the way walled.")
        }
        Tended { healer, patient } => {
            format!("{} tended {}'s hurts.", named_as(w, e.tick, *healer), named_as(w, e.tick, *patient))
        }
        NewClothes {
            agent,
            tier,
            worn_through,
            history,
        } => {
            let new = format!(
                "{} put on {} new clothes",
                named_as(w, e.tick, *agent),
                crate::items::ItemKind::tier_name(*tier)
            );
            match worn_through.as_str() {
                "" => format!("{new}."),
                "clothes" => format!("{new}; the old ones were worn through{history}."),
                old => format!("{new}; {old}, the old ones, was worn through{history}."),
            }
        }
        CaravanLost {
            community,
            members,
            last,
            ..
        } => format!(
            "The party of {} from {} never came back; the last word was: {}.",
            members,
            cname(w, *community),
            last
        ),
    }
}

/// The longer record behind a headline, for the history view: filled only from the
/// event's own fields, like `line`.
pub fn detail(e: &Event) -> Option<String> {
    use EventKind::*;
    match &e.kind {
        CaravanReturned { story, .. } => Some(format!("On the road: {story}.")),
        CaravanLost { last, .. } => Some(format!("The last word was: {last}.")),
        // A custom's narrative is in its line already (#226: it was printed twice).
        CustomFormed { .. } => None,
        _ => None,
    }
}

/// Viewer detail that can depend on which people the viewer has found.
pub fn contextual_detail(w: &World, e: &Event) -> Option<String> {
    match &e.kind {
        EventKind::Broke { explanation, .. } => match explanation {
            Some(BreakExplanation::Known {
                about: Some(about), ..
            }) if !crate::view::known::is_unfound(w, *about) => {
                Some(format!("It was over {}.", named_as(w, e.tick, *about)))
            }
            Some(BreakExplanation::Known { about: None, cause }) => {
                Some(format!("They broke down {cause}."))
            }
            Some(BreakExplanation::Unknown) => Some("No one could name the cause.".into()),
            // A hidden subject hides only the explanation; a legacy event had none.
            _ => None,
        },
        _ => detail(e),
    }
}

/// One thing of a kind with its article, for a line about a single item (#1727): "a hammer",
/// "an iron tool", "some clothes". `ItemKind::label` is the word for a stock ("clothes",
/// "torches"), so trimming its "s" told of "A clothe" and "a torche".
pub fn a_thing(kind: crate::items::ItemKind, material: crate::world::Material) -> String {
    use crate::items::ItemKind as K;
    match kind {
        // No "one" of these: a heap, a set or a measure.
        K::Clothes
        | K::Berries
        | K::IronOre
        | K::CopperOre
        | K::Ore
        | K::Coal
        | K::RawMeat
        | K::Cloth
        | K::Firewood
        | K::Fibre
        | K::Clay
        | K::Sand
        | K::Leather
        | K::Ale
        | K::Wine
        | K::RoughStone
        | K::Thatch
        | K::Dye => format!("some {}", kind.label()),
        K::Torch => "a torch".into(),
        K::GrainSheaf => "a sheaf of grain".into(),
        K::StoneBlock => "a stone block".into(),
        _ => a_or_an(&crate::legend::noun(kind, material)),
    }
}

/// "some clothes" -> "Some clothes", to open a line.
fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// "a figure", "an oak figure".
pub fn a_or_an(what: &str) -> String {
    let vowel = what
        .chars()
        .next()
        .is_some_and(|c| "aeiouAEIOU".contains(c));
    format!("{} {what}", if vowel { "an" } else { "a" })
}

/// "second", "third"... for counts of meetings.
pub fn nth(n: u32) -> String {
    match n {
        1 => "first".into(),
        2 => "second".into(),
        3 => "third".into(),
        4 => "fourth".into(),
        5 => "fifth".into(),
        n => format!("{n}th"),
    }
}
