//! The arrival (see docs/design-emergence.md).
//!
//! Ten survivors came through the portal. Three begin together at the landing site; the other
//! seven are scattered across the world with unknown fate. Nothing here invents history: whether
//! a missing survivor is alive is discovered in play (rumour or observation), never assumed.
//!
//! A reroll from the setup screen changes one slot's identity and nothing else, because each
//! slot's identity is keyed by `(index, rerolls)` in the purpose-tagged stream (D7).
use crate::fmath::Det;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::agents::{Agent, Species};
use crate::ids::{AgentId, TilePos};
use crate::rng::{purpose, stream};
use crate::World;

pub const PARTY: usize = 10;
pub const STARTING: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SurvivorState {
    /// One of the three who begin at the landing site.
    Starting,
    /// Scattered. `at` is where they are; `fate_known` is false until someone goes and looks.
    /// `truth` is what actually happened — decided when the world was made, and not given to any
    /// survivor: it is what a search discovers, not what anyone is told.
    Missing {
        at: TilePos,
        fate_known: bool,
        truth: Option<bool>,
    },
    /// Came through alone, somewhere in the valley, and is living or dying there as a real
    /// person (`agent`): nothing decides their fate but the world. `found` once someone of the
    /// party has seen them, alive or dead. (Replaces `Missing`, which only old saves carry.)
    Lost { agent: AgentId, found: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurvivorSlot {
    pub index: u8,
    /// Rerolls from the setup screen. Part of the identity key, so rerolling is deterministic.
    #[serde(default)]
    pub rerolls: u32,
    pub state: SurvivorState,
    /// Their life before the portal, recorded at the arrival (chronicle event ids, oldest first).
    #[serde(default)]
    pub past_events: Vec<crate::ids::EventId>,
}

/// The pool for a given seed and reroll counters. Rerolling slot `i` changes only slot `i`.
pub fn pool(seed: u64, rerolls: &[u32; PARTY]) -> Vec<SurvivorSlot> {
    // `seed` is part of the contract even though each slot's identity is derived from it at spawn
    // time: a pool belongs to a world, and rerolls are per slot so they cannot disturb the others.
    debug_assert!(rerolls.len() == PARTY);
    let _ = seed;
    (0..PARTY as u8)
        .map(|i| SurvivorSlot {
            index: i,
            rerolls: rerolls[i as usize],
            state: SurvivorState::Missing {
                at: TilePos::new(0, 0, 0),
                fate_known: false,
                truth: None,
            },
            past_events: Vec::new(),
        })
        .collect()
}

/// Word from outside: someone matching one of the party was seen over that way. Nobody knows it
/// is true. It is a reason to go and look, not knowledge.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rumour {
    /// Which of the ten this was about.
    pub who: u8,
    /// Roughly where they were said to be.
    pub at: TilePos,
    /// When this was heard.
    pub heard: u64,
    /// Who said it, when someone said it.
    pub from: Option<AgentId>,
}

/// Someone setting out to look for one of the party. Settled when they reach the ground.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Search {
    pub by: AgentId,
    pub who: u8,
    pub at: TilePos,
    pub started: u64,
    /// Places looked at so far: with no sign at the first, they try nearby before giving up.
    #[serde(default)]
    pub tries: u8,
}

/// Strangers walking in from the edge carry word only of what they actually passed: any of
/// the lost (or their body) within sight of their way in. Each of them now holds it, roughly
/// placed. Nobody else knows until they are told (design-organic.md section 5).
pub fn passed_on_the_way(w: &mut World, walkers: &[AgentId], from: TilePos, to: TilePos) {
    let now = w.meta.tick;
    let mut seen: Vec<(u8, TilePos)> = Vec::new();
    for s in &w.survivor_pool {
        if let SurvivorState::Lost {
            agent,
            found: false,
        } = s.state
        {
            if let Some(a) = w.agents.iter().find(|a| a.id == agent) {
                if dist_to_segment(a.pos, from, to) <= 12.0 {
                    seen.push((s.index, a.pos));
                }
            }
        }
    }
    for (who, at) in seen {
        let day = now / crate::time::TICKS_PER_DAY;
        let mut r = stream(w.meta.seed, &[purpose::OPENING, 0x8_0000, who as u64, day]);
        let near = TilePos::new(
            (at.x + r.gen_range(-3..=3)).clamp(1, w.grid.width - 2),
            (at.y + r.gen_range(-3..=3)).clamp(1, w.grid.height - 2),
            at.z,
        );
        for id in walkers {
            if let Some(a) = w.agents.iter_mut().find(|a| a.id == *id) {
                a.hear(Rumour {
                    who,
                    at: near,
                    heard: now,
                    from: None,
                });
            }
        }
    }
}

fn dist_to_segment(p: TilePos, a: TilePos, b: TilePos) -> f32 {
    let (px, py) = (p.x as f32, p.y as f32);
    let (ax, ay, bx, by) = (a.x as f32, a.y as f32, b.x as f32, b.y as f32);
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (ax + t * dx, ay + t * dy);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// A rumour older than twenty days is not worth walking to.
pub const RUMOUR_DAYS: u64 = 20;

/// The lost and the party meeting by sight: a lost survivor seen by any member of a community
/// (both awake, within eight tiles) is with them again; a body seen close is their fate known.
/// Runs every 20 ticks.
/// How someone died, as words that follow "died alone": "of the cold", "of hunger", "mauled
/// by a bear". Causes are recorded as short words ("froze", "starvation"), not all of them
/// nouns.
pub fn died_how(cause: &str) -> String {
    match cause {
        "froze" => "of the cold".into(),
        "starvation" => "of hunger".into(),
        "old age" => "of old age".into(),
        c if c.starts_with("lost ")
            || c.starts_with("bitten")
            || c.starts_with("mauled")
            || c.starts_with("brought")
            || c.starts_with("taken")
            || c.starts_with("killed")
            || c.starts_with("under ")
            // #1217: a verb-phrase cause ("drowned through the ice", "bled out") already
            // reads as a full clause after "died", so it takes no "of".
            || c.starts_with("drowned")
            || c.starts_with("bled")
            || c.starts_with("engulfed")
            || c.starts_with("scattered")
            || c.starts_with("torn")
            || c.starts_with("froze ") =>
        {
            c.to_string()
        }
        c => format!("of {c}"),
    }
}

pub fn reunite(w: &mut World) {
    // Every 20 ticks (a minute): people walking past each other in five tiles are seen.
    if w.meta.tick % 20 != 1 {
        return;
    }
    let now = w.meta.tick;
    for a in w.agents.iter_mut() {
        a.heard
            .retain(|r| now.saturating_sub(r.heard) < RUMOUR_DAYS * crate::time::TICKS_PER_DAY);
    }
    let lost: Vec<(usize, AgentId)> = w
        .survivor_pool
        .iter()
        .enumerate()
        .filter_map(|(k, s)| match s.state {
            SurvivorState::Lost {
                agent,
                found: false,
            } => Some((k, agent)),
            _ => None,
        })
        .collect();
    for (k, lost_id) in lost {
        let Some(li) = w.agents.iter().position(|a| a.id == lost_id) else {
            continue;
        };
        let (lpos, lalive, lawake) = {
            let l = &w.agents[li];
            (l.pos, l.alive, !l.activity.is_asleep())
        };
        let reach = if lalive { 5 } else { 3 };
        let finder = w
            .agents
            .iter()
            .filter(|o| {
                o.alive
                    && o.community.is_some()
                    && o.id != lost_id
                    && o.pos.z == lpos.z
                    && o.pos.dist(lpos) <= reach
                    && !o.activity.is_asleep()
            })
            .map(|o| (o.id, o.community.unwrap()))
            .next();
        let Some((fid, cid)) = finder else { continue };
        if lalive && !lawake {
            continue;
        }
        let name = w.agents[li].name.clone();
        if lalive {
            w.join_community(lost_id, cid);
            w.survivor_pool[k].state = SurvivorState::Starting;
            for (x, y) in [
                (li, fid),
                (w.agents.iter().position(|a| a.id == fid).unwrap(), lost_id),
            ] {
                w.agents[x].relate(y, 0.4, 0.8, now);
            }
            w.events.push(
                now,
                Some(lpos),
                crate::events::Severity::Major,
                crate::events::EventKind::Discovered {
                    agent: fid,
                    at: lpos,
                    what: format!("{name}, who had come through the portal alone and lived"),
                },
            );
        } else {
            w.survivor_pool[k].state = SurvivorState::Lost {
                agent: lost_id,
                found: true,
            };
            let cause = w.agents[li]
                .death
                .clone()
                .unwrap_or_else(|| "something".into());
            w.events.push(
                now,
                Some(lpos),
                crate::events::Severity::Major,
                crate::events::EventKind::Discovered {
                    agent: fid,
                    at: lpos,
                    what: format!("where {name} had died, alone, {}", died_how(&cause)),
                },
            );
        }
    }
}

/// Whoever went looking has arrived. Settle it, either way, and let them remember it.
pub fn resolve_pending_searches(w: &mut World) {
    let now = w.meta.tick;
    let mut done: Vec<usize> = Vec::new();
    for (i, s) in w.searches.clone().iter().enumerate() {
        let Some(idx) = w.agents.iter().position(|x| x.id == s.by) else {
            done.push(i);
            continue;
        };
        let (alive, pos) = {
            let a = &w.agents[idx];
            (a.alive, a.pos)
        };
        if !alive {
            done.push(i);
            continue;
        }
        if pos.dist(s.at) > 4 {
            continue;
        }
        let out = resolve_search(w, s.who, pos);
        if !out.found && s.tries < 2 {
            // No sign here: look around nearby, a little further each time.
            let mut r = stream(
                w.meta.seed,
                &[purpose::OPENING, 0x5EA2C, s.by.0 as u64, now],
            );
            let span = 8 + 4 * s.tries as i32;
            let next = w.grid.surface(
                (s.at.x + r.gen_range(-span..=span)).clamp(1, w.grid.width - 2),
                (s.at.y + r.gen_range(-span..=span)).clamp(1, w.grid.height - 2),
            );
            if w.grid.walkable(next) && w.path_agent_to(idx, next) {
                w.agents[idx].activity = crate::mind::Activity::Going {
                    to: next,
                    then: Box::new(crate::mind::Activity::Loitering { ticks_left: 120 }),
                };
                w.agents[idx].activity_ticks = 0;
                if let Some(x) = w.searches.get_mut(i) {
                    x.at = next;
                    x.tries += 1;
                }
                continue;
            }
        }
        if out.found {
            let valence = if out.alive { 1.0 } else { -1.0 };
            w.events.push(
                now,
                Some(pos),
                crate::events::Severity::Major,
                crate::events::EventKind::Discovered {
                    agent: s.by,
                    at: s.at,
                    what: out.note.clone(),
                },
            );
            if let Some(pos) = w.agents.iter().position(|x| x.id == s.by) {
                // Remembered either way: relief and loss both change how they walk from here.
                w.agents[pos].memories.short.push(crate::mind::Record {
                    tick: now,
                    kind: crate::mind::MemKind::Company,
                    subject: crate::mind::Subject::Place(s.at),
                    valence,
                    salience: 1.0,
                    event: None,
                });
            }
            done.push(i);
        } else if now.saturating_sub(s.started) > 3 * crate::time::TICKS_PER_DAY {
            done.push(i); // three days of looking and nothing: they give it up
        }
    }
    for i in done.into_iter().rev() {
        if i < w.searches.len() {
            w.searches.remove(i);
        }
    }
}

/// Who a slot is, before they exist anywhere. Both the setup screen's card and the survivor who
/// steps onto the map come from this, so what you are shown is what you get — by construction,
/// not by keeping two generators in step.
pub struct Identity {
    pub name: String,
    pub genome: crate::genetics::Genome,
    pub appearance: crate::agents::Appearance,
    pub traits: crate::mind::Traits,
    pub dispositions: crate::mind::Dispositions,
    pub skills: crate::mind::Skills,
    pub needs: crate::mind::Needs,
    pub facing: u8,
    /// Years lived before the portal, from the same roll everything else uses.
    pub starting_age_years: u32,
}

/// The key a slot is known by. A reroll is a different draw of the same slot, so it is a
/// different name too: that is what "roll until you like them" has to mean.
pub fn slot_key(index: u8, rerolls: u32) -> u32 {
    // A hash rather than a linear mix: with enough rerolls a linear key can wrap onto another
    // slot's name, and then two survivors answer to the same one. splitmix64, truncated.
    let mut x = (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (rerolls as u64 + 0x9E37_79B9);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (x ^ (x >> 31)) as u32
}

/// What a slot is called, whoever is asking: the setup card, the chronicle, or someone setting
/// out to look for them all come here, so they cannot end up talking about different people.
pub fn slot_name(seed: u64, index: u8, rerolls: u32) -> String {
    crate::names::person_name(seed, slot_key(index, rerolls))
}

pub fn identity(seed: u64, index: u8, rerolls: u32) -> Identity {
    let mut r = stream(seed, &[purpose::OPENING, index as u64, rerolls as u64]);
    let genome = crate::genetics::Genome::random(&mut r);
    let appearance = genome.appearance(&mut r);
    let name = slot_name(seed, index, rerolls);
    // The roll stays (it keeps every later draw where it was); the age is the household's.
    let _rolled_age: u32 = r.gen_range(16..45);
    let starting_age_years = crate::past::age_of(seed, index, rerolls);
    let facing = r.gen_range(0..4);
    let needs = crate::mind::Needs {
        hunger: r.gen_range(0.5..0.95),
        rest: r.gen_range(0.6..1.0),
        ..Default::default()
    };
    let skills = {
        let mut sk = genome.aptitudes();
        for st in sk.states.iter_mut() {
            let lvl: f32 = r.gen_range(0.0..6.0);
            st.xp = -120.0 * (1.0 - lvl / 20.0).d_ln();
            st.level = lvl;
        }
        // What their past taught them (an apprenticeship, two nights lost in the woods).
        let life = crate::past::life(seed, index, rerolls, &genome.traits());
        for (skill, add) in &life.skill_bonus {
            let st = &mut sk.states[*skill as usize];
            let lvl = (st.level + add).min(19.0);
            st.xp = -120.0 * (1.0 - lvl / 20.0).d_ln();
            st.level = lvl;
        }
        sk
    };
    Identity {
        name,
        traits: genome.traits(),
        dispositions: genome.dispositions(),
        genome,
        appearance,
        skills,
        needs,
        facing,
        starting_age_years,
    }
}

impl Identity {
    pub fn into_agent(self, id: AgentId, pos: TilePos, tick: u64, life_speed: f32) -> Agent {
        Agent {
            id,
            name: self.name,
            species: Species::Person,
            pos,
            came: tick,
            died_alone: false,
            born: tick as i64
                - ((crate::time::TICKS_PER_YEAR * self.starting_age_years as u64) as f64
                    / life_speed as f64) as i64,
            appearance: self.appearance,
            alive: true,
            facing: self.facing,
            path: Vec::new(),
            from: pos,
            move_ticks: 0,
            move_total: 0,
            path_blocked: false,
            idle_ticks: 0,
            needs: self.needs,
            traits: self.traits,
            dispositions: Some(self.dispositions),
            skills: self.skills,
            knowledge: Default::default(),
            learning: Default::default(),
            problems: Default::default(),
            activity: Default::default(),
            activity_ticks: 0,
            switches: 0,
            last_meal: None,
            last_sleep: tick,
            bereaved: Vec::new(),
            work_cooldowns: Default::default(),
            tool: None,
            weapon: None,
            armour: None,
            fey: None,
            tales: Vec::new(),
            accounts: Vec::new(),
            rough_nights: 0,
            prayed_over: Vec::new(),
            graves_read: Vec::new(),
            suspicions: Vec::new(),
            whispered: None,
            hearth_heard: Vec::new(),
            resonance: None,
            denned: false,
            fed: 0,
            memories: Default::default(),
            relationships: Default::default(),
            mood: 0.0,
            community: None,
            role: None,
            duties: Default::default(),
            leaving: false,
            bound_for: None,
            genome: self.genome,
            mother: None,
            father: None,
            partner: None,
            children: Vec::new(),
            starving_ticks: 0,
            last_birth: 0,
            death: None,
            age_scale: life_speed,
            injured_until: 0,
            caravan: None,
            last_caravan: 0,
            fought_at: 0,
            people: Default::default(),
            kind: None,
            rite: Default::default(),
            culture: None,
            stories: Vec::new(),
            tool_quality: 0.0,
            tool_wear: 0.0,
            clothes_wear: 1.0,
            furred: false,
            wet: 0.0,
            body: Default::default(),
            tended_until: 0,
            thoughts: Default::default(),
            stress: 0.0,
            last_break: 0,
            withdrawn_until: 0,
            past: Vec::new(),
            home: String::new(),
            beliefs: Default::default(),
            techniques: Default::default(),
            heard: Vec::new(),
            band: None,
            tie: None,
            held: None,
            defers: Vec::new(),
            journey: None,
            immune: Vec::new(),
            shuns: Vec::new(),
            owes: Vec::new(),
            suspects: Vec::new(),
            rival: None,
            feelings: Default::default(),
            witnessed: Vec::new(),
            fragments: Vec::new(),
            hardened: 0.0,
            foster: None,
            kept: None,
            conditions: Vec::new(),
            named_at: None,
            told: Vec::new(),
            conscience: Default::default(),
        }
    }
}

/// What a search party finds when they stand on the ground the missing survivor is on.
/// Nothing here invents a person: the fate was decided when the world was made.
pub struct Found {
    /// Did this settle what happened to them.
    pub found: bool,
    /// Are they coming back.
    pub alive: bool,
    /// Something to say about it, already in the world's own words.
    pub note: String,
}

pub fn resolve_search(w: &mut World, target: u8, at: TilePos) -> Found {
    let none = |note: String| Found {
        found: false,
        alive: false,
        note,
    };
    let Some(slot) = w.survivor_pool.iter().position(|s| s.index == target) else {
        return none(String::new());
    };
    let rerolls = w.survivor_pool[slot].rerolls;
    let name = slot_name(w.meta.seed, target, rerolls);
    let agent = match w.survivor_pool[slot].state {
        SurvivorState::Lost {
            agent,
            found: false,
        } => agent,
        // Asked again: answer rather than go quiet.
        SurvivorState::Starting => return none(format!("{name} is already back with the others")),
        _ => return none(format!("what became of {name} is already known")),
    };
    let Some(li) = w.agents.iter().position(|a| a.id == agent) else {
        return none(String::new());
    };
    let (who, lpos, alive) = {
        let l = &w.agents[li];
        (l.name.clone(), l.pos, l.alive)
    };
    // What is there is what is found: someone looking hard sees about twelve tiles around them;
    // a person who moved further on is not found here.
    if lpos.z != at.z || lpos.dist(at) > 12 {
        return none(format!("no sign of {who} here"));
    }
    let cid = w.communities.iter().find(|c| !c.dissolved).map(|c| c.id);
    if alive {
        if let Some(c) = cid {
            w.join_community(agent, c);
        }
        w.survivor_pool[slot].state = SurvivorState::Starting;
        Found {
            found: true,
            alive: true,
            note: format!("{who} was alive, and is coming back"),
        }
    } else {
        w.survivor_pool[slot].state = SurvivorState::Lost { agent, found: true };
        let cause = w.agents[li]
            .death
            .clone()
            .unwrap_or_else(|| "something".into());
        Found {
            found: true,
            alive: false,
            note: format!("{who} had died there, of {cause}"),
        }
    }
}

impl World {
    /// Spawn one of the party: identity keyed by `(index, rerolls)`, so the setup screen's rerolls
    /// produce a different person for that slot and leave every other slot untouched.
    pub fn spawn_survivor(&mut self, slot: &SurvivorSlot, pos: TilePos) -> AgentId {
        let id = self.alloc_agent_id();
        // The card the viewer was shown and the survivor they get come from one place.
        let it = identity(self.meta.seed, slot.index, slot.rerolls);
        let mut a = it.into_agent(id, pos, self.meta.tick, self.meta.life_speed);
        // What they carry through the portal, drawn once and hidden (#308, resonance.rs).
        a.resonance = crate::resonance::draw(self.meta.seed, slot.index, slot.rerolls);
        a.techniques = crate::mind::Techniques::common(&a.skills, &a.traits, self.meta.tick);
        let recorded = if slot.past_events.is_empty() {
            self.survivor_pool
                .iter()
                .find(|s| s.index == slot.index)
                .map(|s| s.past_events.clone())
                .unwrap_or_default()
        } else {
            slot.past_events.clone()
        };
        if !recorded.is_empty() {
            a.past = recorded;
            a.home = crate::past::home_name(self.meta.seed);
        }
        self.agents.push(a);
        id
    }

    /// They have just come through the portal (#392): not for a survivor an old save
    /// upgrades into a person mid-game.
    fn arrival_shock(&mut self, id: AgentId) {
        let now = self.meta.tick;
        if let Some(a) = self.agents.iter_mut().find(|a| a.id == id) {
            crate::systems::strain::arrival_shock(a, now);
        }
    }

    /// Where one of the lost has made camp: the place they came through. Alone in country they
    /// do not know, they keep near it unless hunger drives them out.
    pub fn lost_camp(&self, index: u8) -> TilePos {
        self.scatter_point(index, 40)
    }

    /// Where a scattered survivor ended up: a seeded walkable tile at least `min_dist` from the
    /// landing site. Never called with knowledge the survivors could not have.
    fn scatter_point(&self, index: u8, min_dist: i32) -> TilePos {
        let home = self.meta.founding_site;
        let mut r = stream(self.meta.seed, &[purpose::OPENING, 0x5CA7, index as u64]);
        let (w, h) = (self.grid.width, self.grid.height);
        for _ in 0..400 {
            let x = r.gen_range(2..w - 2);
            let y = r.gen_range(2..h - 2);
            let p = self.grid.surface(x, y);
            if self.grid.walkable(p) && p.dist(home) >= min_dist {
                return p;
            }
        }
        home
    }

    /// Three distinct slots, in order. Anything else a caller might ask for (repeat indices, an
    /// index past the ten) is made good here rather than spawning the same person twice or a
    /// survivor who belongs to no slot.
    fn sanely_chosen(chosen: &[u8; STARTING]) -> [u8; STARTING] {
        let mut out = [0u8; STARTING];
        let mut used = 0usize;
        for idx in chosen {
            let idx = (*idx as usize) % PARTY;
            if out[..used].contains(&(idx as u8)) {
                continue;
            }
            out[used] = idx as u8;
            used += 1;
            if used == STARTING {
                return out;
            }
        }
        // Top up from whichever slots have not been taken, lowest first.
        let mut next = 0u8;
        while used < STARTING {
            while out[..used].contains(&next) {
                next = (next + 1) % PARTY as u8;
            }
            out[used] = next;
            used += 1;
        }
        out
    }

    /// Begin the arrival: three of the pool start at the landing site with the party's provisions,
    /// seven are scattered. `chosen` are slot indices (the default is the first three).
    pub fn begin_opening(&mut self, chosen: &[u8; STARTING], rerolls: &[u32; PARTY]) {
        let chosen = Self::sanely_chosen(chosen);
        // Each slot keeps how many times the viewer rerolled it, which is half of who they are.
        for s in self.survivor_pool.iter_mut() {
            s.rerolls = rerolls.get(s.index as usize).copied().unwrap_or(0);
        }
        self.record_pasts();
        let mut pool = std::mem::take(&mut self.survivor_pool);
        let fs = self.meta.founding_site;
        let cid = self.found_community(fs);

        for (k, idx) in chosen.iter().enumerate() {
            if let Some(slot) = pool.iter_mut().find(|s| s.index == *idx) {
                slot.state = SurvivorState::Starting;
            }
            let p = self.grid.surface(
                (fs.x + (k as i32 % 4) - 2).clamp(1, self.grid.width - 2),
                (fs.y + (k as i32 / 4)).clamp(1, self.grid.height - 2),
            );
            let slot = SurvivorSlot {
                index: *idx,
                rerolls: pool
                    .iter()
                    .find(|s| s.index == *idx)
                    .map(|s| s.rerolls)
                    .unwrap_or(0),
                state: SurvivorState::Starting,
                past_events: pool
                    .iter()
                    .find(|s| s.index == *idx)
                    .map(|s| s.past_events.clone())
                    .unwrap_or_default(),
            };
            let id = self.spawn_survivor(&slot, p);
            self.join_community(id, cid);
            self.carry_past_into(id, slot.index, slot.rerolls);
            self.arrival_shock(id);
            self.events.push(
                self.meta.tick,
                Some(p),
                crate::events::Severity::Notable,
                crate::events::EventKind::Arrived {
                    agent: id,
                    community: Some(cid),
                },
            );
        }

        // The other seven came through too, alone, somewhere in the valley. They are people
        // from the first tick, and what becomes of them is the world's doing, not a roll.
        for entry in pool.iter_mut() {
            if matches!(entry.state, SurvivorState::Starting) {
                continue;
            }
            let at = self.scatter_point(entry.index, 40);
            let slot = SurvivorSlot {
                index: entry.index,
                rerolls: entry.rerolls,
                state: SurvivorState::Starting,
                past_events: entry.past_events.clone(),
            };
            let id = self.spawn_survivor(&slot, at);
            self.carry_past_into(id, slot.index, slot.rerolls);
            self.arrival_shock(id);
            // Some came through hurt.
            let mut r = stream(
                self.meta.seed,
                &[purpose::OPENING, 0xFA7E, entry.index as u64],
            );
            if r.gen_bool(0.35) {
                if let Some(i) = self.agents.iter().position(|a| a.id == id) {
                    let part = [
                        crate::body::Part::LEG_L,
                        crate::body::Part::ARM_R,
                        crate::body::Part::HEAD,
                    ][r.gen_range(0..3)];
                    crate::systems::health::wound(
                        self,
                        i,
                        part,
                        crate::body::WoundKind::Bruise,
                        r.gen_range(0.2..0.6),
                        0.0,
                        "coming through the portal",
                        None,
                    );
                }
            }
            self.events.push(
                self.meta.tick,
                Some(at),
                crate::events::Severity::Trace,
                crate::events::EventKind::Arrived {
                    agent: id,
                    community: None,
                },
            );
            entry.state = SurvivorState::Lost {
                agent: id,
                found: false,
            };
        }

        crate::systems::animals::populate(self);
        self.survivor_pool = pool;
        self.bind_siblings();
        self.drop_provisions(STARTING);
    }

    /// Saves made before the lost were people: each slot still `Missing` and unsettled becomes
    /// a lost survivor standing where the old save said they were; the old world-wide rumours
    /// are given once to the community members, who were the only ones who could act on them.
    pub fn upgrade_old_opening(&mut self) {
        // Only worlds that had an arrival: a world without one keeps its pool untouched.
        if !self
            .survivor_pool
            .iter()
            .any(|s| matches!(s.state, SurvivorState::Starting))
        {
            return;
        }
        let old: Vec<(usize, TilePos)> = self
            .survivor_pool
            .iter()
            .enumerate()
            .filter_map(|(k, s)| match s.state {
                SurvivorState::Missing {
                    at,
                    fate_known: false,
                    ..
                } => Some((k, at)),
                _ => None,
            })
            .collect();
        for (k, at) in old {
            let slot = self.survivor_pool[k].clone();
            let p = self.grid.surface(at.x, at.y);
            let id = self.spawn_survivor(&slot, p);
            self.carry_past_into(id, slot.index, slot.rerolls);
            self.survivor_pool[k].state = SurvivorState::Lost {
                agent: id,
                found: false,
            };
        }
        let rumours = std::mem::take(&mut self.rumours);
        if !rumours.is_empty() {
            for a in self.agents.iter_mut().filter(|a| a.community.is_some()) {
                for r in &rumours {
                    a.hear(*r);
                }
            }
        }
    }

    /// Every one of the ten lived before the portal: their lives go into the chronicle once,
    /// here, at the ticks they happened (below the epoch), and each slot keeps its event ids so
    /// whoever steps through, now or when found, carries their own past.
    fn record_pasts(&mut self) {
        let seed = self.meta.seed;
        let epoch = self.meta.epoch;
        let mut lives = Vec::new();
        for s in &self.survivor_pool {
            let it = identity(seed, s.index, s.rerolls);
            let life = crate::past::life(seed, s.index, s.rerolls, &it.traits);
            lives.push((s.index, it.name, life));
        }
        for (index, name, life) in lives {
            let mut ids = Vec::new();
            for e in &life.events {
                let sev = if e.what.is_trauma()
                    || matches!(e.what, crate::past::Formative::Answered { .. })
                {
                    crate::events::Severity::Notable
                } else {
                    crate::events::Severity::Minor
                };
                let id = self.events.push(
                    crate::past::tick_of(epoch, e),
                    None,
                    sev,
                    crate::events::EventKind::Lived {
                        name: name.clone(),
                        slot: index,
                        age: e.age,
                        what: e.what.clone(),
                    },
                );
                ids.push(id);
            }
            if let Some(s) = self.survivor_pool.iter_mut().find(|s| s.index == index) {
                s.past_events = ids;
            }
        }
    }

    /// What a past leaves on someone at the start: the strongest moments as long memories
    /// (with their chronicle origins), and some weight of stress from the hard ones.
    pub fn carry_past_into(&mut self, id: AgentId, index: u8, rerolls: u32) {
        let seed = self.meta.seed;
        let epoch = self.meta.epoch;
        let it = identity(seed, index, rerolls);
        let life = crate::past::life(seed, index, rerolls, &it.traits);
        let Some(a) = self.agents.iter_mut().find(|a| a.id == id) else {
            return;
        };
        let ids = a.past.clone();
        let mut traumas = 0;
        for (k, e) in life.events.iter().enumerate() {
            let origin = ids.get(k).copied();
            if let (crate::past::Formative::Answered { topic, stance, .. }, Some(src)) =
                (&e.what, origin)
            {
                let s = *stance as f32 / 100.0;
                a.beliefs.hold(
                    *topic,
                    s,
                    0.45 + 0.4 * s.abs(),
                    crate::mind::beliefs::Source::Event(src),
                    crate::past::tick_of(epoch, e),
                );
            }
            // What their life taught them to do.
            {
                use crate::mind::techniques::{Learned, Technique};
                use crate::past::Formative as F;
                let now = self.meta.tick;
                match &e.what {
                    F::FoughtFire => {
                        a.techniques
                            .learn(Technique::BeatFlames, Learned::Past, 0.55, now);
                        a.techniques
                            .learn(Technique::CarryWater, Learned::Past, 0.45, now);
                    }
                    // The crafts of a trade (C70, design rule 3): arrivals bring what their
                    // apprenticeship taught them.
                    F::Apprenticed { craft, master } if craft == "the herb-woman" => {
                        a.techniques
                            .learn(Technique::Poultice, Learned::Past, 0.6, now);
                        let from = format!("apprenticed to {master}");
                        a.techniques.learn_from(
                            Technique::HerbLore,
                            Learned::Past,
                            0.5,
                            now,
                            &from,
                        );
                    }
                    F::Apprenticed { craft, master } if craft == "the carpenter" => {
                        let from = format!("apprenticed to {master}");
                        a.techniques.learn_from(
                            Technique::Carpentry,
                            Learned::Past,
                            0.6,
                            now,
                            &from,
                        );
                    }
                    F::Apprenticed { craft, master } if craft == "the smith" => {
                        let from = format!("apprenticed to {master}");
                        a.techniques.learn_from(
                            Technique::Smithing,
                            Learned::Past,
                            0.6,
                            now,
                            &from,
                        );
                        a.techniques.learn_from(
                            Technique::Smelting,
                            Learned::Past,
                            0.5,
                            now,
                            &from,
                        );
                    }
                    F::Apprenticed { craft, master } if craft == "the quarry" => {
                        let from = format!("apprenticed to {master}");
                        a.techniques
                            .learn_from(Technique::Masonry, Learned::Past, 0.5, now, &from);
                    }
                    F::Apprenticed { craft, master } if craft == "the baker" => {
                        let from = format!("apprenticed to {master}");
                        a.techniques.learn_from(
                            Technique::Hearthcraft,
                            Learned::Past,
                            0.4,
                            now,
                            &from,
                        );
                    }
                    F::LostInWoods => {
                        a.techniques
                            .learn(Technique::ForageFar, Learned::Past, 0.5, now);
                    }
                    _ => {}
                }
            }
            let (kind, valence, strength) = if e.what.is_trauma() {
                traumas += 1;
                (crate::mind::memory::MemKind::Hardship, -0.7, 0.8)
            } else {
                match e.what {
                    crate::past::Formative::KindElder { .. }
                    | crate::past::Formative::Married { .. }
                    | crate::past::Formative::ChildBorn { .. } => {
                        (crate::mind::memory::MemKind::Company, 0.6, 0.6)
                    }
                    crate::past::Formative::Apprenticed { .. } => {
                        (crate::mind::memory::MemKind::Work, 0.4, 0.5)
                    }
                    _ => continue,
                }
            };
            if a.memories.long.len() >= crate::mind::memory::LONG_CAP {
                break;
            }
            a.memories.long.push(crate::mind::memory::Memory {
                kind,
                subject: crate::mind::memory::Subject::None,
                valence,
                strength,
                formed: crate::past::tick_of(epoch, e),
                reinforced: 0,
                last: crate::past::tick_of(epoch, e),
                origins: origin.into_iter().collect(),
            });
        }
        a.stress = (a.stress + 0.07 * traumas as f32).min(0.3);
    }

    /// Siblings who came through together know each other, for better or worse.
    fn bind_siblings(&mut self) {
        let seed = self.meta.seed;
        let tick = self.meta.tick;
        let hs = crate::past::households(seed);
        let here: Vec<(AgentId, u8)> = self
            .survivor_pool
            .iter()
            .filter(|s| matches!(s.state, SurvivorState::Starting))
            .filter_map(|s| {
                let name = slot_name(seed, s.index, s.rerolls);
                self.agents
                    .iter()
                    .find(|a| a.name == name)
                    .map(|a| (a.id, hs[s.index as usize]))
            })
            .collect();
        for i in 0..here.len() {
            for j in 0..here.len() {
                if i != j && here[i].1 == here[j].1 {
                    let other = here[j].0;
                    if let Some(a) = self.agents.iter_mut().find(|a| a.id == here[i].0) {
                        a.relate(other, 0.35, 0.9, tick);
                    }
                }
            }
        }
    }

    /// Provisions a travelling party would carry, recorded against the ledger as `scenario`.
    pub fn drop_provisions(&mut self, n: usize) {
        let fs = self.meta.founding_site;
        let mut spots = Vec::new();
        for dy in -1..=1 {
            for dx in -2..=2 {
                let p = self.grid.surface(
                    (fs.x + dx + 3).clamp(1, self.grid.width - 2),
                    (fs.y + dy + 2).clamp(1, self.grid.height - 2),
                );
                if self.grid.walkable(p) {
                    spots.push(p);
                }
            }
        }
        if spots.is_empty() {
            spots.push(fs);
        }
        let s = |i: usize| spots[i % spots.len()];
        self.drop_item(crate::items::ItemKind::Meal, (n * 2) as u32, s(0));
        self.drop_item(crate::items::ItemKind::Berries, 25, s(1));
        self.drop_item(crate::items::ItemKind::Root, 20, s(2));
        self.drop_item(crate::items::ItemKind::GrainSheaf, 15, s(3));
        self.drop_item(crate::items::ItemKind::Log, 12, s(4));
        // One tool between three people: the party is not equipped for a colony. It came
        // through the portal with them, and has a story from here on (legend.rs).
        self.drop_item(crate::items::ItemKind::Tool, 1, s(5));
        if let Some(id) = self
            .items
            .iter()
            .rev()
            .find(|it| it.kind == crate::items::ItemKind::Tool)
            .map(|it| it.id)
        {
            crate::legend::ensure(self, id, None, "brought through the portal");
        }
    }
}

/// A survivor's formative past as typed events with the chronicle ids they were recorded
/// under (`Agent::past`), oldest first; empty for anyone born here or not of the party.
pub fn formatives(w: &World, id: AgentId) -> Vec<(crate::ids::EventId, crate::past::Formative)> {
    let Some(a) = w.agents.iter().find(|a| a.id == id) else {
        return Vec::new();
    };
    let Some(slot) = w
        .survivor_pool
        .iter()
        .find(|s| slot_name(w.meta.seed, s.index, s.rerolls) == a.name)
    else {
        return Vec::new();
    };
    let it = identity(w.meta.seed, slot.index, slot.rerolls);
    let life = crate::past::life(w.meta.seed, slot.index, slot.rerolls, &it.traits);
    life.events
        .into_iter()
        .zip(a.past.iter().copied())
        .map(|(e, id)| (id, e.what))
        .collect()
}
