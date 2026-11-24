//! Inhabitants and creatures. Phase 1 holds identity, position and appearance; needs, traits,
//! skills, perception, knowledge and memory arrive in Phase 3 as sibling modules.

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, TilePos};
use crate::mind::{Activity, Knowledge, Memories, Needs, Relation, Skills, Traits};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Species {
    /// A person, of whichever sentient species `Agent::people` names (species.rs, P3). Saves
    /// before P3 call it Human.
    #[serde(alias = "Human")]
    Person,
    Deer,
    Boar,
    Wolf,
    Hare,
    Fox,
    Crow,
    Bear,
    Duck,
    /// Appended: wild goats of the stony uplands and fells (placed only on those biomes).
    Goat,
    /// The White Hart (creatures.rs): one in most valleys, never more.
    Hart,
    /// Bats roost in the caverns and flit about when people come in (underground.rs).
    Bat,
    /// The pale crawler: a blind predator of the deepest cavern that stalks people in the
    /// dark and never steps into light.
    Crawler,
    /// A heavy digger that cuts its own tunnels through the rock.
    Burrower,
    /// Appended (#423): a cave slime, a soft body of the deep's wet caverns (plan "soft").
    Slime,
    /// Appended (#423): a swarm of rats, many bodies acting as one (plan "swarm").
    Rats,
    /// Appended (#423): a cave centipede, a chain of segments behind a head (plan "segmented").
    Centipede,
    /// Appended (#336): the blind bear of the deep caverns — a cave-adapted bear that lies up
    /// in the dark of its cavern and mauls whoever comes within reach where no light reaches.
    BlindCaveBear,
    /// Appended (#336): the cave crocodile of the pool caverns — an ambusher at the water's
    /// edge that seizes whoever steps within reach of the pool.
    CaveCroc,
}

/// Visual identity, composed by the renderer from layered sheets.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Appearance {
    pub body: u8,
    pub skin: u8,
    pub hair: u8,
    pub hair_colour: u8,
    pub clothing: u8,
    pub clothing_colour: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub species: Species,
    pub pos: TilePos,
    /// Birth tick; negative for people born before the world's first tick.
    pub born: i64,
    /// Died with none of their people near (C68, #229): the thin place's echoes are of such.
    #[serde(default)]
    pub died_alone: bool,
    /// The tick they came into the valley (C68, #209): their spawn, which for the portal party
    /// is the arrival and for the valley-born their birth. 0 in an old save.
    #[serde(default)]
    pub came: u64,
    pub appearance: Appearance,
    pub alive: bool,
    /// Current facing, 0..4 (S, W, E, N) for the renderer.
    pub facing: u8,
    /// Remaining path, next tile first (reversed storage: pop from the end).
    #[serde(default)]
    pub path: Vec<TilePos>,
    /// Tile the agent is moving from (equals `pos` when standing still).
    #[serde(default = "default_pos")]
    pub from: TilePos,
    /// Ticks elapsed and total for the current tile step (renderer interpolation).
    #[serde(default)]
    pub move_ticks: u32,
    #[serde(default)]
    pub move_total: u32,
    /// Set when a step failed because the tile became unwalkable; cleared by the planner.
    #[serde(default)]
    pub path_blocked: bool,
    /// Ticks until the agent re-evaluates (prevents constant switching).
    #[serde(default)]
    pub idle_ticks: u32,
    #[serde(default)]
    pub needs: Needs,
    #[serde(default)]
    pub traits: Traits,
    /// Persistent dimensional dispositions (#488). `None` in an old save keeps its legacy traits.
    #[serde(default)]
    pub dispositions: Option<crate::mind::Dispositions>,
    #[serde(default)]
    pub skills: Skills,
    #[serde(default)]
    pub knowledge: Knowledge,
    /// Personal observations and revisable lessons (E16); never a colony-wide fact store.
    #[serde(default)]
    pub learning: crate::mind::Learning,
    /// Problems this person has personally noticed and their attempted responses (E16).
    #[serde(default)]
    pub problems: crate::mind::Problems,
    #[serde(default)]
    pub activity: Activity,
    /// Ticks spent in the current activity (for the oscillation guard and the inspect panel).
    #[serde(default)]
    pub activity_ticks: u32,
    /// Count of activity changes (soak metric: goal switches).
    #[serde(default)]
    pub switches: u32,
    /// Tick of the last meal, `None` until they first eat (#425: a spawn tick read as a meal, and
    /// in a world past its history it printed as a day in the thousands). Tick of the last sleep.
    /// Old saves hold a plain tick, which reads as `Some`.
    #[serde(default)]
    pub last_meal: Option<u64>,
    #[serde(default)]
    pub last_sleep: u64,
    /// Work orders this person gave up on recently: designation id -> tick until retry.
    #[serde(default)]
    pub work_cooldowns: std::collections::BTreeMap<u32, u64>,
    #[serde(default)]
    pub memories: Memories,
    #[serde(default)]
    pub relationships: std::collections::BTreeMap<AgentId, Relation>,
    /// -1..1 current mood, recomputed from needs and recent memories.
    #[serde(default)]
    pub mood: f32,
    #[serde(default)]
    pub community: Option<crate::ids::CommunityId>,
    #[serde(default)]
    pub role: Option<crate::society::Role>,
    /// Their recent work by kind and the duties it makes (duties.rs, #454).
    #[serde(default)]
    pub duties: crate::duties::Duties,
    /// Turned away and heading out of the valley.
    #[serde(default)]
    pub leaving: bool,
    /// A community departure's pending realm settlement (#1322), named at the choice but
    /// founded only once the first of its leavers steps off the map.
    #[serde(default)]
    pub bound_for: Option<u32>,
    #[serde(default)]
    pub genome: crate::genetics::Genome,
    #[serde(default)]
    pub mother: Option<AgentId>,
    #[serde(default)]
    pub father: Option<AgentId>,
    #[serde(default)]
    pub partner: Option<AgentId>,
    #[serde(default)]
    pub children: Vec<AgentId>,
    /// Ticks spent at zero hunger (starvation clock) and days without sleep.
    #[serde(default)]
    pub starving_ticks: u32,
    /// Tick of the last birth this person gave or fathered (pacing).
    #[serde(default)]
    pub last_birth: u64,
    /// Death cause, once dead.
    #[serde(default)]
    pub death: Option<String>,
    #[serde(default = "one")]
    pub age_scale: f32,
    /// Hurt until this tick: slower, and rests more.
    #[serde(default)]
    pub injured_until: u64,
    /// Committed to a caravan (id) that is forming at the map edge.
    #[serde(default)]
    pub caravan: Option<u32>,
    /// Tick of the last return from a journey (0 = never went).
    #[serde(default)]
    pub last_caravan: u64,
    /// Tick of the last blow struck or traded in a fight or a hunt (the client draws the
    /// fight frames for a moment after it; P2).
    #[serde(default)]
    pub fought_at: u64,
    /// Which sentient species a person is (species.rs, P3); human for everyone before P3.
    #[serde(default)]
    pub people: crate::species::SpeciesId,
    /// The definition of a generated species (P6), held by the person so their body can be read
    /// without the world: set when they are given it, restored from the world on load.
    #[serde(skip)]
    pub kind: Option<&'static crate::species::SpeciesDef>,
    /// How they would have their dead kept (P7): their culture's rite.
    #[serde(default)]
    pub rite: crate::realm::culture::Rite,
    /// The realm culture they were raised in, if any (P7; the portal party's is their own).
    #[serde(default)]
    pub culture: Option<u32>,
    /// The world's stories they know (P8): realm story ids, from their culture or a hearth.
    #[serde(default)]
    pub stories: Vec<u32>,
    /// The tool in hand: its quality (0 = none) and how much of it is left.
    #[serde(default)]
    pub tool_quality: f32,
    #[serde(default)]
    pub tool_wear: f32,
    /// The tool in hand, if it is an item with a story (legend.rs).
    #[serde(default)]
    pub tool: Option<crate::ids::ItemId>,
    /// A spear in hand and armour on (systems/arms.rs, C33), taken up when strangers threaten.
    #[serde(default)]
    pub weapon: Option<crate::ids::ItemId>,
    #[serde(default)]
    pub armour: Option<crate::ids::ItemId>,
    /// Taken by a work (systems/moods.rs, C35), and why.
    #[serde(default)]
    pub fey: Option<crate::systems::moods::Fey>,
    /// What they have heard (or guessed) of how the old valley's hamlets ended (lore.rs).
    #[serde(default)]
    pub tales: Vec<crate::lore::Tale>,
    /// What they saw, were told or read from a grave of the colony's own dead (accounts.rs,
    /// #452).
    #[serde(default)]
    pub accounts: Vec<crate::accounts::Account>,
    /// How many nights in a row they have slept on the ground; a night in a bed resets it
    /// (systems/strain.rs, OST-357).
    #[serde(default)]
    pub rough_nights: u8,
    /// The warning signs they have set out to pray over, newest last, at most
    /// `pilgrimage::PRAYED_KEPT`: one warning sends a person to the shrine once (OST-505).
    #[serde(default)]
    pub prayed_over: Vec<crate::ids::EventId>,
    /// The deaths whose graves they have read, newest last, at most `accounts::GRAVES_KEPT`:
    /// kept apart from `accounts` so forgetting what they believe does not send them back to
    /// read the same stone again (accounts.rs, OST-165).
    #[serde(default)]
    pub graves_read: Vec<crate::ids::EventId>,
    /// A hidden potential carried through the portal, if any (resonance.rs, #308): shown
    /// nowhere until it shows itself.
    #[serde(default)]
    pub resonance: Option<crate::resonance::Resonance>,
    /// Asleep in a den for the winter (creatures.rs): not seen, not moving.
    #[serde(default)]
    pub denned: bool,
    /// A wild animal: the tick it last ate its fill (#486, ecology.rs); 0 until first seen.
    #[serde(default)]
    pub fed: u64,
    /// Clothes wear out over a season and a half; new ones come from the loom.
    #[serde(default = "one")]
    pub clothes_wear: f32,
    /// The clothes they wear are leather or fur (C34): warmer.
    #[serde(default)]
    pub furred: bool,
    /// How soaked they are, 0..1 (OST-903): rain wets those outdoors, a roof or a hearth dries
    /// them (hazards.rs `cold_step`); it also feeds the cold-chill factor there. 0 in
    /// an old save.
    #[serde(default)]
    pub wet: f32,
    #[serde(default)]
    pub thoughts: crate::mind::Thoughts,
    /// 0..1; breaks when it passes the person's threshold (calm genome raises it).
    #[serde(default)]
    pub stress: f32,
    #[serde(default)]
    pub last_break: u64,
    /// Off work until this tick (withdrawn or wandered off).
    #[serde(default)]
    pub withdrawn_until: u64,
    #[serde(default)]
    pub body: crate::body::Body,
    /// Looked after by a healer until this tick (heals faster, infects less).
    #[serde(default)]
    pub tended_until: u64,
    /// Their life before the portal: chronicle events below the epoch (past.rs), oldest first.
    #[serde(default)]
    pub past: Vec<crate::ids::EventId>,
    /// Where they grew up (the village on the other side), empty if born here.
    #[serde(default)]
    pub home: String,
    /// What they hold true, with where it came from (mind/beliefs.rs).
    #[serde(default)]
    pub beliefs: crate::mind::Beliefs,
    /// What they know how to do in a crisis (mind/techniques.rs).
    #[serde(default)]
    pub techniques: crate::mind::Techniques,
    /// Word they have heard of one of the lost: who, roughly where, from whom. Held in their
    /// head, passed on by talk, gone cold after twenty days.
    #[serde(default)]
    pub heard: Vec<crate::opening::Rumour>,
    /// One of a band of outsiders (threat.rs), not of any community here.
    #[serde(default)]
    pub band: Option<u32>,
    /// Another community they still belong to in their heart (ties.rs, #348): one they left
    /// in a split with kin still in it.
    #[serde(default)]
    pub tie: Option<crate::ids::CommunityId>,
    /// C76: a stranger kept by a community after a fight: (by whom, from which place beyond
    /// the valley, since when). Ransomed by their own people or, in time, taken in.
    #[serde(default)]
    pub held: Option<(crate::ids::CommunityId, u32, u64)>,
    /// C77: the deference this person gives others, per domain (provisioning, protection, the
    /// spirit, the social), from deeds they saw them do; it fades.
    #[serde(default)]
    pub defers: Vec<(AgentId, [f32; 4])>,
    /// #347: away on their own journey, still of their community: (back by, where, why).
    #[serde(default)]
    pub journey: Option<(u64, String, String)>,
    /// Sicknesses had and not to be had again (#328).
    #[serde(default)]
    pub immune: Vec<crate::body::IllnessKind>,
    /// Look-alike plants this person knows and leaves (#286, rule 3): from a sickness of their
    /// own, from seeing someone sicken, or from being told.
    #[serde(default)]
    pub shuns: Vec<crate::plants::PlantKind>,
    /// #295: those this person owes a good turn, and since when (tended, taught).
    #[serde(default)]
    pub owes: Vec<(AgentId, u64)>,
    /// #322: who they suspect of letting the deep's people up, and how sure (0 to 1); passed
    /// on by talk (social.rs `tell_suspicion`).
    #[serde(default)]
    pub suspects: Vec<(AgentId, f32)>,
    /// A rival of the valley (nemesis.rs), by their id in `World::rivals`.
    #[serde(default)]
    pub rival: Option<u32>,
    /// What they feel and about what (mind/feelings.rs, docs/design-feelings.md).
    #[serde(default)]
    pub feelings: crate::mind::feelings::Feelings,
    /// What they have seen of another's manifestation and what they make of it
    /// (mind/witnessed.rs, #508 WP-07 7.5); old saves have seen nothing.
    #[serde(default)]
    pub witnessed: Vec<crate::mind::witnessed::Witnessed>,
    /// Pieces of the old world they hold (mind/fragment.rs, #508 WP-07 7.6), woken or
    /// taught, drifted, withheld; old saves hold none.
    #[serde(default)]
    pub fragments: Vec<crate::mind::fragment::HeldFragment>,
    /// Degrees of cold they have grown used to, out in it (thermal.rs, #346): 0 to 4.
    #[serde(default)]
    pub hardened: f32,
    /// One who took them in when both parents were gone (fostering.rs, #341).
    #[serde(default)]
    pub foster: Option<AgentId>,
    /// An animal kept by a community (livestock.rs, #285).
    #[serde(default)]
    pub kept: Option<crate::ids::CommunityId>,
    /// Curses and the like, each with the row that gave it (P11, conditions.rs).
    #[serde(default)]
    pub conditions: Vec<crate::conditions::Condition>,
    /// The tick a beast earned its name (systems/animals.rs); lines about it from before then
    /// call it by its kind, so the chronicle reads as it did when shown.
    #[serde(default)]
    pub named_at: Option<u64>,
    /// Quarrels to judge once they see how the other was, the judgements made, and the fading
    /// lean toward or away from helping each person they judged (#568, mind/conscience.rs).
    #[serde(default)]
    pub conscience: crate::mind::conscience::Conscience,
    /// What they were told, by whom, and whether they took it (#487, mind/telling.rs).
    #[serde(default)]
    pub told: Vec<crate::mind::telling::Told>,
}

fn one() -> f32 {
    1.0
}

pub const ADULT_YEARS: u64 = 15;
pub const ELDER_YEARS: u64 = 50;
/// A child is an infant below this many (human) years, a small child below `SMALL_YEARS`, an
/// older child after (D101). Scaled for other kinds by their maturity.
pub const INFANT_YEARS: u64 = 2;
pub const SMALL_YEARS: u64 = 7;

/// How far a child has come, below grown. Not saved, so not position-bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Childhood {
    Infant,
    Small,
    Older,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LifeStage {
    Child,
    Adult,
    Elder,
}

fn default_pos() -> TilePos {
    TilePos::new(0, 0, 0)
}

impl Agent {
    /// Take up a new activity: the activity, its tick count from zero, and the count of
    /// switches, together. A wild animal's every change goes through here (#388, 2026-10-26
    /// QA round: animals set their activity by hand and never counted a switch, so every
    /// animal read as never having acted, and the burrower, which moves by digging, never
    /// set one at all and read as stuck for the whole run).
    pub fn set_activity(&mut self, act: crate::mind::Activity) {
        self.activity = act;
        self.activity_ticks = 0;
        self.switches += 1;
    }

    /// The species a person is of (animals read as human; they never ask).
    pub fn species_def(&self) -> &'static crate::species::SpeciesDef {
        self.kind
            .unwrap_or_else(|| crate::species::get(self.people))
    }

    /// Grown mass in kg, from the genome within the species' range; an animal's within its
    /// kind's (size.rs, #422).
    pub fn mass_kg(&self) -> f32 {
        match crate::size::animal(self.species) {
            Some(s) => self.genome.mass_within(s.mass_kg) * self.build().factor(),
            None => self.genome.mass_in(self.species_def()),
        }
    }

    /// Grown height in cm, from the genome within the species' range (an animal's at the
    /// shoulder, within its kind's).
    pub fn height_cm(&self) -> f32 {
        match crate::size::animal(self.species) {
            Some(s) => {
                self.genome.height_within(s.height_cm)
                    * crate::fmath::Det::d_powf(self.build().factor(), 1.0 / 3.0)
            }
            None => self.genome.height_in(self.species_def()),
        }
    }

    /// A runt, a giant or neither of its kind (#422); a person is always neither.
    pub fn build(&self) -> crate::size::Build {
        if self.species == Species::Person {
            crate::size::Build::Plain
        } else {
            crate::size::build_of(&self.genome)
        }
    }

    /// The mass the body has now (#422): what it eats, hits and pushes with. A person's is
    /// their grown mass (their balance is as it was); a young animal's is what it has grown to;
    /// a body that is not a skeleton's, what is left of it (#423).
    pub fn body_kg(&self, now: u64) -> f32 {
        let grown = self.mass_kg();
        let kg = match crate::size::animal(self.species) {
            Some(s) => grown * crate::size::growth(self.years_f(now), s.grown_years),
            None => grown,
        };
        // A swarm thinned, a chain shortened or a mass broken weighs as much as is left of it,
        // and hits and pushes with that (#423).
        match self.body.form {
            Some(f) => kg * f.share(),
            None => kg,
        }
    }

    /// Age in years with the fraction of the year.
    pub fn years_f(&self, now: u64) -> f32 {
        let ticks = (now as i64 - self.born).max(0) as f64 * self.age_scale as f64;
        (ticks / crate::time::TICKS_PER_YEAR as f64) as f32
    }

    /// Years this body would live, before hardship.
    pub fn lifespan_years(&self) -> f32 {
        self.genome.lifespan_in(self.species_def())
    }

    /// How fast this person gets hungry against the human mean (genetics.rs).
    pub fn hunger_scale(&self) -> f32 {
        crate::genetics::hunger_scale(self.mass_kg())
    }

    /// How fast this person loses heat against the human mean: body size and coat.
    pub fn heat_loss(&self) -> f32 {
        let coat = match self.species_def().climate.coat.as_str() {
            "fur" => 0.6,
            "feathers" => 0.7,
            "scales" => 1.2,
            _ => 1.0,
        };
        crate::genetics::heat_loss(self.mass_kg()) * coat
    }

    /// Loads this person carries against the human mean.
    pub fn carry(&self) -> f32 {
        crate::genetics::carry(self.mass_kg(), self.species_def().body_plan().hands())
    }

    /// How many of a kind this body carries in one trip (#422): a full stack of it times
    /// `carry`, at least one. A person of the human mean carries a full stack, as before; a
    /// small body less, so it takes more trips.
    pub fn load(&self, kind: crate::items::ItemKind) -> u32 {
        ((kind.max_stack() as f32 * self.carry()).ceil() as u32).clamp(1, kind.max_stack())
    }

    /// Age in years. `age_scale` is 1.0 in play; long-horizon tests compress lifetimes.
    pub fn age_years(&self, now: u64) -> u64 {
        let ticks = (now as i64 - self.born).max(0) as f64 * self.age_scale as f64;
        (ticks / crate::time::TICKS_PER_YEAR as f64) as u64
    }
    /// The years a person grows up and grows old at: a human's 15 and 50, another kind's own
    /// (grown at its maturity, old from the short end of its lifespan; P5).
    pub fn stage_years(&self) -> (u64, u64) {
        if self.people == crate::species::SpeciesId::HUMAN {
            (ADULT_YEARS, ELDER_YEARS)
        } else {
            let sp = self.species_def();
            (sp.life.mature_years as u64, sp.life.lifespan[0] as u64)
        }
    }

    /// Old enough to take up arms, share drink, be taken by a work or propose one: sixteen for
    /// a human (as before P5), grown for another kind.
    pub fn of_age(&self, now: u64) -> bool {
        let (adult, _) = self.stage_years();
        let need = if self.people == crate::species::SpeciesId::HUMAN {
            16
        } else {
            adult
        };
        self.age_years(now) >= need
    }

    /// The years a person can have children: sixteen to forty-five for a human, from grown to
    /// three quarters of the way to old for another kind.
    pub fn fertile_years(&self) -> std::ops::Range<u64> {
        if self.people == crate::species::SpeciesId::HUMAN {
            16..46
        } else {
            let (adult, elder) = self.stage_years();
            adult..adult + (elder.saturating_sub(adult) * 3 / 4).max(1)
        }
    }

    /// The age the ailments of age begin at (a human's fifty), and frailty (a human's
    /// fifty-five): their kind's old age, and a tenth past it.
    pub fn ailing_years(&self) -> (u64, u64) {
        let (_, elder) = self.stage_years();
        (elder, elder + elder / 10)
    }

    /// How far along a child is, or `None` for one not a child. The human stage ages are
    /// scaled by the kind's maturity over a human's `ADULT_YEARS`.
    pub fn childhood(&self, now: u64) -> Option<Childhood> {
        if self.stage(now) != LifeStage::Child {
            return None;
        }
        let scale = self.stage_years().0 as f32 / ADULT_YEARS as f32;
        let y = self.age_years(now) as f32;
        Some(if y < INFANT_YEARS as f32 * scale {
            Childhood::Infant
        } else if y < SMALL_YEARS as f32 * scale {
            Childhood::Small
        } else {
            Childhood::Older
        })
    }

    /// Past the infant stage: a small or older child, or grown.
    pub fn past_infancy(&self, now: u64) -> bool {
        self.childhood(now) != Some(Childhood::Infant)
    }

    /// Grown, or a child at least as far along as `stage`.
    pub fn old_enough_for(&self, stage: Childhood, now: u64) -> bool {
        self.childhood(now).is_none_or(|c| c >= stage)
    }

    pub fn stage(&self, now: u64) -> LifeStage {
        let y = self.age_years(now);
        let (adult, elder) = self.stage_years();
        if y < adult {
            LifeStage::Child
        } else if y < elder {
            LifeStage::Adult
        } else {
            LifeStage::Elder
        }
    }
    pub fn is_child(&self, now: u64) -> bool {
        self.stage(now) == LifeStage::Child
    }

    /// Note something that happened to this person (short-term; consolidated at sleep).
    pub fn remember(
        &mut self,
        tick: u64,
        kind: crate::mind::MemKind,
        subject: crate::mind::Subject,
        valence: f32,
        salience: f32,
        event: Option<crate::ids::EventId>,
    ) {
        self.memories.record(crate::mind::Record {
            tick,
            kind,
            subject,
            valence,
            salience,
            event,
        });
    }

    /// Move a relationship by `delta` opinion and `fam` familiarity (bounded, capped list).
    /// Take in word of one of the lost: a fresher or first-hand account replaces an older one.
    pub fn hear(&mut self, r: crate::opening::Rumour) {
        if let Some(old) = self.heard.iter_mut().find(|x| x.who == r.who) {
            if r.heard >= old.heard {
                *old = r;
            }
        } else if self.heard.len() < 10 {
            self.heard.push(r);
        }
    }

    pub fn relate(&mut self, other: AgentId, delta: f32, fam: f32, tick: u64) {
        if !self.relationships.contains_key(&other)
            && self.relationships.len() >= crate::mind::memory::RELATION_CAP
        {
            // Forget the least familiar acquaintance.
            if let Some((&k, _)) = self.relationships.iter().min_by(|a, b| {
                a.1.familiarity
                    .partial_cmp(&b.1.familiarity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                self.relationships.remove(&k);
            }
        }
        let r = self.relationships.entry(other).or_default();
        r.opinion = (r.opinion + delta).clamp(-1.0, 1.0);
        r.familiarity = (r.familiarity + fam).min(1.0);
        r.last_contact = tick;
    }

    /// Stamp real, in-person contact with `other` at `tick`. Call this beside `relate` only at
    /// direct-contact sites (talk, quarrel, hearth tale, drink, pairing, ring, making up,
    /// fostering, working side by side, childhood play) -- never for hearsay such as gossip,
    /// suspicion told or feud sides, which only move `last_contact` (#1131, 2026-11-15
    /// fix-1131-met-stamp-real-contact).
    pub fn met(&mut self, other: AgentId, tick: u64) {
        if !self.relationships.contains_key(&other)
            && self.relationships.len() >= crate::mind::memory::RELATION_CAP
        {
            if let Some((&k, _)) = self.relationships.iter().min_by(|a, b| {
                a.1.familiarity
                    .partial_cmp(&b.1.familiarity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                self.relationships.remove(&k);
            }
        }
        let r = self.relationships.entry(other).or_default();
        r.met = tick;
    }

    pub fn opinion_of(&self, other: AgentId) -> f32 {
        self.relationships
            .get(&other)
            .map(|r| r.opinion)
            .unwrap_or(0.0)
    }

    pub fn injured(&self, now: u64) -> bool {
        self.injured_until > now || self.body.hurt()
    }
    pub fn blood_up(&mut self, by: f32) {
        self.body.blood = (self.body.blood + by).min(1.0);
    }

    pub fn is_moving(&self) -> bool {
        self.move_total > 0 && self.move_ticks < self.move_total
    }
    /// 0..1 progress from `from` to `pos`.
    pub fn move_progress(&self) -> f32 {
        if self.move_total == 0 {
            1.0
        } else {
            (self.move_ticks as f32 / self.move_total as f32).min(1.0)
        }
    }
    /// Facing from a step direction (0 S, 1 W, 2 E, 3 N).
    pub fn facing_toward(from: TilePos, to: TilePos) -> u8 {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        if dy.abs() >= dx.abs() {
            if dy > 0 {
                0
            } else if dy < 0 {
                3
            } else {
                0
            }
        } else if dx < 0 {
            1
        } else {
            2
        }
    }
}
