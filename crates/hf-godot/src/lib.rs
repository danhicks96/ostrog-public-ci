//! Godot bridge: a thin GDExtension exposing the simulation to GDScript. No game logic here.

use godot::classes::Node;
use godot::prelude::*;
use hf_sim::chronicle::Chronicle;
use hf_sim::events::Severity;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::view::director::Director;
use hf_sim::view::{experience, narrate, summary};
use hf_sim::world::gen::biome::Character;
use hf_sim::world::gen::GenParams;
use hf_sim::{Session, World};

mod sim_thread;
mod world_renderer;

struct OstrogExtension;

/// Where the settlement is: the centroid of built structures, else the founding site.
use hf_sim::view::director::settlement_centre;

const POSE_TALK: i32 = 10;
const POSE_EAT: i32 = 11;
/// #485: drinking (Activity::Drinking only): bent to the water, cupped hands to the mouth.
const POSE_DRINK: i32 = 12;
/// #485: the work-sound code of a Tend order; the client draws the tending cycle for it. It has
/// no sound or work accent of its own (sound.gd and effects.gd take codes 1 to 7).
const WORK_TEND: i32 = 8;

fn rendered_shape(w: &World, a: &hf_sim::agents::Agent) -> hf_sim::agents::Species {
    // C75: Beast-hunger changes the rendered creature, not just the person's body sheet.
    if w.creatures.shifted.iter().any(|x| x.0 == a.id) {
        hf_sim::agents::Species::Wolf
    } else {
        a.species
    }
}

fn person_pose(w: &World, a: &hf_sim::agents::Agent, carrying: bool, body: Option<i32>) -> i32 {
    // FIXLOG 2026-10-31, #485 first slice: do not let a recent wound or a new social/eating
    // pose erase movement, combat or the dead-body presentation selected by the existing rules.
    let fresh_wound = a
        .body
        .wounds
        .iter()
        .any(|wd| w.meta.tick.saturating_sub(wd.since) < 40);
    if let Some(p) = body {
        p
    } else if a.fought_at > 0 && w.meta.tick.saturating_sub(a.fought_at) < 60 {
        6
    } else if fresh_wound {
        7
    } else if a.is_moving() {
        if carrying {
            3
        } else {
            1
        }
    } else {
        match &a.activity {
            hf_sim::mind::Activity::Sleeping { .. } => 4,
            hf_sim::mind::Activity::Foraging { .. } => 2,
            hf_sim::mind::Activity::Eating { .. }
                if rendered_shape(w, a) == hf_sim::agents::Species::Person =>
            {
                POSE_EAT
            }
            // Rendered animals, including beast-shaped people, keep the established eating bob.
            hf_sim::mind::Activity::Eating { .. } => 5,
            hf_sim::mind::Activity::Talking { .. } => POSE_TALK,
            hf_sim::mind::Activity::Drinking { .. }
                if rendered_shape(w, a) == hf_sim::agents::Species::Person =>
            {
                POSE_DRINK
            }
            hf_sim::mind::Activity::Working { plan } => match plan.current() {
                Some(hf_sim::work::Step::Work { .. }) => 2,
                _ => {
                    if carrying {
                        3
                    } else {
                        0
                    }
                }
            },
            _ => 0,
        }
    }
}

/// Relationship lines for the client overlay (#192, WP-08 8.4): 3 ints each —
/// from_row, to_row, band (1 friend, 2 likes, 3 knows, 4 dislikes; the same
/// thresholds as the inspect panel's "people:" line). Rows are positions in
/// `agents()`/`agent_row` output: only visible people get rows (the same rule as
/// the `agents()` func — dead-until-found, denned, and unfound people have no
/// entity to line from or to), and each pair is emitted once, as (lower row,
/// higher row) in row order. The lower row's view gives the colour (noted in the
/// overlay's legend); a pair that only the higher row holds is still drawn, with
/// that person's view, because relationships are per person and can be one-sided.
pub(crate) fn relationship_lines_core(
    w: &World,
    bodies: &hf_sim::view::bodies::Bodies,
) -> Vec<i32> {
    let mut out = Vec::new();
    let mut row_of = std::collections::BTreeMap::new();
    let mut row = 0i32;
    for a in &w.agents {
        let body = if a.alive || a.species != hf_sim::agents::Species::Person {
            None
        } else {
            bodies.pose(a.id, w.meta.tick)
        };
        if (!a.alive && body.is_none()) || a.denned || hf_sim::view::known::is_unfound(w, a.id) {
            continue;
        }
        row_of.insert(a.id, row);
        row += 1;
    }
    // Each pair once, keyed (lower row, higher row). The lower row's view is the line's colour
    // (D88); a one-sided relationship is drawn from whichever side holds it (review of #723: the
    // loop once read only the lower row's entries, so a pair known only to the higher row was
    // never drawn).
    let mut pairs: std::collections::BTreeMap<(i32, i32), i32> = std::collections::BTreeMap::new();
    for a in &w.agents {
        let Some(&from) = row_of.get(&a.id) else {
            continue;
        };
        for (id, r) in a.relationships.iter() {
            let Some(&to) = row_of.get(id) else {
                continue;
            };
            if from == to {
                continue;
            }
            let band = if r.opinion > 0.4 {
                1
            } else if r.opinion > 0.1 {
                2
            } else if r.opinion < -0.3 {
                4
            } else {
                3
            };
            let key = (from.min(to), from.max(to));
            if from < to {
                pairs.insert(key, band);
            } else {
                pairs.entry(key).or_insert(band);
            }
        }
    }
    for ((lo, hi), band) in pairs {
        out.push(lo);
        out.push(hi);
        out.push(band);
    }
    out
}

pub(crate) fn agent_row(w: &World, a: &hf_sim::agents::Agent, body: Option<i32>) -> [i32; 25] {
    let lk = hf_sim::view::look::look(w, a);
    let carrying = w
        .items
        .iter()
        .any(|it| it.place == hf_sim::items::ItemPlace::Carried(a.id) && !it.kind.is_food());
    let pose = person_pose(w, a, carrying, body);
    // What the work looks and sounds like, for the client's positional effects and the
    // activity emote (#192, WP-08 8.4): 0 nothing, 1 chop, 2 mine, 3 hammer (building),
    // 4 hoe (field work), 5 saw (workshop), 6 cook, 7 water (dousing); the emote is an
    // ItemKind index + 1 (the client's ITEM_NAME order) or 0 for none. The sound stays
    // gated on the working pose (footfall, not work, while walking there); the emote keys
    // on the step itself, so a seated craft or a study shows its tool too.
    let (sound, emote) = match &a.activity {
        hf_sim::mind::Activity::Working { plan } => {
            let at_work = match plan.current() {
                Some(hf_sim::work::Step::Work { designation, .. }) => w
                    .designations
                    .iter()
                    .find(|d| d.id == *designation)
                    .map(|d| {
                        use hf_sim::items::ItemKind as I;
                        use hf_sim::work::DesignationKind as K;
                        match d.kind {
                            K::Chop => (1, I::Tool as i32 + 1),
                            K::Mine | K::DigStairs | K::DigEarth | K::DigChannel => {
                                (2, I::SledgeHammer as i32 + 1)
                            }
                            K::BuildWall(_)
                            | K::BuildDoor
                            | K::BuildStairs(_)
                            | K::BuildFloor(_)
                            | K::BuildRoof(_)
                            | K::BuildStructure(..) => (3, I::Hammer as i32 + 1),
                            K::Sow | K::Harvest => (4, I::Tool as i32 + 1),
                            K::Craft(_) => (5, I::Saw as i32 + 1),
                            K::Cook => (6, I::Vessel as i32 + 1),
                            K::Douse => (7, I::Bucket as i32 + 1),
                            K::Tend(_) => (WORK_TEND, I::Poultice as i32 + 1),
                            K::BeatOut => (4, I::Bucket as i32 + 1),
                            K::Hunt(_) => (0, I::Weapon as i32 + 1),
                            K::Study(_) => (0, I::Record as i32 + 1),
                            _ => (0, 0),
                        }
                    })
                    .unwrap_or((0, 0)),
                _ => (0, 0),
            };
            if pose == 2 {
                at_work
            } else {
                (0, at_work.1)
            }
        }
        _ => (0, 0),
    };
    // One of a band of strangers (threat.rs): 1, or 2 while out after food.
    let band = match hf_sim::threat::band_of(w, a.id) {
        Some(b) if b.raiders.iter().any(|r| r.0 == a.id) => 2,
        Some(_) => 1,
        None => 0,
    };
    // A body lies still where it fell.
    let (from, step) = if body.is_some() {
        (a.pos, 1000)
    } else {
        (a.from, (a.move_progress() * 1000.0) as i32)
    };
    [
        a.id.0 as i32,
        a.pos.x,
        a.pos.y,
        a.pos.z,
        rendered_shape(w, a) as i32,
        a.facing as i32,
        lk.body as i32,
        lk.skin as i32,
        lk.hair as i32,
        lk.hair_colour as i32,
        lk.clothing as i32,
        lk.clothing_colour as i32,
        from.x,
        from.y,
        step,
        pose,
        lk.child as i32,
        lk.injured as i32,
        lk.elder as i32,
        lk.equipment as i32,
        sound,
        band,
        a.people.0 as i32,
        from.z,
        emote,
    ]
}

#[gdextension]
unsafe impl ExtensionLibrary for OstrogExtension {
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::MainLoop {
            clear_finished_update();
        }
    }
}

/// In a portable install (`launcher.toml` beside `app\`), the running game removes what the
/// launcher's last update left in `.ostrog-update`: once this build has started, its backup is
/// no longer needed for rollback. Friends' launchers are never replaced by an update and the
/// 0.1.0 launcher never removed it, so its next update failed with `BackupAlreadyExists` (WP-02
/// 2.9); the app is the part an update replaces, so the cleanup rides here too.
fn clear_finished_update() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(app_dir) = exe.parent() else {
        return;
    };
    let portable = app_dir
        .parent()
        .is_some_and(|install| install.join("launcher.toml").is_file());
    if !portable {
        return;
    }
    match hf_update::clear_finished_update(app_dir) {
        Ok(true) => godot_print!("cleared the last update's leftovers"),
        Ok(false) => {}
        Err(error) => godot_warn!("could not clear the last update's leftovers: {error}"),
    }
}

/// Owns a world + chronicle and steps it. Exposes read models for the client.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SimHost {
    base: Base<Node>,
    session: Option<Session>,
    director: Director,
    /// Feed lines accumulated since the client last pulled them: (tick, x, y, z, severity, text, kind, fav, because).
    feed: sim_thread::Feed,
    /// Events the client plays a sound for, since it last pulled them: (sound name, where).
    sounds: Vec<(&'static str, TilePos)>,
    /// The dead still lying where they fell (#185), from the deaths the viewer was told.
    bodies: hf_sim::view::bodies::Bodies,
    /// The newest event id already passed to the feed and director (the arrival's are passed
    /// on attach, before the first tick flushes them).
    published: Option<u32>,
    last_step_ms: f64,
    save_thread: Option<std::thread::JoinHandle<Result<(), String>>>,
    last_save_prepare_ms: f64,
    last_save_result: String,
    /// The world's region map for the Legends browser, by seed (regenerated in milliseconds,
    /// never saved).
    realm_map: Option<(u64, hf_sim::realm::RealmMap)>,
    /// The wizard's world map and valley previews (#560).
    previews: PreviewJobs,
    /// The viewer's favourited people for this save (#413), mirrored in `chronicle.meta`.
    favourites: std::collections::BTreeSet<hf_sim::ids::AgentId>,
    /// The sim's step thread (#603): while it has the session, `session` is None and the view
    /// reads `snapshot`. None when stepping in place (`set_threaded(false)`, HF_SIM_THREAD=0).
    stepper: Option<sim_thread::SimThread>,
    /// The world as the last batch of ticks left it, for the view while the next batch runs.
    snapshot: Option<std::sync::Arc<sim_thread::Snapshot>>,
    /// Ticks asked for and not yet handed to the step thread.
    pending: u64,
    /// The director's last suggestion: made at home, or by the step thread after its last batch
    /// while the client asks (FIXLOG 2026-11-14, #797 finding 2).
    suggestion: Option<hf_sim::view::director::Suggestion>,
    /// When the client last asked for a suggestion: the step thread works one out only while
    /// it is being asked for, so the director's dwell and memory of shown places do not run on
    /// while the player has the camera.
    suggest_asked: Option<std::time::Instant>,
    /// Each inspected person's life before the portal, as read from the chronicle (#603).
    past_cache: std::collections::BTreeMap<i32, Vec<String>>,
}

#[godot_api]
impl INode for SimHost {
    fn init(base: Base<Node>) -> Self {
        SimHost {
            base,
            session: None,
            director: Director::default(),
            feed: Vec::new(),
            sounds: Vec::new(),
            bodies: Default::default(),
            published: None,
            last_step_ms: 0.0,
            save_thread: None,
            last_save_prepare_ms: 0.0,
            last_save_result: String::new(),
            realm_map: None,
            previews: PreviewJobs::default(),
            favourites: std::collections::BTreeSet::new(),
            stepper: (std::env::var("HF_SIM_THREAD").as_deref() != Ok("0"))
                .then(sim_thread::SimThread::spawn),
            snapshot: None,
            pending: 0,
            suggestion: None,
            suggest_asked: None,
            past_cache: Default::default(),
        }
    }
}

/// A valley preview asked for: seed, region (-1: the seed's) and the character's bits, if one
/// was given.
type PreviewKey = (u64, i64, Option<[u32; 3]>);
type PreviewSlots = std::sync::Arc<
    std::sync::Mutex<std::collections::BTreeMap<PreviewKey, Option<hf_sim::setup::Preview>>>,
>;

/// The wizard's valley previews (#560). A preview generates a whole valley (about 50 ms in a
/// release build, several times that in a debug one), so each is made on its own thread and
/// the wizard asks again each frame until it is there: a click on the world map never waits
/// for one. The world map of the seed is kept, so a click does not make it again either.
#[derive(Default)]
struct PreviewJobs {
    map: Option<(u64, std::sync::Arc<hf_sim::realm::RealmMap>)>,
    /// None while a preview is being made, then the preview until it is taken.
    slots: PreviewSlots,
    /// How many previews were started and the thread the last was made on, so a test can see
    /// that none is made on the calling thread (#582).
    made_on: std::sync::Arc<std::sync::Mutex<(usize, Option<std::thread::ThreadId>)>>,
}

impl PreviewJobs {
    /// Most finished previews kept that nobody has taken (the wizard takes only the one it
    /// is showing; the rest it no longer wants).
    const KEEP: usize = 16;

    fn map(&mut self, seed: u64) -> std::sync::Arc<hf_sim::realm::RealmMap> {
        match &self.map {
            Some((s, m)) if *s == seed => m.clone(),
            _ => {
                let m = std::sync::Arc::new(hf_sim::realm::generate(
                    seed,
                    &hf_sim::realm::RealmParams::default(),
                ));
                self.map = Some((seed, m.clone()));
                m
            }
        }
    }

    fn key(seed: u64, ch: Option<Character>, region: i64) -> PreviewKey {
        (
            seed,
            region.max(-1),
            ch.map(|c| {
                [
                    c.wetness.to_bits(),
                    c.warmth.to_bits(),
                    c.stoniness.to_bits(),
                ]
            }),
        )
    }

    /// Start making a preview unless it is made or being made. True when it is ready.
    fn begin(&mut self, seed: u64, ch: Option<Character>, region: i64) -> bool {
        let key = Self::key(seed, ch, region);
        let m = self.map(seed);
        let mut slots = self.slots.lock().unwrap();
        if let Some(slot) = slots.get(&key) {
            return slot.is_some();
        }
        if slots.len() >= Self::KEEP {
            slots.retain(|_, v| v.is_none());
        }
        slots.insert(key, None);
        drop(slots);
        let out = self.slots.clone();
        let made_on = self.made_on.clone();
        std::thread::spawn(move || {
            {
                let mut m = made_on.lock().unwrap();
                m.0 += 1;
                m.1 = Some(std::thread::current().id());
            }
            let region = (region >= 0).then_some(region as usize);
            let p = hf_sim::setup::preview_in(seed, &m, ch, region);
            if let Some(slot) = out.lock().unwrap().get_mut(&key) {
                *slot = Some(p);
            }
        });
        false
    }

    /// The preview, once made; it is handed over once.
    fn take(
        &mut self,
        seed: u64,
        ch: Option<Character>,
        region: i64,
    ) -> Option<hf_sim::setup::Preview> {
        let key = Self::key(seed, ch, region);
        let mut slots = self.slots.lock().unwrap();
        if matches!(slots.get(&key), Some(Some(_))) {
            slots.remove(&key).flatten()
        } else {
            None
        }
    }
}

/// [wetness, warmth, stoniness] each -1..1 from the wizard, or None when it sent none.
fn character_arg(character: &PackedFloat32Array) -> Option<Character> {
    (character.len() == 3).then(|| Character {
        wetness: character[0].clamp(-1.0, 1.0),
        warmth: character[1].clamp(-1.0, 1.0),
        stoniness: character[2].clamp(-1.0, 1.0),
    })
}

fn preview_dict(p: &hf_sim::setup::Preview) -> VarDictionary {
    let mut v = VarDictionary::new();
    let lines: PackedStringArray = p.lines.iter().map(|l| GString::from(l.as_str())).collect();
    v.set("lines", &lines);
    v.set("width", p.width);
    v.set("height", p.height);
    v.set("columns", &PackedByteArray::from(p.columns.as_slice()));
    let tiles: Vec<i32> = p.tiles.iter().map(|&t| t as i32).collect();
    v.set("tiles", &PackedInt32Array::from(tiles.as_slice()));
    v.set("heights", &PackedInt32Array::from(p.heights.as_slice()));
    v.set("plants", &PackedByteArray::from(p.plants.as_slice()));
    v.set(
        "founding",
        Vector3i::new(p.founding.x, p.founding.y, p.founding.z),
    );
    v.set("region", p.region as i64);
    let c = p.character;
    v.set("label", c.label().as_str());
    v.set(
        "character",
        &PackedFloat32Array::from([c.wetness, c.warmth, c.stoniness].as_slice()),
    );
    v
}

impl SimHost {
    fn carried_rows(&self, with_count: bool) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for it in &w.items {
                if let hf_sim::items::ItemPlace::Carried(id) = it.place {
                    out.push(id.0 as i32);
                    out.push(it.kind as i32);
                    out.push(it.material.0 as i32);
                    if with_count {
                        out.push(it.count.min(i32::MAX as u32) as i32);
                    }
                }
            }
        }
        out
    }

    /// The world the view reads: the session's, or while the session is on the step thread,
    /// the snapshot of it as the last batch of ticks left it (#603).
    fn world(&self) -> Option<&World> {
        self.session
            .as_ref()
            .map(|s| &s.world)
            .or_else(|| self.snapshot.as_ref().map(|s| &s.world))
    }

    /// The dead lying where they fell, as the world `world()` gives has them.
    fn bodies_view(&self) -> &hf_sim::view::bodies::Bodies {
        match (&self.session, &self.snapshot) {
            (None, Some(s)) => &s.bodies,
            _ => &self.bodies,
        }
    }

    /// Bring the session back from the step thread, waiting for the batch in hand if there is
    /// one. Everything that reads the chronicle or changes the world calls this first.
    fn settle(&mut self) {
        let done = self.stepper.as_mut().and_then(|t| t.wait());
        if let Some(d) = done {
            self.restore(d);
        }
    }

    /// Take back a finished batch, if there is one, without waiting.
    fn poll(&mut self) {
        let done = self.stepper.as_mut().and_then(|t| t.try_take());
        if let Some(d) = done {
            self.restore(d);
        }
    }

    /// A new world: no ticks waiting for the old one, and no snapshot of it (#603).
    fn forget_steps(&mut self) {
        self.pending = 0;
        self.suggestion = None;
        self.past_cache.clear();
        if let Some(old) = self.snapshot.take() {
            if let Some(t) = self.stepper.as_ref() {
                t.discard(old);
            }
        }
    }

    fn restore(&mut self, d: sim_thread::Done) {
        let c = *d.core;
        self.session = Some(c.session);
        // What this batch made joins what the client has not yet drained (FIXLOG 2026-11-14).
        self.feed.extend(c.feed);
        if self.feed.len() > 400 {
            let cut = self.feed.len() - 400;
            self.feed.drain(..cut);
        }
        self.sounds.extend(c.sounds);
        self.bodies = c.bodies;
        self.director = c.director;
        self.published = c.published;
        self.suggestion = c.suggestion;
        self.last_step_ms = d.step_ms;
        if let Some(old) = self.snapshot.replace(d.snapshot) {
            if let Some(t) = self.stepper.as_ref() {
                t.discard(old);
            }
        }
    }

    /// Whether the client asked for a director suggestion in the last few seconds (it asks
    /// every 2 s while the director has the camera).
    fn suggestion_wanted(&self) -> bool {
        self.suggest_asked
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(5))
    }

    /// Hand the ticks asked for to the step thread, at most `BATCH` at a time so the view's
    /// snapshot keeps up, when it has nothing in hand.
    fn dispatch(&mut self) {
        const BATCH: u64 = 20;
        let want_suggestion = self.suggestion_wanted();
        let Some(t) = self.stepper.as_mut() else {
            return;
        };
        if t.in_flight || self.pending == 0 || self.session.is_none() {
            return;
        }
        let n = self.pending.min(BATCH);
        self.pending -= n;
        // The first batch of a world: the view needs something to read while it runs.
        if self.snapshot.is_none() {
            let w = &self.session.as_ref().expect("checked").world;
            self.snapshot = Some(std::sync::Arc::new(sim_thread::Snapshot {
                world: w.clone(),
                bodies: self.bodies.clone(),
            }));
        }
        let core = Box::new(sim_thread::Core {
            session: self.session.take().expect("checked"),
            // The feed and sounds stay here for the client to drain while the batch runs; the
            // batch makes its own and `restore` adds them. Moving them out with the batch meant
            // `step()`, which hands the next batch over the moment one comes home, kept them away
            // from `drain_feed` and `drain_sounds` in a running game (FIXLOG 2026-11-14).
            feed: Vec::new(),
            sounds: Vec::new(),
            bodies: std::mem::take(&mut self.bodies),
            director: std::mem::take(&mut self.director),
            published: self.published,
            favourites: self.favourites.clone(),
            // A copy: the host keeps answering with it while this batch runs (step() hands the
            // next batch over the moment the last one comes home).
            suggestion: self.suggestion.clone(),
            want_suggestion,
        });
        t.send(core, n);
    }

    /// A compact item read model for the inspect panel. Keep the item itself authoritative in
    /// the simulation; this only turns its identity and story into player-facing text.
    fn inspect_item_line(w: &World, it: &hf_sim::items::Item, owner: AgentId) -> String {
        let mut line = if let Some(story) = hf_sim::legend::describe(w, it.id) {
            story
        } else {
            let name = hf_sim::items::stack_name(&w.grid.materials, it.kind, it.material);
            if it.count == 1 {
                name
            } else {
                format!("{} {}", it.count, name)
            }
        };
        if let Some(story) = it.story.as_ref() {
            let custody: Vec<String> = story
                .owners
                .iter()
                .map(|(_, name, since)| format!("{} ({})", name, hf_sim::legend::when(w, *since)))
                .collect();
            if !custody.is_empty() {
                line.push_str(&format!(" | custody: {}", custody.join(" -> ")));
            }
            if story.owners.last().is_some_and(|(id, _, _)| *id == owner) {
                line.push_str(" | with them now");
            }
            if story.fights > 0 || story.hunts > 0 {
                line.push_str(&format!(
                    " | fights: {}, hunts: {}",
                    story.fights, story.hunts
                ));
            }
        } else if it.quality > 0.0 {
            line.push_str(&format!(
                " | {}",
                hf_sim::items::ItemKind::tier_name(hf_sim::items::ItemKind::tier(it.quality))
            ));
        }
        line
    }

    /// A new world: what is already waiting to be told (the valley's past, the arrival) goes
    /// into the chronicle and the feed now, so a world shown before its first tick is whole.
    fn attach(&mut self, world: World) {
        self.settle();
        self.forget_steps();
        let chron = Chronicle::working()
            .or_else(|_| Chronicle::in_memory())
            .expect("chronicle");
        self.favourites = chron.favourites();
        let mut s = Session::new(world, chron);
        let early = s.write_pending().unwrap_or_else(|e| {
            godot_error!("chronicle write failed: {e}");
            Vec::new()
        });
        self.feed.clear();
        self.sounds.clear();
        self.bodies.clear();
        self.director = Director::default();
        self.published = None;
        let tick = s.world.tick();
        publish(
            &s.world,
            &s.chronicle,
            tick,
            &early,
            &self.favourites,
            &mut self.feed,
            &mut self.director,
            Seen {
                sounds: &mut self.sounds,
                bodies: &mut self.bodies,
            },
            &mut self.published,
        );
        self.session = Some(s);
    }
}

/// What the client is shown of events beyond the feed: the sounds to play and the dead lying
/// where they fell (#185).
struct Seen<'a> {
    sounds: &'a mut Vec<(&'static str, TilePos)>,
    bodies: &'a mut hf_sim::view::bodies::Bodies,
}

/// Pass events on to the feed, the director and the sounds, each event once. Lives before the
/// arrival are history, not news: they go to the chronicle and the inspect panel, never to the
/// feed or the director. Nor does anything about the lost nobody has found yet (view::known).
/// The simulation-backed causal note for a feed event (#413 A5): the recorded cause or origin
/// event in the chronicle, a contextual event detail, or the primary participant's mood
/// `why`/`feeling` on notable events whose headline does not already state a `because`.
fn event_because(
    w: &World,
    chronicle: &Chronicle,
    e: &hf_sim::events::Event,
    line: &str,
) -> String {
    use hf_sim::events::EventKind as K;
    // The recorded cause (the list lives in view::causes, shared with the "why?" chain).
    let cause_id = hf_sim::view::causes::cause_of(&e.kind);
    if let Some(cid) = cause_id {
        if let Ok(Some((hid, ctick, detail))) = chronicle.row(cid) {
            if let Some(cev) = hf_sim::view::history::event_from_row(hid, ctick, &detail) {
                return narrate::line(w, &cev);
            }
        }
    }
    let detail = narrate::contextual_detail(w, e);
    // A hidden recorded explanation is not permission to substitute the actor's current mood.
    if detail.is_some()
        || matches!(
            &e.kind,
            K::Broke {
                explanation: Some(_),
                ..
            }
        )
    {
        return detail.unwrap_or_default();
    }
    // The participant's mood is the cause only of what mood drives: a mind giving way, a quarrel,
    // a fight, leaving. On anything else ("finished the hearth", "tended her hurts") it read as a
    // cause it is not, on nearly every line (FIXLOG 2026-11-14); the inspect panel still shows it.
    if !line.contains("because") && e.severity >= Severity::Notable && mood_drives(e) {
        if let Some(&aid) = e.participants().first() {
            if let Some(a) = w.agents.iter().find(|a| a.id == aid) {
                let why = hf_sim::view::why::because(w, a);
                if !why.is_empty() {
                    return why;
                }
                let feel = hf_sim::view::why::feeling(w, a);
                if !feel.is_empty() {
                    return format!("feels {feel}");
                }
            }
        }
    }
    String::new()
}

/// Events a person's mood brings about, whose because-line may be that mood (`event_because`).
fn mood_drives(e: &hf_sim::events::Event) -> bool {
    matches!(
        e.kind_name(),
        "broke" | "quarrelled" | "fought" | "would_not_stay" | "left_the_valley"
    )
}

/// Each community's people here and away, for the HUD. "Away" is everyone of it in another
/// place, as the People panel counts them: on a journey of their own, with a caravan, or carried
/// off (`overworld.away`). Counting only caravan members said "(3 on a journey)" twice while the
/// People panel said "8 away" (FIXLOG 2026-11-14).
fn communities_line(w: &World) -> String {
    let parts: Vec<String> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| {
            let here = c
                .members
                .iter()
                .filter(|m| w.agents.iter().any(|a| a.id == **m && a.alive))
                .count();
            let away = w
                .overworld
                .away
                .iter()
                .filter(|a| a.community == Some(c.id))
                .count();
            let mut line = if away > 0 {
                format!("{} {} ({} away)", c.name, here, away)
            } else {
                format!("{} {}", c.name, here)
            };
            if let Some(word) = hf_sim::society::cohesion_word(c.cohesion) {
                line.push_str(", ");
                line.push_str(word);
            }
            line
        })
        .collect();
    parts.join(" · ")
}

/// A wild animal's death: a creature that is not a person and in nobody's herd. The chronicle,
/// the feed, the bodies and witnesses' accounts keep it; the camera director is not sent to it.
/// Every death is a Major event (life::die), so a few bats lost to a slime pit in an unexplored
/// cave sixteen levels down outscored the whole settlement at work, and a new valley's camera
/// opened on an empty cave (FIXLOG 2026-11-14, a follow-up to #797 finding 2).
fn wild_death(w: &World, e: &hf_sim::events::Event) -> bool {
    let hf_sim::events::EventKind::Died { agent, .. } = &e.kind else {
        return false;
    };
    w.agents
        .iter()
        .find(|a| a.id == *agent)
        .is_some_and(|a| a.species != hf_sim::agents::Species::Person && a.kept.is_none())
}

/// A feed row's fold key (#1483): `view::fold::key`, or "" for a row that is always its own news.
fn feed_key(e: &hf_sim::events::Event) -> String {
    hf_sim::view::fold::key(e).unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn publish(
    w: &World,
    chronicle: &Chronicle,
    tick: u64,
    events: &[hf_sim::events::Event],
    favourites: &std::collections::BTreeSet<hf_sim::ids::AgentId>,
    feed: &mut sim_thread::Feed,
    director: &mut Director,
    seen: Seen,
    published: &mut Option<u32>,
) {
    // Event ids only grow, so what was passed on already is a prefix.
    let k = events.partition_point(|e| published.is_some_and(|p| e.id.0 <= p));
    let events = &events[k..];
    if let Some(last) = events.last() {
        *published = Some(last.id.0);
    }
    let epoch = w.meta.epoch;
    let told = |e: &hf_sim::events::Event| e.tick >= epoch && hf_sim::view::known::told(w, e);
    let for_director = |e: &hf_sim::events::Event| told(e) && !wild_death(w, e);
    if events.iter().all(for_director) {
        director.observe(tick, events);
    } else {
        let now: Vec<_> = events.iter().filter(|e| for_director(e)).cloned().collect();
        director.observe(tick, &now);
    }
    for e in events {
        if e.severity >= Severity::Minor && told(e) {
            let fav = e.participants().iter().any(|a| favourites.contains(a));
            let line = narrate::line(w, e);
            let because = event_because(w, chronicle, e, &line);
            feed.push((
                e.tick,
                e.at.unwrap_or(TilePos::new(-1, -1, 0)),
                e.severity as i32,
                line,
                e.kind_name(),
                fav,
                because,
                feed_key(e),
            ));
        }
    }
    if feed.len() > 400 {
        let cut = feed.len() - 400;
        feed.drain(..cut);
    }
    for e in events {
        if !told(e) {
            continue;
        }
        seen.bodies.observe(std::slice::from_ref(e));
        if let (Some(name), Some(at)) = (hf_sim::view::sound_of(&e.kind), e.at) {
            seen.sounds.push((name, at));
        }
        // The music's answer to a death or a welcome (#290, WP-08 8.8): carried on the sounds
        // row with no place, since the score is heard across the valley; sound.gd hands it
        // to the music director instead of playing an effect.
        if let Some(m) = hf_sim::view::music::moment_of(w, e) {
            seen.sounds.push((m, TilePos::new(-1, -1, 0)));
        }
    }
    seen.bodies.prune(tick);
    let sounds = seen.sounds;
    // Only the latest matter: a long fast-forward should not replay a backlog.
    if sounds.len() > 64 {
        let cut = sounds.len() - 64;
        sounds.drain(..cut);
    }
}

#[godot_api]
impl SimHost {
    #[func]
    fn hello(&self) -> GString {
        GString::from(format!("ostrog sim {}", env!("CARGO_PKG_VERSION")).as_str())
    }

    #[func]
    fn generate(&mut self, seed: i64, small: bool) {
        self.settle();
        let p = if small {
            GenParams::small()
        } else {
            GenParams::default()
        };
        let mut w = World::generate(seed as u64, p);
        // Twelve founders: enough hands that a party of three can leave on a journey.
        w.found_party(12);
        self.attach(w);
    }

    /// Staged representative scene (art review only; DECISIONS D13).
    #[func]
    fn stage_demo(&mut self, seed: i64) {
        self.settle();
        self.attach(World::stage_demo(seed as u64));
    }

    /// The demo scene with one grown person of every kind in the library standing in a row by the
    /// landing (C53: the line-up in the game itself, layers and all; art review only, D13).
    #[func]
    fn stage_peoples(&mut self, seed: i64) {
        self.settle();
        let mut w = World::stage_demo(seed as u64);
        let fs = w.meta.founding_site;
        let kinds = hf_sim::species::library().len();
        for k in 0..kinds {
            let p = w.grid.surface(fs.x - 6 + k as i32, fs.y + 4);
            let id = w.spawn_agent(hf_sim::agents::Species::Person, p);
            hf_sim::species::give(&mut w, id, hf_sim::species::SpeciesId(k as u16), None);
        }
        self.attach(w);
    }

    /// Art review only (C56): a short wall of each wood and each rock that cuts to blocks, in a
    /// row below the landing, recorded as built of that material.
    #[func]
    fn stage_walls(&mut self, seed: i64) {
        self.settle();
        use hf_sim::world::{Material, Tile};
        let mut w = World::stage_demo(seed as u64);
        let fs = w.meta.founding_site;
        let mats = [
            Material::OAK,
            Material::BIRCH,
            Material::PINE,
            Material::FUNGUS_WOOD,
            Material::GRANITE,
            Material::BASALT,
            Material::SLATE,
            Material::MARBLE,
            Material::LIMESTONE,
            Material::SANDSTONE,
        ];
        for (k, m) in mats.iter().enumerate() {
            let x = fs.x - 10 + (k as i32) * 2;
            for dy in 0..3 {
                let p = w.grid.surface(x, fs.y - 12 + dy);
                let wood =
                    w.grid.materials.get(*m).kind == hf_sim::world::material::MaterialKind::Wood;
                w.grid.set(
                    p,
                    Tile::solid(if wood {
                        Material::WOOD
                    } else {
                        Material::STONE_BLOCK
                    }),
                );
                w.built_of.insert(p, *m);
            }
        }
        w.rebuild_caches();
        self.attach(w);
    }

    /// Art review only (C57): the homes of the kinds that do not build houses, below the
    /// landing: a hollow dug into a granite knoll (door, beds inside), three tents and two nests.
    #[func]
    fn stage_homes(&mut self, seed: i64) {
        self.settle();
        use hf_sim::structures::StructureKind as K;
        use hf_sim::world::{Material, Tile, TileFlags};
        let mut w = World::stage_demo(seed as u64);
        let fs = w.meta.founding_site;
        let base = w.grid.surface(fs.x + 13, fs.y - 14);
        let z = base.z;
        w.plants.retain(|p| {
            !((base.x..base.x + 7).contains(&p.pos.x) && (base.y..base.y + 6).contains(&p.pos.y))
        });
        // The knoll: rock two levels high, a room of three by three cut into it from the south.
        for y in 0..6 {
            for x in 0..7 {
                for dz in 0..2 {
                    w.grid.set(
                        TilePos::new(base.x + x, base.y + y, z + dz),
                        Tile::solid(Material::GRANITE),
                    );
                }
            }
        }
        for y in 1..4 {
            for x in 2..5 {
                w.grid
                    .set(TilePos::new(base.x + x, base.y + y, z), Tile::AIR);
            }
        }
        let mouth = TilePos::new(base.x + 3, base.y + 4, z);
        let mut door = Tile::solid(Material::WOOD);
        door.flags = TileFlags(TileFlags::DOOR);
        w.grid.set(mouth, door);
        for (x, y) in [(2, 1), (4, 1), (3, 1)] {
            w.place_structure(
                K::Bed,
                TilePos::new(base.x + x, base.y + y, z),
                Material::WOOD,
                false,
            );
        }
        for k in 0..3 {
            let p = w.grid.surface(fs.x - 2 + k * 2, fs.y - 10);
            w.place_structure(K::Tent, p, Material::WOOD, false);
        }
        for k in 0..2 {
            let p = w.grid.surface(fs.x + 6 + k * 2, fs.y - 10);
            w.place_structure(K::Nest, p, Material::WOOD, false);
        }
        w.rebuild_caches();
        self.attach(w);
    }

    /// Step N ticks; records events to the chronicle, feed and director. On the step thread
    /// (#603) this returns at once: the ticks run there, and their world shows in the view
    /// when they are done. More than 400 waiting ticks (a sim slower than the speed asked) are
    /// let go, as the client's own cap of 400 a frame always did: the game runs slower, the
    /// frames do not.
    #[func]
    fn step(&mut self, ticks: i64) {
        if self.stepper.is_some() {
            self.poll();
            self.pending = (self.pending + ticks.max(0) as u64).min(400);
            self.dispatch();
            return;
        }
        let t0 = std::time::Instant::now();
        let Some(s) = self.session.take() else {
            return;
        };
        let mut core = sim_thread::Core {
            session: s,
            feed: std::mem::take(&mut self.feed),
            sounds: std::mem::take(&mut self.sounds),
            bodies: std::mem::take(&mut self.bodies),
            director: std::mem::take(&mut self.director),
            published: self.published,
            favourites: std::mem::take(&mut self.favourites),
            suggestion: self.suggestion.take(),
            want_suggestion: false,
        };
        core.run(ticks.max(0) as u64);
        self.session = Some(core.session);
        self.feed = core.feed;
        self.sounds = core.sounds;
        self.bodies = core.bodies;
        self.director = core.director;
        self.published = core.published;
        self.favourites = core.favourites;
        self.suggestion = core.suggestion;
        self.last_step_ms = t0.elapsed().as_secs_f64() * 1000.0;
    }

    /// Wait until every tick asked for has run and the session is back (#603): for shots,
    /// footage, tests and anything that needs the world at an exact tick.
    #[func]
    fn sync(&mut self) {
        loop {
            self.settle();
            if self.pending == 0 || self.session.is_none() || self.stepper.is_none() {
                return;
            }
            self.dispatch();
        }
    }

    /// Step on the step thread (true, the default) or in place on the caller's (false; also
    /// HF_SIM_THREAD=0 or `--sim-inline`), for debugging (#603).
    #[func]
    fn set_threaded(&mut self, on: bool) {
        if on == self.stepper.is_some() {
            return;
        }
        self.sync();
        self.stepper = on.then(sim_thread::SimThread::spawn);
        if !on {
            self.snapshot = None;
        }
    }

    #[func]
    fn is_threaded(&self) -> bool {
        self.stepper.is_some()
    }

    /// Turn the debug decision log on (#565): `settings` is the run's `key=value` pairs for the
    /// header. The file is written at the first decision, with the world's seed in its header.
    #[func]
    fn debug_log_start(&mut self, path: GString, settings: GString, command_line: GString) {
        let pairs = settings
            .to_string()
            .split_whitespace()
            .filter_map(|kv| kv.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        hf_sim::debuglog::configure(
            std::path::PathBuf::from(path.to_string()),
            pairs,
            command_line.to_string(),
        );
    }

    /// Turn the debug decision log off (the settings toggle).
    #[func]
    fn debug_log_stop(&mut self) {
        hf_sim::debuglog::disable();
    }

    /// The debug log's path while it is on, else empty (the indicator reads this).
    #[func]
    fn debug_log_path(&self) -> GString {
        hf_sim::debuglog::path()
            .map(|p| GString::from(p.to_string_lossy().as_ref()))
            .unwrap_or_default()
    }

    #[func]
    fn last_step_ms(&self) -> f64 {
        self.last_step_ms
    }

    /// Save to a `.hfsave` directory. Capture happens now (blocking, measured); file
    /// writes run on a worker thread. Returns false if a save is still in progress.
    #[func]
    fn save_to(&mut self, path: GString) -> bool {
        self.settle();
        if self
            .save_thread
            .as_ref()
            .map(|t| !t.is_finished())
            .unwrap_or(false)
        {
            return false;
        }
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let t0 = std::time::Instant::now();
        let dir = std::path::PathBuf::from(path.to_string());
        if let Some(parent) = dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match hf_sim::save::prepare(&s.world, &mut s.chronicle, &dir) {
            Ok(p) => {
                self.last_save_prepare_ms = t0.elapsed().as_secs_f64() * 1000.0;
                let Some(user_data_dir) = dir.parent() else {
                    self.last_save_result = "save failed: save path has no user-data root".into();
                    godot_error!("save prepare failed: save path has no user-data root");
                    return false;
                };
                let save_lock = match hf_update::SaveWriteLock::acquire(user_data_dir) {
                    Ok(lock) => lock,
                    Err(error) => {
                        self.last_save_result = format!("save failed: {error}");
                        godot_error!("save prepare failed: {error}");
                        return false;
                    }
                };
                self.save_thread = Some(std::thread::spawn(move || {
                    let _save_lock = save_lock;
                    hf_sim::save::commit(p).map_err(|e| e.to_string())
                }));
                true
            }
            Err(e) => {
                self.last_save_result = format!("save failed: {e}");
                godot_error!("save prepare failed: {e}");
                false
            }
        }
    }

    /// Poll the background save: "" while running or idle, "ok" or an error once done.
    #[func]
    fn save_status(&mut self) -> GString {
        if let Some(t) = self.save_thread.as_ref() {
            if t.is_finished() {
                let t = self.save_thread.take().unwrap();
                self.last_save_result = match t.join() {
                    Ok(Ok(())) => "ok".into(),
                    Ok(Err(e)) => format!("save failed: {e}"),
                    Err(_) => "save thread panicked".into(),
                };
                return GString::from(self.last_save_result.as_str());
            }
            return GString::from("saving");
        }
        GString::new()
    }

    #[func]
    fn last_save_prepare_ms(&self) -> f64 {
        self.last_save_prepare_ms
    }

    /// Load a save directory, replacing the running world. Returns an error string or "".
    #[func]
    fn load_from(&mut self, path: GString) -> GString {
        self.settle();
        match hf_sim::save::read(std::path::Path::new(&path.to_string())) {
            Ok((world, chronicle)) => {
                self.favourites = chronicle.favourites();
                self.session = Some(Session::new(world, chronicle));
                self.forget_steps();
                self.feed.clear();
                self.sounds.clear();
                self.bodies.clear();
                self.director = Director::default();
                self.published = None;
                GString::new()
            }
            Err(e) => GString::from(format!("{e}").as_str()),
        }
    }

    /// Newest valid save among `base.0.hfsave` .. `base.{n-1}.hfsave`, or "".
    #[func]
    fn newest_valid(&self, base: GString, n: i64) -> GString {
        let ring =
            hf_sim::save::ring_paths(std::path::Path::new(&base.to_string()), n.max(1) as usize);
        match hf_sim::save::newest_valid(&ring) {
            Some((p, _)) => GString::from(p.to_string_lossy().as_ref()),
            None => GString::new(),
        }
    }

    /// The ring slot to write next.
    #[func]
    fn next_ring_slot(&self, base: GString, n: i64) -> GString {
        let ring =
            hf_sim::save::ring_paths(std::path::Path::new(&base.to_string()), n.max(1) as usize);
        GString::from(
            hf_sim::save::next_ring_slot(&ring)
                .to_string_lossy()
                .as_ref(),
        )
    }

    /// Set the surface at (x, y) alight (review and test runs only: `--ignite=x,y`).
    #[func]
    fn ignite_at(&mut self, x: i32, y: i32) -> bool {
        self.settle();
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let p = s.world.grid.surface(x, y);
        s.world.ignite(p, "set for a review")
    }

    /// Review hook: the first inhabited place beyond the valley has run out of food (as if after
    /// a long hunger). Returns its name, or "" if there is none.
    #[func]
    fn starve_neighbour(&mut self) -> GString {
        self.settle();
        let Some(s) = self.session.as_mut() else {
            return GString::new();
        };
        match s.world.overworld.sites.iter_mut().find(|x| x.people > 0) {
            Some(site) => {
                site.food = -(site.people as f32) * 4.0;
                site.hungry_days = 3;
                GString::from(site.name.as_str())
            }
            None => GString::new(),
        }
    }

    /// Ground where fire burned at level `z`, burned within `days`: [x, y, age_hours, ...].
    #[func]
    fn scorched(&self, z: i32, days: i64) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            let now = w.tick();
            let span = days.max(0) as u64 * hf_sim::time::TICKS_PER_DAY;
            for (p, t) in &w.scorched {
                if p.z == z && now.saturating_sub(*t) <= span {
                    out.push(p.x);
                    out.push(p.y);
                    out.push((now.saturating_sub(*t) / hf_sim::time::TICKS_PER_HOUR) as i32);
                }
            }
        }
        out
    }

    /// The thin place for the viewer (thin.rs): stone x, y, z, strength x100, hollow radius
    /// in tenths of a tile, then the echoes standing now (count, then x, y, z each) and the
    /// patches of wrong weather (count, then x, y, z, cold 1/0 each). Empty with no stone.
    #[func]
    fn thin_view(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        let Some(w) = self.world() else { return out };
        let Some(s) = hf_sim::thin::stone(w) else {
            return out;
        };
        let hollow = if w.thin.strength > 0.3 && !w.thin.quieted {
            hf_sim::thin::radius(w) * 0.5
        } else {
            0.0
        };
        for v in [
            s.x,
            s.y,
            s.z,
            (w.thin.strength * 100.0) as i32,
            (hollow * 10.0) as i32,
            w.thin.echoes.len() as i32,
        ] {
            out.push(v);
        }
        for (p, _, _) in &w.thin.echoes {
            for v in [p.x, p.y, p.z] {
                out.push(v);
            }
        }
        out.push(w.thin.patches.len() as i32);
        for (p, _, cold) in &w.thin.patches {
            for v in [p.x, p.y, p.z, *cold as i32] {
                out.push(v);
            }
        }
        out
    }

    /// Lit hearths and lamps at the current z (for sound and the director): [x, y, z, kind].
    #[func]
    fn lights(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for s in &w.structures {
                if s.active && s.kind.emits_light() {
                    for v in [s.pos.x, s.pos.y, s.pos.z, s.kind as i32] {
                        out.push(v);
                    }
                }
            }
        }
        out
    }

    /// Drain feed lines: Array of [tick, x, y, z, severity, text, kind, favourite, because, fold
    /// key] (kind, #329: the event's chronicle name; favourite, #413: whether a favourited person
    /// took part; because, #413 A5: the simulation's causal line for the event when present; fold
    /// key, #1483: rows with the same key are the same news again, "" for a row that never folds,
    /// `view::fold::key`).
    #[func]
    fn drain_feed(&mut self) -> VarArray {
        // Collect a finished batch first (never waits): paused, nothing else would (FIXLOG 2026-11-14).
        self.poll();
        let mut out = VarArray::new();
        for (t, p, sev, text, kind, fav, because, key) in self.feed.drain(..) {
            let mut row = VarArray::new();
            row.push(&(t as i64).to_variant());
            row.push(&p.x.to_variant());
            row.push(&p.y.to_variant());
            row.push(&p.z.to_variant());
            row.push(&sev.to_variant());
            row.push(&GString::from(text.as_str()).to_variant());
            row.push(&GString::from(kind).to_variant());
            row.push(&fav.to_variant());
            row.push(&GString::from(because.as_str()).to_variant());
            row.push(&GString::from(key.as_str()).to_variant());
            out.push(&row.to_variant());
        }
        out
    }

    /// Drain event sounds: Array of [sound name, x, y, z] (douse when a fire is put out, flare
    /// when one starts), oldest first.
    #[func]
    fn drain_sounds(&mut self) -> VarArray {
        self.poll();
        let mut out = VarArray::new();
        for (name, p) in self.sounds.drain(..) {
            let mut row = VarArray::new();
            row.push(&GString::from(name).to_variant());
            row.push(&p.x.to_variant());
            row.push(&p.y.to_variant());
            row.push(&p.z.to_variant());
            out.push(&row.to_variant());
        }
        out
    }

    /// Director suggestion: [x, y, z, score, dwell_ticks, reason].
    #[func]
    fn director_suggest(&mut self) -> VarArray {
        self.suggest_asked = Some(std::time::Instant::now());
        // At home the director answers now; while the session is on the step thread, the
        // answer that thread worked out after its last batch stands. Before #797 finding 2 the
        // thread worked none out, and a running game's camera never moved (FIXLOG 2026-11-14).
        if let Some(s) = self.session.as_ref() {
            let fallback = settlement_centre(&s.world);
            let people = hf_sim::view::director::people(&s.world);
            self.suggestion = Some(self.director.suggest(s.world.tick(), fallback, &people));
        }
        let mut out = VarArray::new();
        let Some(sug) = self.suggestion.as_ref() else {
            return out;
        };
        out.push(&sug.at.x.to_variant());
        out.push(&sug.at.y.to_variant());
        out.push(&sug.at.z.to_variant());
        out.push(&sug.score.to_variant());
        out.push(&(sug.dwell as i64).to_variant());
        // In a viewer's words, not the event kind's name (#200).
        out.push(&GString::from(hf_sim::view::director::watching(sug.kind)).to_variant());
        out
    }

    /// Away summary since `from_tick`: Dictionary {counts: {kind: n}, lines: [text], from, to}.
    #[func]
    fn summary_since(&mut self, from_tick: i64) -> VarDictionary {
        self.settle();
        let to = self.world().map(|w| w.tick() as i64).unwrap_or(0);
        self.summary_range(from_tick, to, 12)
    }

    /// The Catch Up card since `from_tick`: Dictionary {text, from, to}, `text` being the card in
    /// reading order (headline in words, named changes, folded details) as BBCode. Empty when
    /// there is no world. `summary_since` and its raw `counts` stay as they were.
    #[func]
    fn catch_up_since(&mut self, from_tick: i64) -> VarDictionary {
        self.settle();
        let mut d = VarDictionary::new();
        let Some(s) = self.session.as_ref() else {
            return d;
        };
        let to = s.world.tick();
        match summary::catch_up_card(&s.world, &s.chronicle, from_tick.max(0) as u64, to, 12) {
            Ok(card) => {
                d.set("text", &GString::from(card.bbcode().as_str()));
                d.set("has_news", card.has_news);
                d.set("from", from_tick);
                d.set("to", to as i64);
            }
            Err(e) => godot_error!("catch up failed: {e}"),
        }
        d
    }

    /// The Catch Up card for a span ending now: `span` is "previous_day", "current_day",
    /// "previous_season" or "current_season". Dictionary {text, has_news, from, to}; empty when
    /// there is no world, the name is unknown, or there is no previous span yet.
    #[func]
    fn catch_up_span(&mut self, span: GString) -> VarDictionary {
        self.settle();
        let mut d = VarDictionary::new();
        let Some(s) = self.session.as_ref() else {
            return d;
        };
        let Some(which) = summary::CatchUpSpan::parse(&span.to_string()) else {
            return d;
        };
        let now = s.world.tick();
        match summary::catch_up_card_span(&s.world, &s.chronicle, which, 12) {
            Ok(Some(card)) => {
                let (from, to) =
                    summary::catch_up_span_bounds(s.world.meta.epoch, now, which).unwrap_or((0, 0));
                d.set("text", &GString::from(card.bbcode().as_str()));
                d.set("has_news", card.has_news);
                d.set("from", from.max(0));
                d.set("to", to);
            }
            Ok(None) => {}
            Err(e) => godot_error!("catch up span failed: {e}"),
        }
        d
    }

    /// The record the viewer has been told of between two ticks (after `from_tick`, up to and
    /// including `to_tick`), in the shape of `summary_since`, with at most `max_major` lines.
    /// The timelapse stores it beside each sample as that sample's event markers (#484).
    #[func]
    fn summary_between(&mut self, from_tick: i64, to_tick: i64, max_major: i64) -> VarDictionary {
        self.settle();
        self.summary_range(from_tick, to_tick, max_major.clamp(0, 64) as usize)
    }

    /// Which world this is, the same at every tick and in every save of it (#484).
    #[func]
    fn world_identity(&self) -> GString {
        GString::from(
            self.world()
                .map(hf_sim::digest::world_identity)
                .unwrap_or_default()
                .as_str(),
        )
    }

    /// The tick the portal opened: day 1 of the calendar people see starts here (docs/TIME.md).
    #[func]
    fn epoch(&self) -> i64 {
        self.world().map(|w| w.meta.epoch as i64).unwrap_or(0)
    }

    /// A page of the Legends browser (view::legends): "world:0", "people:N", "site:N",
    /// "figure:N" or "polity:N", as BBCode with [url=kind:id] links.
    #[func]
    fn legends_page(&mut self, target: GString) -> GString {
        self.settle();
        let Some(s) = self.session.as_ref() else {
            return GString::new();
        };
        GString::from(
            hf_sim::view::legends::page(&s.world, &s.chronicle, &target.to_string()).as_str(),
        )
    }

    /// The world's last year before the portal (0 in a world without one).
    #[func]
    fn realm_year(&self) -> i64 {
        self.world().map(|w| w.realm.year).unwrap_or(0)
    }

    /// The world map's side in regions (0 without a world).
    #[func]
    fn legends_map_size(&mut self) -> i64 {
        self.settle();
        self.realm_map_ready().map(|m| m.size as i64).unwrap_or(0)
    }

    /// The world map at a year, as RGB bytes (size x size x 3).
    #[func]
    fn legends_map(&mut self, year: i64) -> PackedByteArray {
        self.settle();
        let Some(size) = self.realm_map_ready().map(|m| m.size) else {
            return PackedByteArray::new();
        };
        let _ = size;
        let (Some(s), Some((_, m))) = (self.session.as_ref(), self.realm_map.as_ref()) else {
            return PackedByteArray::new();
        };
        PackedByteArray::from(hf_sim::view::legends::map_rgb(&s.world, m, year).as_slice())
    }

    /// The world's history before the portal for the reel at a new game's start (#561):
    /// [realm year, PackedStringArray of that year's notable lines], oldest first.
    #[func]
    fn history_reel(&mut self) -> VarArray {
        self.settle();
        let mut out = VarArray::new();
        let Some(s) = self.session.as_ref() else {
            return out;
        };
        if let Ok(years) = hf_sim::view::history::reel(&s.world, &s.chronicle) {
            for (year, lines) in years {
                let mut r = VarArray::new();
                r.push(&year.to_variant());
                let l: PackedStringArray =
                    lines.iter().map(|x| GString::from(x.as_str())).collect();
                r.push(&l.to_variant());
                out.push(&r.to_variant());
            }
        }
        out
    }

    /// Those of the valley in another place (#562, view::away): name, place ("" when the
    /// valley does not know where), region on the world map (-1 when not shown), reaction
    /// ("welcomed", "threatened", ...) and a line saying what they are doing there.
    #[func]
    fn away_list(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(w) = self.world() else {
            return out;
        };
        for r in hf_sim::view::away::list(w) {
            let mut d = VarDictionary::new();
            d.set("name", &GString::from(r.name.as_str()));
            d.set("place", &GString::from(r.place.as_str()));
            d.set("region", r.region.map(|x| x as i64).unwrap_or(-1));
            d.set("reaction", &GString::from(r.reaction.word()));
            d.set("line", &GString::from(r.line.as_str()));
            out.push(&d.to_variant());
        }
        out
    }

    /// The history view: eras, the major record with detail, and customs with origins.
    #[func]
    fn history(&mut self, limit: i64) -> VarDictionary {
        self.settle();
        let mut d = VarDictionary::new();
        let Some(s) = self.session.as_ref() else {
            return d;
        };
        // The history panel's three parts (#203): the colony's years newest first, the
        // valley's own past newest first, and the wider world a line per century, each as
        // [label, summary] ([label, summary, named lines] for a century).
        let row = |label: &str, summary: String| {
            let mut r = VarArray::new();
            r.push(&GString::from(label).to_variant());
            r.push(&GString::from(summary.as_str()).to_variant());
            r
        };
        let mut colony = VarArray::new();
        let mut valley = VarArray::new();
        let mut world = VarArray::new();
        if let Ok(o) = hf_sim::view::history::overview(&s.world, &s.chronicle) {
            for e in &o.colony {
                colony.push(&row(&e.label, hf_sim::view::history::summary(&e.counts)).to_variant());
            }
            for e in &o.valley {
                valley.push(&row(&e.label, hf_sim::view::history::summary(&e.counts)).to_variant());
            }
            for c in &o.world {
                let mut r = row(&c.label, hf_sim::view::history::summary(&c.counts));
                let mut named = PackedStringArray::new();
                for l in &c.named {
                    named.push(l.as_str());
                }
                if c.more > 0 {
                    named.push(format!("and {} more in Legends", c.more).as_str());
                }
                r.push(&named.to_variant());
                world.push(&r.to_variant());
            }
        }
        d.set("colony", &colony);
        d.set("valley", &valley);
        d.set("world", &world);
        let mut lines = VarArray::new();
        if let Ok(list) =
            hf_sim::view::history::major(&s.world, &s.chronicle, limit.max(1) as usize)
        {
            for (hid, tick, line, detail) in list {
                let mut text = format!("{}  {}", hf_sim::view::history::when(&s.world, tick), line);
                // An event the record gives a cause for opens its chain.
                if hf_sim::view::causes::has_cause(&s.chronicle, hid) {
                    text.push_str(&format!("  [url=why:{hid}]why?[/url]"));
                }
                if let Some(dd) = detail {
                    text.push_str("\n    ");
                    text.push_str(&dd);
                }
                lines.push(&GString::from(text.as_str()).to_variant());
            }
        }
        d.set("lines", &lines);
        let mut customs = VarArray::new();
        if let Ok(list) = hf_sim::view::history::custom_origins(&s.world, &s.chronicle) {
            for o in list {
                let text = format!(
                    "{}: {} ({} keep it, {:.0}%). {} Because, {}: {}",
                    o.community,
                    o.label,
                    o.holders,
                    o.intensity * 100.0,
                    o.narrative,
                    hf_sim::view::history::when(&s.world, o.origin_tick),
                    o.origin_line.unwrap_or_else(|| "(record missing)".into())
                );
                customs.push(&GString::from(text.as_str()).to_variant());
            }
        }
        d.set("customs", &customs);
        d
    }

    /// Cells (x/8, y/8) on `z` that community `cid` has seen, flat [cx, cy, cx, cy, ...].
    /// cid < 0 means the first living community.
    #[func]
    fn explored(&self, cid: i64, z: i64) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        let Some(w) = self.world() else { return out };
        let c = if cid < 0 {
            w.communities.iter().find(|c| !c.dissolved)
        } else {
            w.communities.iter().find(|c| c.id.0 as i64 == cid)
        };
        if let Some(c) = c {
            for (cx, cy, cz) in &c.explored {
                if *cz as i64 == z {
                    out.push(*cx);
                    out.push(*cy);
                }
            }
        }
        out
    }

    /// Inspect an inhabitant by id.
    #[func]
    fn inspect(&mut self, id: i32) -> VarDictionary {
        // Their life before the portal, narrated from its chronicle rows: it never changes, so
        // it is read once a person (waiting for the step thread that once) and kept, and
        // inspecting reads the snapshot without waiting (#603).
        if !self.past_cache.contains_key(&id) {
            self.settle();
            if let Some(s) = self.session.as_ref() {
                let w = &s.world;
                let lines: Vec<String> = w
                    .agents
                    .iter()
                    .find(|a| a.id.0 as i32 == id)
                    .map(|a| {
                        a.past
                            .iter()
                            .filter_map(|r| s.chronicle.row(*r).ok().flatten())
                            .filter_map(|(hid, tick, detail)| {
                                hf_sim::view::history::event_from_row(hid, tick, &detail)
                            })
                            .map(|e| hf_sim::view::narrate::line(w, &e))
                            .collect()
                    })
                    .unwrap_or_default();
                self.past_cache.insert(id, lines);
            }
        }
        let mut d = VarDictionary::new();
        let Some(w) = self.world() else { return d };
        let Some(a) = w
            .agents
            .iter()
            .find(|a| a.id.0 as i32 == id && !hf_sim::view::known::is_unfound(w, a.id))
        else {
            return d;
        };
        d.set("name", &GString::from(a.name.as_str()));
        // A person's species by name, with what it means for them (P3); a beast's kind.
        if a.species == hf_sim::agents::Species::Person {
            let sp = a.species_def();
            d.set("species", &GString::from(sp.name.as_str()));
            d.set(
                "kind",
                &GString::from(
                    format!(
                        "{} ({:.0} cm, {:.0} kg)",
                        sp.describe(),
                        a.height_cm(),
                        a.mass_kg()
                    )
                    .as_str(),
                ),
            );
        } else {
            d.set(
                "species",
                &GString::from(format!("{:?}", a.species).as_str()),
            );
        }
        d.set("age", hf_sim::time::age_years(a.born, w.tick()) as i64);
        // Their life before the portal, narrated from its chronicle rows (oldest first).
        let mut past = VarArray::new();
        for l in self.past_cache.get(&id).into_iter().flatten() {
            past.push(&GString::from(l.as_str()).to_variant());
        }
        d.set("past", &past);
        // What they hold true, how surely, and where it came from.
        let mut beliefs = VarArray::new();
        for b in &a.beliefs.list {
            if b.stance.abs() * b.confidence < 0.08 {
                continue;
            }
            let sure = if b.confidence > 0.7 {
                "surely"
            } else if b.confidence > 0.4 {
                "fairly sure"
            } else {
                "half-believes"
            };
            let mut from: Vec<String> = Vec::new();
            let lived = b
                .sources
                .iter()
                .filter(|s| matches!(s, hf_sim::mind::beliefs::Source::Event(_)))
                .count();
            if lived > 0 {
                from.push(if lived == 1 {
                    "what happened".into()
                } else {
                    format!("{lived} things that happened")
                });
            }
            for s in &b.sources {
                match s {
                    hf_sim::mind::beliefs::Source::Person(p) => {
                        if let Some(o) = w.agents.iter().find(|x| x.id == *p) {
                            let t = format!("told by {}", o.name);
                            if !from.contains(&t) {
                                from.push(t);
                            }
                        }
                    }
                    hf_sim::mind::beliefs::Source::Ideology(_) => {
                        let t = "what their people held".to_string();
                        if !from.contains(&t) {
                            from.push(t);
                        }
                    }
                    // P7: the culture they were raised in, and the leanings of their kind.
                    hf_sim::mind::beliefs::Source::Culture(c) => {
                        if let Some(c) = w.realm.cultures.get(*c as usize) {
                            let t = format!("the way of {}", c.name);
                            if !from.contains(&t) {
                                from.push(t);
                            }
                        }
                    }
                    hf_sim::mind::beliefs::Source::Species(sp) => {
                        let t = format!("what {} lean toward", w.kind(*sp).plural);
                        if !from.contains(&t) {
                            from.push(t);
                        }
                    }
                    _ => {}
                }
            }
            beliefs.push(
                &GString::from(
                    format!(
                        "{} ({}; from {})",
                        hf_sim::mind::beliefs::phrase(b.topic, b.stance),
                        sure,
                        from.join(", ")
                    )
                    .as_str(),
                )
                .to_variant(),
            );
        }
        d.set("beliefs", &beliefs);
        // What they know how to do and where they learned it (techniques.rs; C13 adds studied).
        let mut techs = VarArray::new();
        for k in &a.techniques.list {
            let how = match (k.learned, k.from.is_empty()) {
                (_, false) => k.from.clone(),
                (hf_sim::mind::techniques::Learned::Past, true) => "from their past".to_string(),
                (hf_sim::mind::techniques::Learned::Watched, true) => "from watching".to_string(),
                (hf_sim::mind::techniques::Learned::Tried, true) => "by trying".to_string(),
                (hf_sim::mind::techniques::Learned::Studied, true) => "by study".to_string(),
                (hf_sim::mind::techniques::Learned::Taught, true) => "taught".to_string(),
            };
            techs.push(
                &GString::from(format!("{} ({how})", k.technique.name()).as_str()).to_variant(),
            );
        }
        d.set("techniques", &techs);
        // What they were told and what they made of it, newest first (#487).
        let mut told = PackedStringArray::new();
        for t in a.told.iter().rev() {
            let teller = w
                .agents
                .iter()
                .find(|o| o.id == t.from)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "someone".to_string());
            told.push(Self::told_line(&t.what.words(), &teller, t.took, t.trust).as_str());
        }
        d.set("told", &told);
        let mut tales = VarArray::new();
        for t in hf_sim::lore::heard(w, a)
            .into_iter()
            .chain(hf_sim::accounts::heard(w, a))
        {
            tales.push(&GString::from(t.as_str()).to_variant());
        }
        d.set("tales", &tales);
        d.set("home", &GString::from(a.home.as_str()));
        let surf = w.grid.ground_z(a.pos.x, a.pos.y);
        let rel_z = if a.pos.z == surf + 1 {
            "surface".to_string()
        } else if a.pos.z > surf + 1 {
            format!("{} above ground", a.pos.z - (surf + 1))
        } else {
            format!("depth {} below ground", (surf + 1) - a.pos.z)
        };
        let where_str = if let Some(cav) = w.grid.caverns.iter().find(|c| c.contains(a.pos)) {
            format!(
                "in {} ({}, {}, z{} — {rel_z})",
                cav.name, a.pos.x, a.pos.y, a.pos.z
            )
        } else {
            format!("at ({}, {}, z{} — {rel_z})", a.pos.x, a.pos.y, a.pos.z)
        };
        d.set(
            "activity",
            &GString::from(format!("{} — {where_str}", a.activity.describe()).as_str()),
        );
        d.set("surface_z", surf);
        d.set("where", &GString::from(where_str.as_str()));
        d.set("activity_ticks", a.activity_ticks as i64);
        let mut needs = VarDictionary::new();
        needs.set("hunger", a.needs.hunger);
        needs.set("rest", a.needs.rest);
        needs.set("warmth", a.needs.warmth);
        needs.set("social", a.needs.social);
        needs.set("comfort", a.needs.comfort);
        d.set("needs", &needs);
        // Temperament, skills and a tool are a person's; a beast (or one from an old save that
        // was given them) shows none.
        let person = a.species == hf_sim::agents::Species::Person;
        let mut traits = VarArray::new();
        let mut skills = VarDictionary::new();
        if person {
            for t in a.surface_traits() {
                traits.push(&GString::from(t.label).to_variant());
            }
            for s in hf_sim::mind::skills::ALL {
                skills.set(&GString::from(format!("{s:?}").as_str()), a.skills.level(s));
            }
        }
        d.set("traits", &traits);
        d.set("skills", &skills);
        d.set("known", a.knowledge.map.len() as i64);
        let mut known = VarArray::new();
        for k in a.knowledge.map.keys().take(12) {
            let line = hf_sim::view::known::describe(w, k);
            known.push(&GString::from(line.as_str()).to_variant());
        }
        d.set("known_list", &known);
        d.set("mood", a.mood);
        d.set(
            "mood_word",
            &GString::from(hf_sim::view::why::mood_word(hf_sim::view::why::mood_band(
                a,
            ))),
        );
        d.set(
            "tool",
            &GString::from(
                // The tool in hand and its story (legend.rs); a tool from before stories as it was.
                if !person {
                    String::new()
                } else if let Some(story) = a.tool.and_then(|t| hf_sim::legend::describe(w, t)) {
                    story
                } else if a.tool_quality > 0.0 {
                    format!(
                        "{} tool, {:.0}% left",
                        hf_sim::items::ItemKind::tier_name(hf_sim::items::ItemKind::tier(
                            a.tool_quality
                        )),
                        a.tool_wear * 100.0
                    )
                } else {
                    "no tool".to_string()
                }
                .as_str(),
            ),
        );
        // A spear in hand and armour on (C33), each with its story.
        let arms: Vec<String> = [a.weapon, a.armour]
            .into_iter()
            .flatten()
            .filter_map(|id| hf_sim::legend::describe(w, id))
            .collect();
        d.set("arms", &GString::from(arms.join("; ").as_str()));
        // Every item currently attached to the person is exposed in one of two sections. Held
        // items are the equipped/worn layer; carried items are their inventory. Storied items
        // include the owner chain and maker/origin text, so the player can follow an heirloom
        // without opening the legends browser.
        let mut equipment = VarArray::new();
        let mut inventory = VarArray::new();
        for it in &w.items {
            match it.place {
                hf_sim::items::ItemPlace::Held(id) if id == a.id => {
                    let role = if Some(it.id) == a.weapon {
                        "weapon"
                    } else if Some(it.id) == a.armour {
                        "armour"
                    } else if Some(it.id) == a.tool {
                        "tool"
                    } else if it.kind == hf_sim::items::ItemKind::Clothes {
                        "wearing"
                    } else {
                        "held"
                    };
                    equipment.push(
                        &GString::from(
                            format!("{role}: {}", Self::inspect_item_line(w, it, a.id)).as_str(),
                        )
                        .to_variant(),
                    );
                }
                hf_sim::items::ItemPlace::Carried(id) if id == a.id => {
                    inventory.push(
                        &GString::from(Self::inspect_item_line(w, it, a.id).as_str()).to_variant(),
                    );
                }
                _ => {}
            }
        }
        d.set("equipment", &equipment);
        d.set("inventory", &inventory);
        d.set("clothes_wear", a.clothes_wear);
        d.set("body", &GString::from(a.body.summary().as_str()));
        d.set("stress", a.stress);
        // The why of a mood (#292).
        d.set(
            "feeling",
            &GString::from(hf_sim::view::why::feeling(w, a).as_str()),
        );
        d.set(
            "why",
            &GString::from(hf_sim::view::why::because(w, a).as_str()),
        );
        let mut thoughts = VarArray::new();
        let mut tl: Vec<&hf_sim::mind::Thought> = a.thoughts.list.iter().collect();
        tl.sort_by(|x, y| y.weight.abs().partial_cmp(&x.weight.abs()).unwrap());
        for t in tl.iter().take(6) {
            thoughts.push(
                &GString::from(format!("{} {:+.2}", t.kind.label(), t.weight).as_str())
                    .to_variant(),
            );
        }
        d.set("thoughts", &thoughts);
        let cname = a
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
            .map(|c| c.name.clone())
            .unwrap_or_default();
        d.set("community", &GString::from(cname.as_str()));
        // What their people has come to be (customs and shared convictions).
        let people: PackedStringArray = a
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
            .map(|c| {
                c.identity
                    .iter()
                    .map(|l| GString::from(l.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        d.set("people", &people);
        d.set(
            "role",
            &GString::from(
                // #454: the duties they hold and the hours behind them (duties.rs).
                if a.duties.held.is_empty() {
                    a.role
                        .map(|r| format!("{r:?}").to_lowercase())
                        .unwrap_or_default()
                } else {
                    hf_sim::duties::describe(a)
                }
                .as_str(),
            ),
        );
        let mut mems = VarArray::new();
        let mut sorted: Vec<&hf_sim::mind::Memory> = a.memories.long.iter().collect();
        sorted.sort_by(|x, y| {
            y.strength
                .partial_cmp(&x.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for m in sorted.iter().take(8) {
            let subject = match m.subject {
                hf_sim::mind::Subject::Person(id) => w
                    .agents
                    .iter()
                    .find(|o| o.id == id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| format!("#{}", id.0)),
                hf_sim::mind::Subject::Place(p) => format!("{}", p),
                hf_sim::mind::Subject::Food(k) => format!("{:?}", k).to_lowercase(),
                hf_sim::mind::Subject::None => String::new(),
            };
            let tone = if m.valence > 0.25 {
                "fond"
            } else if m.valence < -0.25 {
                "sour"
            } else {
                "plain"
            };
            mems.push(
                &GString::from(
                    format!(
                        "{:?} {} ({}, {:.0}%)",
                        m.kind,
                        subject,
                        tone,
                        m.strength * 100.0
                    )
                    .as_str(),
                )
                .to_variant(),
            );
        }
        d.set("memories", &mems);
        let mut rels = VarArray::new();
        let mut rs: Vec<(&hf_sim::ids::AgentId, &hf_sim::mind::Relation)> =
            a.relationships.iter().collect();
        rs.sort_by(|x, y| {
            y.1.opinion
                .partial_cmp(&x.1.opinion)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (id, r) in rs.iter().take(6) {
            let name = w
                .agents
                .iter()
                .find(|o| o.id == **id)
                .map(|o| o.name.clone())
                .unwrap_or_default();
            let word = if r.opinion > 0.4 {
                "friend"
            } else if r.opinion > 0.1 {
                "likes"
            } else if r.opinion < -0.3 {
                "dislikes"
            } else {
                "knows"
            };
            rels.push(
                &GString::from(format!("{} {} ({:+.2})", word, name, r.opinion).as_str())
                    .to_variant(),
            );
        }
        d.set("relationships", &rels);
        let who = |id: Option<hf_sim::ids::AgentId>| {
            id.and_then(|id| w.agents.iter().find(|o| o.id == id))
                .map(|o| o.name.clone())
                .unwrap_or_default()
        };
        let mut customs = VarArray::new();
        if let Some(cid) = a.community {
            for c in w.customs.iter().filter(|c| c.community == cid && !c.lapsed) {
                let held = if c.adopted_by.contains(&a.id) {
                    ""
                } else {
                    " (not yet taken up)"
                };
                customs.push(
                    &GString::from(format!("{}{} — {}", c.label(), held, c.narrative).as_str())
                        .to_variant(),
                );
            }
        }
        d.set("customs", &customs);
        d.set("partner", &GString::from(who(a.partner).as_str()));
        d.set(
            "parents",
            &GString::from(format!("{} {}", who(a.mother), who(a.father)).trim()),
        );
        let kids: Vec<String> = a.children.iter().map(|c| who(Some(*c))).collect();
        d.set("children", &GString::from(kids.join(", ").as_str()));
        d.set("x", a.pos.x);
        d.set("y", a.pos.y);
        d.set("z", a.pos.z);
        d.set("favourite", self.favourites.contains(&a.id));

        // E16: personal experience only. This readout is derived from the selected agent's
        // bounded records; it does not scan the world or expose another person's knowledge.
        let experience = experience::inspect(a);
        let mut observations = VarArray::new();
        for o in experience.observations {
            let mut row = VarDictionary::new();
            row.set("tick", o.tick as i64);
            row.set("x", o.pos.x);
            row.set("y", o.pos.y);
            row.set("z", o.pos.z);
            row.set("kind", o.kind);
            row.set("strength", o.strength);
            row.set("full", o.full);
            row.set("evidence", o.evidence);
            row.set("event", o.event.map(|id| id as i64).unwrap_or(-1));
            row.set("reading", o.reading.unwrap_or_default());
            observations.push(&row);
        }
        d.set("observations", &observations);

        let mut lessons = VarArray::new();
        for l in experience.lessons {
            let mut row = VarDictionary::new();
            row.set("cue", l.cue);
            row.set("outcome", l.outcome);
            row.set("confidence", l.confidence);
            row.set("confirmations", l.confirmations as i64);
            row.set("contradictions", l.contradictions as i64);
            row.set("formed", l.formed as i64);
            row.set("last_tested", l.last_tested as i64);
            row.set(
                "event_origins",
                l.origins.iter().map(|id| *id as i64).collect::<Vec<_>>(),
            );
            if let Some(pos) = l.pos {
                row.set("x", pos.x);
                row.set("y", pos.y);
                row.set("z", pos.z);
            }
            lessons.push(&row);
        }
        d.set("lessons", &lessons);

        let mut problems = VarArray::new();
        for p in experience.problems {
            let mut row = VarDictionary::new();
            row.set("kind", p.kind);
            row.set("x", p.pos.x);
            row.set("y", p.pos.y);
            row.set("z", p.pos.z);
            row.set("urgency", p.urgency);
            row.set("status", p.status);
            row.set("evidence", p.evidence);
            row.set("opened", p.opened as i64);
            row.set("last_seen", p.last_seen as i64);
            row.set("trigger", p.trigger.map(|id| id as i64).unwrap_or(-1));
            row.set("reaction", p.reaction.unwrap_or_default());
            row.set(
                "outcome",
                p.outcome.unwrap_or_else(|| "pending".to_string()),
            );
            row.set("attempts", p.attempts as i64);
            problems.push(&row);
        }
        d.set("problems", &problems);

        let mut reaction_trace = VarArray::new();
        for r in experience.reaction_trace {
            let mut row = VarDictionary::new();
            row.set("problem", r.problem);
            row.set("x", r.pos.x);
            row.set("y", r.pos.y);
            row.set("z", r.pos.z);
            row.set("tick", r.tick as i64);
            row.set("reaction", r.reaction);
            row.set("expected", r.expected.unwrap_or_default());
            row.set("actual", r.actual.unwrap_or_default());
            row.set("outcome", r.outcome);
            row.set("event", r.event.map(|id| id as i64).unwrap_or(-1));
            reaction_trace.push(&row);
        }
        d.set("reaction_trace", &reaction_trace);

        // The Why? lens (#567): the traced person's recent decision cycles, newest first, from
        // the simulation's bounded read model (hf_sim::why). Empty unless this person is traced.
        let traced = w.why_target() == Some(a.id);
        d.set("why_traced", traced);
        let mut cycles = VarArray::new();
        if let Some(t) = w.why.as_ref().filter(|_| traced) {
            for c in t.cycles.iter().rev() {
                cycles.push(&Self::why_cycle(c));
            }
        }
        d.set("why_cycles", &cycles);
        // What the traced person's attention caught or missed (#569), newest first.
        let mut percepts = VarArray::new();
        if let Some(t) = w.why.as_ref().filter(|_| traced) {
            for p in t.percepts.iter().rev() {
                let mut pd = VarDictionary::new();
                pd.set("domain", p.domain);
                pd.set("what", p.what.as_str());
                pd.set("salience", p.salience);
                pd.set("noticed", p.noticed);
                pd.set("habituated", p.habituated);
                percepts.push(&pd);
            }
        }
        d.set("why_percepts", &percepts);
        d
    }

    /// One telling in words (#487): what, from whom, and whether they took it.
    fn told_line(
        what: &str,
        teller: &str,
        took: hf_sim::mind::telling::Took,
        trust: f32,
    ) -> String {
        use hf_sim::mind::telling::Took;
        match took {
            Took::Taken => format!("{what}, from {teller}: took it (trust {trust:.2})"),
            Took::Misheard => {
                format!("{what}, from {teller}: took it, but misheard (trust {trust:.2})")
            }
            Took::Refused => format!("{what}, from {teller}: would not take it (trust {trust:.2})"),
        }
    }

    /// One decision cycle of the Why? lens as a dictionary (#567).
    fn why_cycle(c: &hf_sim::why::Cycle) -> VarDictionary {
        let mut row = VarDictionary::new();
        row.set("tick", c.tick as i64);
        let cal = hf_sim::time::Calendar::from_tick(c.tick);
        row.set(
            "when",
            format!(
                "{:?} day {} {:02}:{:02}",
                cal.season, cal.day_of_season, cal.hour, cal.minute
            )
            .as_str(),
        );
        match &c.mode {
            hf_sim::why::Mode::Deliberate => {
                row.set("mode", "deliberate");
                row.set("interrupted", "");
            }
            hf_sim::why::Mode::Interrupted { was, by } => {
                row.set("mode", "interrupted");
                row.set("interrupted", format!("{by}: stopped {was}").as_str());
            }
        }
        row.set("chosen", c.chosen.as_str());
        let mut because = PackedStringArray::new();
        for b in &c.because {
            because.push(b.as_str());
        }
        row.set("because", &because);
        let mut put_off = PackedStringArray::new();
        for p in &c.put_off {
            put_off.push(p.as_str());
        }
        row.set("put_off", &put_off);
        let mut conditions = VarArray::new();
        for k in &c.conditions {
            let mut kd = VarDictionary::new();
            kd.set("name", k.name);
            kd.set("value", k.value);
            kd.set("ok", k.ok);
            conditions.push(&kd);
        }
        row.set("conditions", &conditions);
        let mut options = VarArray::new();
        for o in &c.options {
            let mut od = VarDictionary::new();
            od.set("cycle", o.cycle);
            od.set("what", o.what.as_str());
            od.set("base", o.base);
            od.set("emotion", o.emotion);
            od.set("gate", o.gate);
            od.set("learned", o.learned);
            od.set("score", o.score);
            od.set("rejected", o.rejected.unwrap_or(""));
            od.set("chosen", o.chosen);
            options.push(&od);
        }
        row.set("options", &options);
        row
    }

    /// Trace inhabitant `id`'s decisions for the Why? lens (#567), or stop with -1. Selecting a
    /// person calls this; the trace is a read model and never changes the simulation.
    #[func]
    fn set_why_target(&mut self, id: i32) {
        self.settle();
        if let Some(s) = self.session.as_mut() {
            s.world
                .trace_why((id >= 0).then_some(hf_sim::ids::AgentId(id as u32)));
        }
    }

    /// The inhabitant the Why? lens is tracing, or -1 (#567): the client's tracing indicator.
    #[func]
    fn why_target(&self) -> i32 {
        self.world()
            .and_then(|w| w.why_target())
            .map(|id| id.0 as i32)
            .unwrap_or(-1)
    }

    /// Toggle whether inhabitant `id` is in the viewer's favourites list for this save (#413).
    /// Returns the new state (`true` if now favourited).
    #[func]
    fn toggle_favourite(&mut self, id: i32) -> bool {
        self.settle();
        if id < 0 {
            return false;
        }
        let aid = hf_sim::ids::AgentId(id as u32);
        let now_fav = if self.favourites.contains(&aid) {
            self.favourites.remove(&aid);
            false
        } else {
            self.favourites.insert(aid);
            true
        };
        if let Some(s) = self.session.as_ref() {
            if let Err(e) = s.chronicle.set_favourites(&self.favourites) {
                godot_error!("failed to save favourites: {e}");
            }
        }
        now_fav
    }

    /// Whether inhabitant `id` is in the viewer's favourites list for this save (#413).
    #[func]
    fn is_favourite(&self, id: i32) -> bool {
        id >= 0 && self.favourites.contains(&hf_sim::ids::AgentId(id as u32))
    }

    /// All favourited inhabitants for the Favourites panel (#413): name, where they are,
    /// what they are doing now, alive or dead, and which community they belong to.
    #[func]
    fn favourites(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(w) = self.world() else { return out };
        for &aid in &self.favourites {
            if let Some(row) = person_row(w, aid) {
                out.push(&row.to_variant());
            }
        }
        out
    }

    /// Everyone of the valley's communities for the People panel (#351): the living here, then
    /// those away, each community's people together and by name.
    #[func]
    fn people(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(w) = self.world() else { return out };
        let mut ids: Vec<(u32, String, bool, hf_sim::ids::AgentId)> = w
            .agents
            .iter()
            .filter(|a| a.alive && a.community.is_some())
            .map(|a| (a.community.map_or(0, |c| c.0), a.name.clone(), false, a.id))
            .chain(
                w.overworld
                    .away
                    .iter()
                    .filter(|a| a.community.is_some())
                    .map(|a| (a.community.map_or(0, |c| c.0), a.name.clone(), true, a.id)),
            )
            .collect();
        ids.sort();
        for (_, _, _, aid) in ids {
            if let Some(row) = person_row(w, aid) {
                out.push(&row.to_variant());
            }
        }
        out
    }

    /// Inspect a block at `(x, y, z)` (#413): ground/rock & material, water kind (fresh, swamp,
    /// foul, or none), water quality (#453: fresh, salt, brackish, or none) & depth, minerals &
    /// metals, temperature & shelter, plants, loose items, built structures, and historical notes.
    #[func]
    fn inspect_block(&mut self, x: i32, y: i32, z: i32) -> VarDictionary {
        // Home from the step thread first: who built it is in the chronicle, which travels with
        // the session, so in a running game the note was left off (FIXLOG 2026-11-14). A click,
        // so one wait for the batch in hand.
        self.settle();
        let mut d = VarDictionary::new();
        let Some(w) = self.world() else { return d };
        let info = hf_sim::view::block::inspect(w, TilePos::new(x, y, z));
        d.set("x", info.x);
        d.set("y", info.y);
        d.set("z", info.z);
        d.set("surface_z", info.surface_z);
        d.set("title", &GString::from(info.title.as_str()));
        d.set("biome", &GString::from(info.biome.as_str()));
        d.set("ground", &GString::from(info.ground.as_str()));
        d.set("water", &GString::from(info.water.as_str()));
        d.set("water_kind", &GString::from(info.water_kind.as_str()));
        d.set("water_quality", &GString::from(info.water_quality.as_str()));
        d.set("water_depth", info.water_depth);
        d.set("temperature", &GString::from(info.temperature.as_str()));
        let to_arr = |v: &[String]| {
            let mut a = PackedStringArray::new();
            for s in v {
                a.push(s.as_str());
            }
            a
        };
        d.set("minerals", &to_arr(&info.minerals));
        d.set("plants", &to_arr(&info.plants));
        d.set("items", &to_arr(&info.items));
        d.set("structures", &to_arr(&info.structures));
        let mut notes = to_arr(&info.notes);
        if let Some(un) = Self::upper_house_note(w, TilePos::new(x, y, z)) {
            notes.push(un.as_str());
        }
        // Who built it, from the chronicle's built record (#292 item 2, WP-08 8.6).
        if let Some(s) = self.session.as_ref() {
            if let Some(b) = hf_sim::view::built::line(w, &s.chronicle, TilePos::new(x, y, z)) {
                notes.push(b.as_str());
            }
        }
        d.set("notes", &notes);
        d
    }

    /// Upper-storey note for `site_note` and `inspect_block` when `p` lies in or on the wall of a
    /// house's upper room (#483).
    fn upper_house_note(w: &World, p: TilePos) -> Option<String> {
        for c in &w.communities {
            for o in &c.house_sites {
                let plan = hf_sim::houses::plan(w, *o);
                if plan.upper_floor.is_empty() {
                    continue;
                }
                let ring = hf_sim::houses::walls(&plan.upper_floor);
                if !plan.upper_floor.contains(&p) && !ring.iter().any(|(q, _)| *q == p) {
                    continue;
                }
                let surf = w.grid.ground_z(p.x, p.y);
                let above = (p.z - (surf + 1)).max(1);
                let walls = ring
                    .iter()
                    .filter(|(q, _)| w.grid.get(*q).shape == hf_sim::world::Shape::Wall)
                    .count();
                let edges = ring.len();
                let roof_targets: Vec<TilePos> = plan
                    .upper_floor
                    .iter()
                    .copied()
                    .filter(|q| Some(q.offset(0, 0, -1)) != plan.upper_stair)
                    .collect();
                let roofs = roof_targets
                    .iter()
                    .filter(|q| w.roofs.contains_key(q))
                    .count();
                if walls < edges {
                    return Some(format!(
                        "Upper storey ({above} above ground): walls {walls} of {edges}."
                    ));
                }
                if roofs < roof_targets.len() {
                    return Some(format!(
                        "Upper storey ({above} above ground): walls up, roof {roofs} of {}.",
                        roof_targets.len()
                    ));
                }
                return Some(format!("Upper storey ({above} above ground), roofed."));
            }
        }
        None
    }

    #[func]
    fn tick(&self) -> i64 {
        self.world().map(|w| w.tick() as i64).unwrap_or(0)
    }

    #[func]
    fn world_seed(&self) -> i64 {
        self.world().map(|w| w.meta.seed as i64).unwrap_or(0)
    }

    /// Read-only sampled-state identity for viewer timelapse metadata (#484).
    #[func]
    fn state_digest(&self) -> GString {
        GString::from(
            self.world()
                .map(hf_sim::digest::digest)
                .unwrap_or_default()
                .as_str(),
        )
    }

    /// Every material's name as an icon suffix, indexed by material id ("oak", "white_harts_hide"):
    /// the client draws `items32/<kind>_<suffix>.png` when that variant exists (C32).
    #[func]
    fn material_slugs(&self) -> PackedStringArray {
        self.world()
            .map(|w| {
                w.grid
                    .materials
                    .defs
                    .iter()
                    .map(|d| GString::from(hf_sim::items::slug(&d.name).as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The valley's ending, once nobody of any community is left in it (C60): one line per
    /// community, how it ended. Empty until then.
    #[func]
    fn ending(&self) -> GString {
        GString::from(
            self.world()
                .map(hf_sim::systems::moving::ending)
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn world_name(&self) -> GString {
        GString::from(self.world().map(|w| w.meta.name.as_str()).unwrap_or(""))
    }

    /// Ten survivors to choose from. `rerolls` are this session's per-slot counters, so a card
    /// changes only when its own slot is rerolled. This is what the setup screen shows.
    #[func]
    fn survivor_cards(&self, seed: i64, rerolls: PackedInt64Array) -> Array<VarDictionary> {
        let counts: Vec<u32> = (0..hf_sim::opening::PARTY)
            .map(|i| rerolls.get(i).unwrap_or(0) as u32)
            .collect();
        let out: Array<VarDictionary> = (0..hf_sim::opening::PARTY as u8)
            .map(|i| {
                let it = hf_sim::opening::identity(seed as u64, i, counts[i as usize]);
                let mut best: Vec<(String, f32)> = it
                    .skills
                    .states
                    .iter()
                    .zip(hf_sim::mind::skills::ALL.iter())
                    .map(|(st, s)| (format!("{s:?}"), st.level))
                    .collect();
                best.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let top: Array<GString> = best
                    .iter()
                    .take(3)
                    .map(|(n, l)| GString::from(format!("{n} {:.0}", l.floor()).as_str()))
                    .collect();
                let traits = it
                    .dispositions
                    .surface_traits(&it.needs, 0.0, 0.0, 0.0)
                    .iter()
                    .map(|t| t.label)
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut d = VarDictionary::new();
                d.set(&GString::from("index"), i as i64);
                d.set(
                    &GString::from("name"),
                    &GString::from(it.name.as_str()).to_variant(),
                );
                d.set(&GString::from("age"), it.starting_age_years as i64);
                d.set(
                    &GString::from("traits"),
                    &GString::from(traits.as_str()).to_variant(),
                );
                d.set(&GString::from("skills"), &top.to_variant());
                // A few lines of who they were: answers first, then the hard things, then work.
                let life = hf_sim::past::life(seed as u64, i, counts[i as usize], &it.traits);
                let mut lines: Vec<(u8, String)> = life
                    .events
                    .iter()
                    .filter_map(|e| {
                        use hf_sim::past::Formative as F;
                        let rank = match &e.what {
                            F::Answered { .. } => 0,
                            F::LeftBehind { .. } => 1,
                            w if w.is_trauma() => 2,
                            F::Apprenticed { .. } | F::FoughtFire | F::KindElder { .. } => 3,
                            _ => return None,
                        };
                        Some((rank, hf_sim::past::narrate(&it.name, e.age, &e.what)))
                    })
                    .collect();
                lines.sort_by_key(|l| l.0);
                let past: Array<GString> = lines
                    .iter()
                    .take(3)
                    .map(|(_, t)| GString::from(t.as_str()))
                    .collect();
                d.set(&GString::from("past"), &past.to_variant());
                d.set(
                    &GString::from("home"),
                    &GString::from(life.home.as_str()).to_variant(),
                );
                d
            })
            .collect();
        out
    }

    /// Begin the arrival: these three come through with whatever the cards showed, the rest are
    /// scattered, this Engine paces it. `rerolls` are the ten per-slot counters from the screen.
    #[func]
    fn opening(
        &mut self,
        seed: i64,
        chosen: PackedInt64Array,
        rerolls: PackedInt64Array,
        engine: GString,
    ) -> GString {
        let three: Vec<u8> = (0..3)
            .map(|k| chosen.get(k).unwrap_or(k as i64) as u8)
            .collect();
        let picked: [u8; hf_sim::opening::STARTING] = [three[0], three[1], three[2]];
        let mut counts = [0u32; hf_sim::opening::PARTY];
        for (slot, count) in counts.iter_mut().enumerate() {
            *count = rerolls.get(slot).unwrap_or(0) as u32;
        }
        let engine = match engine.to_string().parse::<hf_sim::engine::Engine>() {
            Ok(e) => e,
            Err(e) => return GString::from(e.as_str()),
        };
        // The deity is chosen before the world is made: it watches the valley's past too.
        let mut setup = hf_sim::setup::WorldSetup::new(seed as u64);
        setup.chosen = picked;
        setup.rerolls = counts;
        setup.engine = engine;
        self.attach(World::from_setup(&setup));
        GString::from("")
    }

    /// The deities a new world can be watched by: key (the old Engine name, which saves keep),
    /// name, title, and what each notices and how it answers.
    #[func]
    fn deities(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        for e in hf_sim::engine::Engine::ALL {
            let d = hf_sim::deity::of(e);
            let mut v = VarDictionary::new();
            v.set("key", format!("{e:?}").to_ascii_lowercase());
            v.set("name", d.name);
            v.set("title", d.title);
            v.set("about", hf_sim::deity::about(e));
            out.push(&v);
        }
        out
    }

    /// The history depths offered, in years before the portal, and the default.
    #[func]
    fn history_choices(&self) -> PackedInt64Array {
        hf_sim::setup::HISTORY_CHOICES
            .iter()
            .map(|y| *y as i64)
            .collect()
    }

    /// The topics founders can hold in common, each with the words for either side.
    #[func]
    fn ideology_topics(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        for t in hf_sim::past::Topic::ALL {
            let mut v = VarDictionary::new();
            v.set("key", t.name());
            v.set("against", hf_sim::setup::held_words(t, -1.0));
            v.set("for", hf_sim::setup::held_words(t, 1.0));
            out.push(&v);
        }
        out
    }

    /// The valley a seed would make, before anything is simulated. `character` is
    /// [wetness, warmth, stoniness] each -1..1, or empty to draw it from the seed. Returns
    /// lines (character and biome mix), width, height, columns (biome index per column, 255
    /// for water), tiles (the packed tile seen from above per column, as `level`), heights
    /// (ground level per column), plants (PlantKind + 1 per column, 0 for none), the founding
    /// site, region, label and character. Waits for the valley to generate; the wizard uses
    /// `valley_preview_begin` and `valley_preview_take` instead.
    #[func]
    fn valley_preview(
        &mut self,
        seed: i64,
        character: PackedFloat32Array,
        region: i64,
    ) -> VarDictionary {
        let m = self.previews.map(seed as u64);
        let region = (region >= 0).then_some(region as usize);
        preview_dict(&hf_sim::setup::preview_in(
            seed as u64,
            &m,
            character_arg(&character),
            region,
        ))
    }

    /// Start making the valley preview `valley_preview` returns on a thread of its own (#560),
    /// unless it is made or being made. True when it is ready for `valley_preview_take`.
    #[func]
    fn valley_preview_begin(
        &mut self,
        seed: i64,
        character: PackedFloat32Array,
        region: i64,
    ) -> bool {
        self.previews
            .begin(seed as u64, character_arg(&character), region)
    }

    /// The preview `valley_preview_begin` started, once it is made (handed over once); an
    /// empty dictionary until then.
    #[func]
    fn valley_preview_take(
        &mut self,
        seed: i64,
        character: PackedFloat32Array,
        region: i64,
    ) -> VarDictionary {
        self.previews
            .take(seed as u64, character_arg(&character), region)
            .map(|p| preview_dict(&p))
            .unwrap_or_default()
    }

    /// The character a valley has (character, label): the one given, else its region's climate,
    /// without generating the valley.
    #[func]
    fn valley_character(
        &mut self,
        seed: i64,
        character: PackedFloat32Array,
        region: i64,
    ) -> VarDictionary {
        let m = self.previews.map(seed as u64);
        let region = (region >= 0).then_some(region as usize);
        let c = character_arg(&character)
            .unwrap_or_else(|| hf_sim::setup::region_character(seed as u64, &m, region));
        let mut v = VarDictionary::new();
        v.set("label", c.label().as_str());
        v.set(
            "character",
            &PackedFloat32Array::from([c.wetness, c.warmth, c.stoniness].as_slice()),
        );
        v
    }

    /// The world a seed makes, for the wizard to choose the valley's region on: size (regions
    /// a side), rgb (size x size x 3) and the region the seed would choose.
    #[func]
    fn world_preview(&mut self, seed: i64) -> VarDictionary {
        let m = self.previews.map(seed as u64);
        let mut v = VarDictionary::new();
        v.set("size", m.size as i64);
        v.set(
            "rgb",
            &PackedByteArray::from(hf_sim::realm::map::rgb(&m).as_slice()),
        );
        v.set(
            "region",
            hf_sim::setup::resolve_region(seed as u64, &m, None) as i64,
        );
        v
    }

    /// What a region of a seed's world is like, and whether people could live there.
    #[func]
    fn region_info(&mut self, seed: i64, region: i64) -> VarDictionary {
        let m = self.previews.map(seed as u64);
        let mut v = VarDictionary::new();
        if region < 0 || region as usize >= m.size * m.size {
            v.set("habitable", false);
            return v;
        }
        let i = region as usize;
        v.set("habitable", hf_sim::realm::embark::habitable(&m, i));
        let lines: PackedStringArray = hf_sim::setup::region_lines(&m, i)
            .iter()
            .map(|l| GString::from(l.as_str()))
            .collect();
        v.set("lines", &lines);
        v
    }

    /// The world ages the wizard offers, in years before the portal.
    #[func]
    fn realm_year_choices(&self) -> PackedInt64Array {
        hf_sim::setup::REALM_YEAR_CHOICES
            .iter()
            .map(|&y| y as i64)
            .collect()
    }

    /// The species library for the wizard's peoples page (P5): key, name, plural, plan, its own
    /// rarity and the line that explains it.
    #[func]
    fn species_list(&self) -> VarArray {
        let mut out = VarArray::new();
        for sp in hf_sim::species::library() {
            let mut d = VarDictionary::new();
            d.set("key", &GString::from(sp.id.as_str()));
            d.set("name", &GString::from(sp.name.as_str()));
            d.set("plural", &GString::from(sp.plural.as_str()));
            d.set("plan", &GString::from(sp.plan.as_str()));
            d.set("rarity", &GString::from(sp.rarity.as_str()));
            d.set("describe", &GString::from(sp.describe().as_str()));
            out.push(&d.to_variant());
        }
        out
    }

    /// The peoples a seed and the page's choices would draw, before any history runs (P5):
    /// per people its name, species, way of living and rarity. Only the world map and the draw
    /// are made, so it is quick enough to follow every change on the page.
    #[func]
    fn peoples_preview(&mut self, seed: i64, peoples: VarDictionary) -> VarArray {
        let mut out = VarArray::new();
        let choice = peoples_from(&peoples);
        if choice.check().is_err() {
            return out;
        }
        let m = self.previews.map(seed as u64);
        let generated = hf_sim::species::generate(
            seed as u64,
            hf_sim::species::generated_count(&choice.generated),
        );
        let refs: Vec<&hf_sim::species::SpeciesDef> = generated.iter().collect();
        for p in hf_sim::realm::peoples::peoples_with(seed as u64, &m, &choice, &refs) {
            let sp = if p.species.0 >= hf_sim::species::GENERATED {
                refs[(p.species.0 - hf_sim::species::GENERATED) as usize]
            } else {
                hf_sim::species::get(p.species)
            };
            let mut d = VarDictionary::new();
            d.set("name", &GString::from(p.name.as_str()));
            d.set("species", &GString::from(sp.name.as_str()));
            d.set("living", &GString::from(p.living.label()));
            d.set("rarity", &GString::from(p.rarity.as_str()));
            out.push(&d.to_variant());
        }
        out
    }

    /// Begin a world from the wizard's choices. Keys: seed, region (of the world map, -1 for
    /// the seed's), realm_years (the world's age), deity (key from `deities`),
    /// history_years, character ([w, w, s] or empty), chosen (three slots), rerolls (ten
    /// counters), ideology ({topic key: stance -1..1}). Returns "" or what was wrong.
    #[func]
    fn begin(&mut self, setup: VarDictionary) -> GString {
        self.settle();
        let seed = setup
            .get("seed")
            .and_then(|v| v.try_to::<i64>().ok())
            .unwrap_or(1);
        let mut s = hf_sim::setup::WorldSetup::new(seed as u64);
        if let Some(r) = setup.get("region").and_then(|v| v.try_to::<i64>().ok()) {
            s.region = (r >= 0).then_some(r as usize);
        }
        if let Some(y) = setup
            .get("realm_years")
            .and_then(|v| v.try_to::<i64>().ok())
        {
            s.realm_years =
                (y.max(0) as u64).min(*hf_sim::setup::REALM_YEAR_CHOICES.last().unwrap());
        }
        if let Some(d) = setup.get("deity").and_then(|v| v.try_to::<GString>().ok()) {
            match d.to_string().parse::<hf_sim::engine::Engine>() {
                Ok(e) => s.engine = e,
                Err(e) => return GString::from(e.as_str()),
            }
        }
        if let Some(y) = setup
            .get("history_years")
            .and_then(|v| v.try_to::<i64>().ok())
        {
            s.history_years = y.max(0) as u64;
        }
        if let Some(c) = setup
            .get("character")
            .and_then(|v| v.try_to::<PackedFloat32Array>().ok())
            .filter(|c| c.len() == 3)
        {
            s.character = Some(hf_sim::world::gen::biome::Character {
                wetness: c[0].clamp(-1.0, 1.0),
                warmth: c[1].clamp(-1.0, 1.0),
                stoniness: c[2].clamp(-1.0, 1.0),
            });
        }
        if let Some(c) = setup
            .get("chosen")
            .and_then(|v| v.try_to::<PackedInt64Array>().ok())
        {
            for (k, slot) in s.chosen.iter_mut().enumerate() {
                *slot = c.get(k).unwrap_or(k as i64).clamp(0, 9) as u8;
            }
        }
        if let Some(r) = setup
            .get("rerolls")
            .and_then(|v| v.try_to::<PackedInt64Array>().ok())
        {
            for (k, count) in s.rerolls.iter_mut().enumerate() {
                *count = r.get(k).unwrap_or(0).max(0) as u32;
            }
        }
        if let Some(p) = setup
            .get("peoples")
            .and_then(|v| v.try_to::<VarDictionary>().ok())
        {
            s.peoples = peoples_from(&p);
            if let Err(e) = s.peoples.check() {
                return GString::from(e.as_str());
            }
        }
        if let Some(held) = setup
            .get("ideology")
            .and_then(|v| v.try_to::<VarDictionary>().ok())
        {
            for (k, v) in held.iter_shared() {
                let Some(t) = hf_sim::past::Topic::from_name(&k.to_string()) else {
                    return GString::from(format!("unknown topic {k}").as_str());
                };
                let stance = v.try_to::<f64>().unwrap_or(0.0).clamp(-1.0, 1.0) as f32;
                if stance != 0.0 {
                    s.ideology.push((t, stance));
                }
            }
            s.ideology.sort_by_key(|(t, _)| *t);
        }
        self.attach(World::from_setup(&s));
        GString::from("")
    }

    /// One line saying what an Engine does, in words a player can read on the setup screen.
    #[func]
    fn engine_blurb(&self, engine: GString) -> GString {
        match engine.to_string().parse::<hf_sim::engine::Engine>() {
            Ok(e) => GString::from(e.blurb()),
            Err(e) => GString::from(e.as_str()),
        }
    }

    /// What the Engine is asking of the generators right now: its name and the colony's strain.
    /// The community's strain 0..1 (hf_sim::engine::tension); the music director reads it.
    #[func]
    fn tension(&self) -> f32 {
        // Strain on the people, or troubles piling up (arcs.rs), whichever is worse.
        self.world()
            .map(|w| hf_sim::engine::tension(w).max(hf_sim::arcs::weight(w)))
            .unwrap_or(0.0)
    }

    /// The troubles going on now, as one line for the screen ("" when none).
    #[func]
    fn arc_line(&self) -> GString {
        self.world()
            .and_then(hf_sim::arcs::status_line)
            .map(|s| GString::from(s.as_str()))
            .unwrap_or_default()
    }

    /// The watching deity's name, for the HUD ("Aun"). #1740: the HUD shows only this unless
    /// `--debug-hud`; the engine's strain is the storyteller's hidden reading (#1560).
    #[func]
    fn deity_name(&self) -> GString {
        match self.world() {
            Some(w) => GString::from(hf_sim::deity::of(w.meta.engine).name),
            None => GString::new(),
        }
    }

    /// The engine's tension, 0..1, for `--debug-hud` (#1740).
    #[func]
    fn engine_strain(&self) -> f32 {
        self.world().map(hf_sim::engine::tension).unwrap_or(0.0)
    }

    #[func]
    fn engine_summary(&self) -> GString {
        match self.world() {
            Some(w) => {
                let t = hf_sim::engine::tension(w);
                let d = hf_sim::deity::of(w.meta.engine);
                GString::from(format!("{} watches  strain {:.0}%", d.name, t * 100.0).as_str())
            }
            None => GString::from(""),
        }
    }

    #[func]
    fn size(&self) -> Vector3i {
        match self.world() {
            Some(w) => Vector3i::new(w.grid.width, w.grid.height, w.grid.z_max - w.grid.z_min + 1),
            None => Vector3i::ZERO,
        }
    }

    #[func]
    fn z_range(&self) -> Vector2i {
        match self.world() {
            Some(w) => Vector2i::new(w.grid.z_min, w.grid.z_max),
            None => Vector2i::ZERO,
        }
    }

    #[func]
    fn settlement_centre(&self) -> Vector3i {
        match self.world() {
            Some(w) => {
                let c = settlement_centre(w);
                Vector3i::new(c.x, c.y, c.z)
            }
            None => Vector3i::ZERO,
        }
    }

    #[func]
    fn founding_site(&self) -> Vector3i {
        match self.world() {
            Some(w) => Vector3i::new(
                w.meta.founding_site.x,
                w.meta.founding_site.y,
                w.meta.founding_site.z,
            ),
            None => Vector3i::ZERO,
        }
    }

    /// Packed tiles (material | shape<<16 | flags<<24) for a whole z-level, row-major.
    #[func]
    fn level_tiles(&self, z: i32) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            let g = &w.grid;
            out.resize((g.width * g.height) as usize);
            let mut i = 0;
            for y in 0..g.height {
                for x in 0..g.width {
                    out[i] = g.get(TilePos::new(x, y, z)).pack() as i32;
                    i += 1;
                }
            }
        }
        out
    }

    /// Stage 3's opt-in people/animal batch: id, interpolated x/y in tiles, z, species, facing,
    /// pose and movement progress. The immutable snapshot is reduced on rayon workers; Godot
    /// only receives the compact result. The GDScript layered path remains the default.
    #[func]
    fn render_batches(&self, z: i32) -> PackedFloat32Array {
        let mut out = PackedFloat32Array::new();
        let Some(w) = self.world() else {
            return out;
        };
        let batch = world_renderer::instances(w, self.bodies_view(), z);
        out.resize(batch.len() * world_renderer::INSTANCE_STRIDE);
        for (n, i) in batch.iter().enumerate() {
            let at = n * world_renderer::INSTANCE_STRIDE;
            for (offset, value) in [
                i.id as f32,
                i.x,
                i.y,
                i.z as f32,
                i.species as f32,
                i.facing as f32,
                i.pose as f32,
                i.progress,
            ]
            .into_iter()
            .enumerate()
            {
                out[at + offset] = value;
            }
        }
        out
    }

    /// Stage 4's opt-in static batches: category, id, x, y, z, kind, material/variant,
    /// progress/active or item count. Rust builds the stable rows; the client only uploads them.
    #[func]
    fn render_static_batches(&self, z: i32) -> PackedFloat32Array {
        let mut out = PackedFloat32Array::new();
        let Some(w) = self.world() else {
            return out;
        };
        let batch = world_renderer::static_instances(w, z);
        out.resize(batch.len() * world_renderer::STATIC_STRIDE);
        for (n, i) in batch.iter().enumerate() {
            let at = n * world_renderer::STATIC_STRIDE;
            for (offset, value) in [
                i.category as f32,
                i.id as f32,
                i.x as f32,
                i.y as f32,
                i.z as f32,
                i.kind as f32,
                i.variant_or_material as f32,
                i.progress_or_active,
            ]
            .into_iter()
            .enumerate()
            {
                out[at + offset] = value;
            }
        }
        out
    }

    /// Stage 4's packed ground rows: x, y, z, tile pack and column ground z. Tile packing stays
    /// integer-valued across the bridge; the optional renderer uses x/y for its first batch.
    #[func]
    fn render_ground_batches(&self, z: i32) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        let Some(w) = self.world() else {
            return out;
        };
        let batch = world_renderer::ground_instances(w, z);
        out.resize(batch.len() * world_renderer::GROUND_STRIDE);
        for (n, i) in batch.iter().enumerate() {
            let at = n * world_renderer::GROUND_STRIDE;
            for (offset, value) in [i.x, i.y, i.z, i.packed, i.ground_z]
                .into_iter()
                .enumerate()
            {
                out[at + offset] = value;
            }
        }
        out
    }

    /// One number per 32×32 chunk column (row-major over the map), the sum of the chunk
    /// versions from `z_lo` to `z_hi`: it changes whenever any tile the client draws for that
    /// column changes, so the client redraws only the cached ground chunks that need it.
    #[func]
    fn chunk_versions(&self, z_lo: i32, z_hi: i32) -> PackedInt64Array {
        let mut out = PackedInt64Array::new();
        if let Some(w) = self.world() {
            let g = &w.grid;
            let cs = hf_sim::world::chunk::CHUNK_SIZE as i32;
            let cw = (g.width + cs - 1) / cs;
            let ch = (g.height + cs - 1) / cs;
            out.resize((cw * ch) as usize);
            for (&(cx, cy, cz), c) in &g.chunks {
                if cz < z_lo || cz > z_hi || cx < 0 || cy < 0 || cx >= cw || cy >= ch {
                    continue;
                }
                let i = (cy * cw + cx) as usize;
                out[i] = out[i].wrapping_add(c.version as i64 + 1);
            }
        }
        out
    }

    #[func]
    fn ground(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            out.resize(w.grid.ground.len());
            for (i, g) in w.grid.ground.iter().enumerate() {
                out[i] = *g;
            }
        }
        out
    }

    /// Plants as [x, y, z, kind, growth*100, variant] groups of 6.
    #[func]
    fn plants(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for p in &w.plants {
                if !p.alive {
                    continue;
                }
                for v in [
                    p.pos.x,
                    p.pos.y,
                    p.pos.z,
                    p.kind as i32,
                    (p.growth * 100.0) as i32,
                    (p.variant % 1000) as i32,
                ] {
                    out.push(v);
                }
            }
        }
        out
    }

    /// How each species of this world is drawn (P3; the generated ones too, P6), by species id:
    /// its body sheet set, its body plan (whose clothing, hair and equipment sheets it wears,
    /// unless its body set has a layer set of its own: the Hollin, Murrow and Seamfolk rigs of
    /// C53, chosen in entity_layer.gd) and whether it has hair.
    #[func]
    fn species_art(&self) -> VarDictionary {
        let mut out = VarDictionary::new();
        let kinds = match self.world() {
            Some(w) => w.kinds(),
            None => hf_sim::species::library()
                .iter()
                .enumerate()
                .map(|(i, d)| (hf_sim::species::SpeciesId(i as u16), d))
                .collect(),
        };
        for (id, sp) in kinds {
            let mut d = VarDictionary::new();
            d.set("body", &GString::from(sp.art.body.as_str()));
            d.set("plan", &GString::from(sp.plan.as_str()));
            d.set("hair", sp.art.hair);
            d.set("name", &GString::from(sp.name.as_str()));
            out.set(id.0 as i64, &d);
        }
        out
    }

    /// Agents, 24 ints each (the 23rd, P3, the person's species in the library; the 24th, #324,
    /// from_z): id, x, y, z,
    /// species, facing, body, skin, hair, hair_colour,
    /// clothing, clothing_colour, from_x, from_y, progress*1000, pose, child(0/1), injured,
    /// elder, equipment, work sound, band (0 none, 1 one of a band of strangers, 2 out
    /// raiding), people, from_z, emote (0 none, else an item kind + 1: the activity's tool,
    /// #192) (pose: 0 idle, 1 walking, 2 working, 3 carrying, 4 sleeping, 5 sitting, 6 fighting,
    /// 7 struck, 8 falling, 9 lying, 10 talking, 11 eating).
    #[func]
    fn agents(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for a in &w.agents {
                // (A bear asleep in its den for the winter is not seen, nor the lost nobody
                // has found: view::known, #107.)
                // The dead lie where they fell for a while (#185, view::bodies): fall, then lying.
                let body = if a.alive || a.species != hf_sim::agents::Species::Person {
                    None
                } else {
                    self.bodies_view().pose(a.id, w.meta.tick)
                };
                if (!a.alive && body.is_none())
                    || a.denned
                    || hf_sim::view::known::is_unfound(w, a.id)
                {
                    continue;
                }
                for v in agent_row(w, a, body) {
                    out.push(v);
                }
            }
        }
        out
    }

    /// Relationship lines (#192, WP-08 8.4): 3 ints each — from_row, to_row, band
    /// (1 friend, 2 likes, 3 knows, 4 dislikes; the same thresholds as the inspect
    /// panel's "people:" line) — for the client overlay that draws who is bound to
    /// whom. Row numbers are positions in `agents()` output.
    #[func]
    fn relationship_lines(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for v in relationship_lines_core(w, self.bodies_view()) {
                out.push(v);
            }
        }
        out
    }

    /// Work nobody has taken (#292, WP-08 8.6): orders waiting an hour or more and held by
    /// nobody, oldest first, at most 20, from `hf_sim::view::waiting::stuck`. Each a dictionary:
    /// label, reason, x, y, z, hours.
    #[func]
    fn stuck_orders(&self) -> VarArray {
        let mut out = VarArray::new();
        if let Some(w) = self.world() {
            for s in hf_sim::view::waiting::stuck(w, hf_sim::time::TICKS_PER_HOUR, 20) {
                let mut d = VarDictionary::new();
                d.set("label", &GString::from(s.label.as_str()));
                d.set("reason", &GString::from(s.reason.as_str()));
                d.set("x", s.pos.x);
                d.set("y", s.pos.y);
                d.set("z", s.pos.z);
                d.set("hours", (s.age / hf_sim::time::TICKS_PER_HOUR) as i64);
                out.push(&d);
            }
        }
        out
    }

    /// Built walls, doors and floors at level `z` with what they were made of (C56), 3 ints
    /// each: x, y, material (an id into `material_slugs`).
    #[func]
    fn built_of(&self, z: i32) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for p in w.built_of.keys().filter(|p| p.z == z) {
                if let Some(m) = w.made_of(*p) {
                    for v in [p.x, p.y, m.0 as i32] {
                        out.push(v);
                    }
                }
            }
        }
        out
    }

    /// Roofs at level `z` (building.rs), 5 ints each: x, y, material, quality*100, snow hours.
    #[func]
    fn roofs(&self, z: i32) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for (p, r) in w.roofs.iter().filter(|(p, _)| p.z == z) {
                for v in [
                    p.x,
                    p.y,
                    r.material.0 as i32,
                    (r.quality * 100.0) as i32,
                    r.snow as i32,
                ] {
                    out.push(v);
                }
            }
        }
        out
    }

    /// What the house at this tile has come to ("" if none): walls, roof, snow on it.
    #[func]
    fn site_note(&self, x: i32, y: i32, z: i32) -> GString {
        self.world()
            .and_then(|w| {
                let p = hf_sim::ids::TilePos::new(x, y, z);
                hf_sim::thin::note(w, p)
                    .or_else(|| hf_sim::engrave::note(w, p))
                    .or_else(|| hf_sim::project::note(w, p))
                    .or_else(|| hf_sim::legend::ground_note(w, p))
                    .or_else(|| hf_sim::items::stack_note(w, p))
                    .or_else(|| hf_sim::structures::note(w, p))
                    .or_else(|| hf_sim::deep::note(w, p))
                    .or_else(|| hf_sim::building::house_note(w, p))
                    .or_else(|| Self::upper_house_note(w, p))
            })
            .map(|s| GString::from(s.as_str()))
            .unwrap_or_default()
    }

    /// Bands of strangers, 9 ints each: id, camp x, y, z, state (0 approaching, 1 camped,
    /// 2 leaving), sighted (0/1), welcomed (0/1), members here, taken.
    #[func]
    fn bands(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for b in &w.bands {
                let here = b
                    .members
                    .iter()
                    .filter(|m| w.agents.iter().any(|a| a.id == **m && a.alive))
                    .count();
                let state = match b.state {
                    hf_sim::threat::BandState::Approaching => 0,
                    hf_sim::threat::BandState::Camped(_) => 1,
                    hf_sim::threat::BandState::Leaving => 2,
                };
                for v in [
                    b.id as i32,
                    b.camp.x,
                    b.camp.y,
                    b.camp.z,
                    state,
                    b.sighted as i32,
                    b.welcomed as i32,
                    here as i32,
                    b.taken as i32,
                ] {
                    out.push(v);
                }
            }
        }
        out
    }

    /// One plain line per band the settlement knows about, for the screen.
    #[func]
    fn band_lines(&self) -> PackedStringArray {
        let mut out = PackedStringArray::new();
        if let Some(w) = self.world() {
            for b in &w.bands {
                if let Some(l) = hf_sim::threat::status_line(w, b) {
                    out.push(&GString::from(l.as_str()));
                }
            }
        }
        out
    }

    #[func]
    fn agent_name(&self, id: i32) -> GString {
        self.world()
            .and_then(|w| {
                w.agents
                    .iter()
                    .find(|a| a.id.0 as i32 == id && !hf_sim::view::known::is_unfound(w, a.id))
            })
            .map(|a| GString::from(a.name.as_str()))
            .unwrap_or_default()
    }

    /// Structures, 8 ints each: id, x, y, z, kind, material, active(0/1), progress*100.
    #[func]
    fn structures(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for s in &w.structures {
                for v in [
                    s.id.0 as i32,
                    s.pos.x,
                    s.pos.y,
                    s.pos.z,
                    s.kind as i32,
                    s.material.0 as i32,
                    s.active as i32,
                    (s.progress * 100.0) as i32,
                ] {
                    out.push(v);
                }
            }
        }
        out
    }

    /// The old hamlets' house walls (#202, deep::ruins), 4 ints each: x, y, z, standing (1 a
    /// ruined wall still up, 0 where one fell and its stones lie).
    #[func]
    fn ruins(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for (p, standing) in hf_sim::deep::ruins(w) {
                for v in [p.x, p.y, p.z, standing as i32] {
                    out.push(v);
                }
            }
        }
        out
    }

    /// Designations, 8 ints each: id, x, y, z, kind code, progress*100, materials_complete,
    /// material (for walls, stairs, floors, roofs and structures; else 0).
    /// kind code: 0 wall, 1 door, 2 floor, 3 structure(+kind*100), 4 chop, 5 mine, 6 sow,
    /// 7 harvest, 8 haul, 9 cook, 10 dug stairs, 11 hunt, 12 craft, 13 tend, 14 douse,
    /// 15 beat out, 16 roof, 17 clear roof, 18 study, 19 dig clay or sand, 20 carve a wall,
    /// 21 fish, 22 snare, 23 tame, 24 fetch water, 25 built stairs, 26 dig a channel, 27 uproot a weed.
    #[func]
    fn designations(&self) -> PackedInt32Array {
        use hf_sim::work::DesignationKind as K;
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for d in &w.designations {
                if d.done {
                    continue;
                }
                let code = match d.kind {
                    K::BuildWall(_) => 0,
                    K::BuildDoor => 1,
                    K::BuildFloor(_) => 2,
                    K::BuildStructure(k, _) => 3 + (k as i32) * 100,
                    K::Chop => 4,
                    K::Mine => 5,
                    K::Sow => 6,
                    K::Harvest => 7,
                    K::Haul => 8,
                    K::Cook => 9,
                    K::DigStairs => 10,
                    K::Hunt(_) => 11,
                    K::Craft(_) => 12,
                    K::Tend(_) => 13,
                    K::Douse => 14,
                    K::BeatOut => 15,
                    K::BuildRoof(_) => 16,
                    K::ClearRoof => 17,
                    K::Study(_) => 18,
                    K::DigEarth => 19,
                    K::Engrave => 20,
                    K::Fish => 21,
                    K::Snare => 22,
                    K::Tame(_) => 23,
                    K::FetchWater => 24,
                    K::BuildStairs(_) => 25,
                    K::DigChannel => 26,
                    K::Uproot => 27,
                };
                let progress = d
                    .work_total
                    .checked_div(1)
                    .filter(|t| *t > 0)
                    .map(|t| ((d.work_total - d.work_left) * 100 / t) as i32)
                    .unwrap_or(0);
                for v in [
                    d.id.0 as i32,
                    d.pos.x,
                    d.pos.y,
                    d.pos.z,
                    code,
                    progress,
                    d.materials_complete() as i32,
                    match d.kind {
                        K::BuildWall(m)
                        | K::BuildStairs(m)
                        | K::BuildFloor(m)
                        | K::BuildRoof(m) => m.0 as i32,
                        K::BuildStructure(_, m) => m.0 as i32,
                        _ => 0,
                    },
                ] {
                    out.push(v);
                }
            }
        }
        out
    }

    /// What each agent carries, as [id, kind, material] triples: the material (an id into
    /// `material_slugs`) picks the same icon in hand as on the ground (#186).
    #[func]
    fn carried(&self) -> PackedInt32Array {
        self.carried_rows(false)
    }

    /// Held item stacks as [carrier, item kind, material, count] for the world renderer.
    #[func]
    fn carried_stacks(&self) -> PackedInt32Array {
        self.carried_rows(true)
    }

    /// Carved walls (C35), 4 ints each: x, y, z, quality tier 0..5.
    #[func]
    fn engravings(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for (p, e) in &w.engravings {
                for v in [
                    p.x,
                    p.y,
                    p.z,
                    hf_sim::items::ItemKind::tier(e.quality) as i32,
                ] {
                    out.push(v);
                }
            }
        }
        out
    }

    /// Ground items, 8 ints each: id, x, y, z, kind, count, material, quality tier 0..5.
    #[func]
    fn items(&self) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(w) = self.world() {
            for it in &w.items {
                if !hf_sim::view::known::item_told(w, it) {
                    continue;
                }
                if let hf_sim::items::ItemPlace::Ground(p) = it.place {
                    for v in [
                        it.id.0 as i32,
                        p.x,
                        p.y,
                        p.z,
                        it.kind as i32,
                        it.count as i32,
                        it.material.0 as i32,
                        hf_sim::items::ItemKind::tier(it.quality) as i32,
                    ] {
                        out.push(v);
                    }
                }
            }
        }
        out
    }

    /// Sky: 0 clear, 1 overcast, 2 rain, 3 snow.
    #[func]
    fn sky(&self) -> i64 {
        self.world()
            .map(|w| match w.weather.sky {
                hf_sim::systems::weather::Sky::Clear => 0,
                hf_sim::systems::weather::Sky::Overcast => 1,
                hf_sim::systems::weather::Sky::Rain => 2,
                hf_sim::systems::weather::Sky::Snow => 3,
                hf_sim::systems::weather::Sky::Fog => 4,
                hf_sim::systems::weather::Sky::Storm => 5,
            })
            .unwrap_or(0)
    }

    /// Season: 0 spring, 1 summer, 2 autumn, 3 winter.
    #[func]
    fn season(&self) -> i64 {
        self.world()
            .map(
                |w| match hf_sim::time::Calendar::from_tick(w.tick()).season {
                    hf_sim::time::Season::Spring => 0,
                    hf_sim::time::Season::Summer => 1,
                    hf_sim::time::Season::Autumn => 2,
                    hf_sim::time::Season::Winter => 3,
                },
            )
            .unwrap_or(0)
    }

    #[func]
    fn temperature(&self) -> f32 {
        self.world()
            .map(|w| w.weather.temperature_c)
            .unwrap_or(10.0)
    }

    #[func]
    fn hour(&self) -> f32 {
        self.world()
            .map(|w| {
                (w.tick() % hf_sim::time::TICKS_PER_DAY) as f32
                    / hf_sim::time::TICKS_PER_HOUR as f32
            })
            .unwrap_or(12.0)
    }

    #[func]
    fn calendar(&self) -> GString {
        match self.world() {
            Some(w) => {
                let c = w.calendar();
                GString::from(
                    format!(
                        "Year {} {:?} day {}  {:02}:{:02}",
                        c.year + 1,
                        c.season,
                        c.day_of_season,
                        c.hour,
                        c.minute
                    )
                    .as_str(),
                )
            }
            None => GString::new(),
        }
    }

    #[func]
    fn daylight(&self) -> f32 {
        self.world().map(|w| w.weather.daylight).unwrap_or(1.0)
    }

    /// "Oakfolk 8 · Reedkin 5 (2 away)" for the HUD (`communities_line`).
    #[func]
    fn communities_summary(&self) -> GString {
        match self.world() {
            Some(w) => GString::from(communities_line(w).as_str()),
            None => GString::new(),
        }
    }

    #[func]
    /// Living people the viewer knows of: the unfound lost are not counted (view::known).
    fn population(&self) -> i64 {
        self.world()
            .map(|w| hf_sim::view::known::people_known(w) as i64)
            .unwrap_or(0)
    }
}

impl SimHost {
    fn summary_range(&mut self, from_tick: i64, to_tick: i64, max_major: usize) -> VarDictionary {
        self.settle();
        let mut d = VarDictionary::new();
        let Some(s) = self.session.as_ref() else {
            return d;
        };
        let to = (to_tick.max(0) as u64).min(s.world.tick());
        match summary::since(
            &s.world,
            &s.chronicle,
            from_tick.max(0) as u64,
            to,
            max_major,
        ) {
            Ok(sum) => {
                let counts_words = sum.counts_in_words();
                let mut counts = VarDictionary::new();
                for (k, n) in sum.counts {
                    counts.set(&GString::from(k.as_str()), n);
                }
                let mut lines = VarArray::new();
                for (_, tick, detail) in sum.major {
                    if let Ok(kind) = serde_json::from_str::<hf_sim::events::EventKind>(&detail) {
                        let e = hf_sim::events::Event {
                            id: hf_sim::ids::EventId(0),
                            tick,
                            at: None,
                            severity: Severity::Major,
                            kind,
                        };
                        let cal = hf_sim::time::Calendar::from_tick(
                            tick.saturating_sub(s.world.meta.epoch),
                        );
                        let text = format!(
                            "Day {} {:02}:{:02}  {}",
                            cal.day_of_year + 1,
                            cal.hour,
                            cal.minute,
                            narrate::line(&s.world, &e)
                        );
                        lines.push(&GString::from(text.as_str()).to_variant());
                    }
                }
                d.set("counts", &counts);
                d.set("counts_words", &GString::from(counts_words.as_str()));
                d.set("lines", &lines);
                d.set("from", from_tick);
                d.set("to", to as i64);
            }
            Err(e) => godot_error!("summary failed: {e}"),
        }
        d
    }

    /// The region map of the current world's seed, generated once (None without a realm).
    fn realm_map_ready(&mut self) -> Option<&hf_sim::realm::RealmMap> {
        self.settle();
        let seed = self.session.as_ref().map(|s| s.world.meta.seed)?;
        if self.session.as_ref()?.world.realm.peoples.is_empty() {
            return None;
        }
        if self.realm_map.as_ref().map(|(k, _)| *k) != Some(seed) {
            let m = hf_sim::realm::generate(seed, &hf_sim::realm::RealmParams::default());
            self.realm_map = Some((seed, m));
        }
        self.realm_map.as_ref().map(|(_, m)| m)
    }
}

/// The wizard's peoples choices from its dictionary: exclude (species keys), exclude_plans,
/// count ([least, most]), rarity ({key: rarity}), generated.
fn peoples_from(d: &VarDictionary) -> hf_sim::species::PeopleSetup {
    let mut p = hf_sim::species::PeopleSetup::default();
    if let Some(x) = d
        .get("exclude")
        .and_then(|v| v.try_to::<PackedStringArray>().ok())
    {
        p.exclude = x.as_slice().iter().map(|s| s.to_string()).collect();
    }
    if let Some(x) = d
        .get("exclude_plans")
        .and_then(|v| v.try_to::<PackedStringArray>().ok())
    {
        p.exclude_plans = x.as_slice().iter().map(|s| s.to_string()).collect();
    }
    if let Some(c) = d
        .get("count")
        .and_then(|v| v.try_to::<PackedInt64Array>().ok())
    {
        if c.len() == 2 {
            p.count = [c[0].clamp(0, 255) as u8, c[1].clamp(0, 255) as u8];
        }
    }
    if let Some(r) = d
        .get("rarity")
        .and_then(|v| v.try_to::<VarDictionary>().ok())
    {
        for (k, v) in r.iter_shared() {
            p.rarity.push((k.to_string(), v.to_string()));
        }
        p.rarity.sort();
    }
    if let Some(g) = d.get("generated").and_then(|v| v.try_to::<GString>().ok()) {
        p.generated = g.to_string();
    }
    p.normalise();
    p
}

/// One person's row for the Favourites and People panels (#413, #351): name, where they are,
/// what they are doing now, alive or dead, and which community they belong to. Someone away is
/// listed as travelling, never as dead: the valley has no word of them yet.
fn person_row(w: &World, aid: hf_sim::ids::AgentId) -> Option<VarDictionary> {
    if hf_sim::view::known::is_unfound(w, aid) {
        return None;
    }
    if let Some(a) = w.agents.iter().find(|a| a.id == aid) {
        let mut row = VarDictionary::new();
        row.set("id", a.id.0 as i32);
        row.set("name", &GString::from(a.name.as_str()));
        row.set("alive", a.alive);
        row.set("away", false);
        row.set("x", a.pos.x);
        row.set("y", a.pos.y);
        row.set("z", a.pos.z);
        let comm = a
            .community
            .and_then(|cid| w.communities.iter().find(|c| c.id == cid))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| {
                if a.species == hf_sim::agents::Species::Person {
                    "wanderer".into()
                } else {
                    format!("{:?}", a.species).to_lowercase()
                }
            });
        row.set("community", &GString::from(comm.as_str()));
        let surf = w.grid.ground_z(a.pos.x, a.pos.y);
        let rel_z = if a.pos.z == surf + 1 {
            "surface".to_string()
        } else if a.pos.z > surf + 1 {
            format!("{} above ground", a.pos.z - (surf + 1))
        } else {
            format!("depth {} below ground", (surf + 1) - a.pos.z)
        };
        let where_str = if !a.alive {
            format!("fell at ({}, {}, z{} — {rel_z})", a.pos.x, a.pos.y, a.pos.z)
        } else if let Some(cav) = w.grid.caverns.iter().find(|c| c.contains(a.pos)) {
            format!(
                "in {} ({}, {}, z{} — {rel_z})",
                cav.name, a.pos.x, a.pos.y, a.pos.z
            )
        } else if a.pos.z <= surf {
            format!(
                "underground ({}, {}, z{} — {rel_z})",
                a.pos.x, a.pos.y, a.pos.z
            )
        } else {
            format!("at ({}, {}, z{} — {rel_z})", a.pos.x, a.pos.y, a.pos.z)
        };
        row.set("where", &GString::from(where_str.as_str()));
        let act_str = if !a.alive {
            match a.death.as_deref() {
                Some(how) if !how.is_empty() => format!("dead ({how})"),
                _ => "dead".into(),
            }
        } else {
            a.activity.describe().to_string()
        };
        row.set("activity", &GString::from(act_str.as_str()));
        let why_str = if a.alive {
            hf_sim::view::why::because(w, a)
        } else {
            String::new()
        };
        let feel_str = if a.alive {
            hf_sim::view::why::feeling(w, a)
        } else {
            String::new()
        };
        row.set("why", &GString::from(why_str.as_str()));
        row.set("feeling", &GString::from(feel_str.as_str()));
        return Some(row);
    } else if let Some(a) = w.overworld.away.iter().find(|a| a.id == aid) {
        let mut row = VarDictionary::new();
        row.set("id", a.id.0 as i32);
        row.set("name", &GString::from(a.name.as_str()));
        // The valley does not know whether someone away still lives (review of #739).
        row.set("alive", true);
        row.set("away", true);
        row.set("x", a.pos.x);
        row.set("y", a.pos.y);
        row.set("z", a.pos.z);
        let comm = a
            .community
            .and_then(|cid| w.communities.iter().find(|c| c.id == cid))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "wanderer".into());
        row.set("community", &GString::from(comm.as_str()));
        let where_str = if let Some((_, where_to, _)) = &a.journey {
            format!("beyond the valley ({where_to})")
        } else {
            "away on an expedition".to_string()
        };
        row.set("where", &GString::from(where_str.as_str()));
        row.set("activity", &GString::from("travelling beyond the valley"));
        row.set("why", &GString::new());
        row.set("feeling", &GString::new());
        return Some(row);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use hf_sim::ids::ItemId;
    use hf_sim::mind::Activity;

    /// FIXLOG 2026-11-14: the HUD counts a community's people away as the People panel does,
    /// everyone of it in another place, not only caravan members.
    #[test]
    fn the_hud_counts_away_as_the_people_panel_does() {
        let mut w = World::opening_world(3);
        let i = w
            .agents
            .iter()
            .position(|a| a.alive && a.community.is_some())
            .expect("a person of a community");
        let cid = w.agents[i].community.expect("a community");
        let name = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .expect("their community")
            .name
            .clone();
        let before = communities_line(&w);
        assert!(!before.contains("away"), "nobody away yet: {before}");
        // Gone on a journey of their own: in another place, in no caravan.
        let gone = w.agents.remove(i);
        w.overworld.away.push(gone);
        let after = communities_line(&w);
        assert!(after.contains(&format!("{name} ")), "{after}");
        assert!(after.contains("(1 away)"), "one of them is away: {after}");
        assert!(!after.contains("journey"), "{after}");
    }

    /// FIXLOG 2026-11-14: a wild animal's death no longer draws the camera director, while a
    /// person's death and a herd animal's still do.
    #[test]
    fn the_director_is_not_sent_to_a_wild_animals_death() {
        use hf_sim::agents::Species;
        let mut w = World::opening_world(11);
        let wild = w
            .agents
            .iter()
            .position(|a| a.alive && a.species != Species::Person && a.kept.is_none())
            .expect("a wild animal");
        let person = w
            .agents
            .iter()
            .position(|a| a.alive && a.species == Species::Person && a.community.is_some())
            .expect("a person");
        let herd = w
            .agents
            .iter()
            .position(|a| a.alive && a.species != Species::Person && a.id != w.agents[wild].id)
            .expect("a second animal");
        let cid = w.agents[person].community;
        w.agents[herd].kept = cid;
        let ids = [w.agents[wild].id, w.agents[person].id, w.agents[herd].id];
        let first = w.events.pending.len();
        for id in ids {
            hf_sim::systems::life::die(&mut w, id, "a test".into());
        }
        let deaths: Vec<_> = w.events.pending[first..]
            .iter()
            .filter(|e| matches!(e.kind, hf_sim::events::EventKind::Died { .. }))
            .cloned()
            .collect();
        assert_eq!(deaths.len(), 3, "three deaths recorded, the wild one too");
        let wild_flags: Vec<bool> = deaths.iter().map(|e| wild_death(&w, e)).collect();
        assert_eq!(
            wild_flags,
            vec![true, false, false],
            "only the wild animal's is held back"
        );

        // Five wild deaths in one place, nobody there: the director stays with the settlement.
        let mut d = Director::default();
        let tick = w.meta.tick;
        let far = deaths[0].clone();
        let many: Vec<_> = (0..5)
            .map(|_| far.clone())
            .filter(|e| !wild_death(&w, e))
            .collect();
        d.observe(tick, &many);
        let home = settlement_centre(&w);
        let sug = d.suggest(tick, home, &[]);
        assert_eq!(
            sug.kind, "settlement",
            "not drawn to the wild deaths: {sug:?}"
        );
    }

    /// #1483: feed rows carry the fold key; two suspicions of one sleeper share it, a death has "".
    #[test]
    fn feed_rows_carry_a_fold_key() {
        let w = World::opening_world(1);
        let people: Vec<_> = w
            .agents
            .iter()
            .filter(|a| a.species == hf_sim::agents::Species::Person)
            .map(|a| a.id)
            .collect();
        let ev = |n: u32, kind: hf_sim::events::EventKind| hf_sim::events::Event {
            id: hf_sim::ids::EventId(n),
            tick: w.meta.tick,
            at: None,
            severity: Severity::Notable,
            kind,
        };
        let s = |n, by| {
            ev(
                n,
                hf_sim::events::EventKind::Suspected {
                    by,
                    of: people[0],
                    because: "the fever".into(),
                },
            )
        };
        let (a, b) = (feed_key(&s(1, people[1])), feed_key(&s(2, people[2])));
        assert!(
            !a.is_empty() && a == b,
            "one sleeper's suspicions share a key: {a} {b}"
        );
        let died = ev(
            3,
            hf_sim::events::EventKind::Died {
                agent: people[1],
                cause: "a fever".into(),
            },
        );
        assert_eq!(feed_key(&died), "", "a death never folds");
    }

    #[test]
    fn a_relationship_held_only_by_the_higher_row_is_still_drawn() {
        let mut w = World::opening_world(485);
        for s in w.survivor_pool.iter_mut() {
            if let hf_sim::opening::SurvivorState::Lost { found, .. } = &mut s.state {
                *found = true;
            }
        }
        for a in w.agents.iter_mut() {
            a.relationships.clear();
        }
        let a = w.agents[0].id;
        w.agents[1].relationships.entry(a).or_default().opinion = 0.5; // only b knows a
        let lines = relationship_lines_core(&w, &hf_sim::view::bodies::Bodies::default());
        assert_eq!(
            lines,
            vec![0, 1, 1],
            "a one-sided pair held by the higher row: {lines:?}"
        );
        // When both hold one, the lower row's view is the colour (D88).
        let b = w.agents[1].id;
        w.agents[0].relationships.entry(b).or_default().opinion = -0.5;
        let lines = relationship_lines_core(&w, &hf_sim::view::bodies::Bodies::default());
        assert_eq!(lines, vec![0, 1, 4], "the lower row's view wins: {lines:?}");
    }

    #[test]
    fn relationship_lines_emit_each_visible_pair_once_with_the_from_side_band() {
        let mut w = World::opening_world(485);
        // The seven scattered starters are unfound (view::known::is_unfound): give every
        // lost survivor a found mark so the whole roster takes rows in this test.
        for s in w.survivor_pool.iter_mut() {
            if let hf_sim::opening::SurvivorState::Lost { found, .. } = &mut s.state {
                *found = true;
            }
        }
        let a = w.agents[0].id;
        let b = w.agents[1].id;
        w.agents[0].relationships.entry(b).or_default().opinion = 0.5; // friend, from a
        w.agents[1].relationships.entry(a).or_default().opinion = -0.5; // dislike, back
        let lines = relationship_lines_core(&w, &hf_sim::view::bodies::Bodies::default());
        assert_eq!(
            lines,
            vec![0, 1, 1],
            "one pair, emitted once, from-side band"
        );
        // A dead person nobody has found takes no row: the next living person's
        // row shifts down, and the line still lands on them.
        w.agents[2].alive = false;
        let c = w.agents[3].id;
        w.agents[1].relationships.entry(c).or_default().opinion = -0.5;
        let lines = relationship_lines_core(&w, &hf_sim::view::bodies::Bodies::default());
        assert!(
            lines.windows(3).any(|t| t == [1, 2, 4]),
            "b(1) to c(now row 2), dislikes band: {lines:?}"
        );
        assert_eq!(lines.len(), 6, "still one triple per pair");
    }

    #[test]
    fn talking_and_eating_have_dedicated_poses_without_changing_priority() {
        let mut w = World::opening_world(485);
        let other = w.agents[1].id;
        {
            let a = &mut w.agents[0];
            a.path.clear();
            a.from = a.pos;
            a.move_ticks = 0;
            a.move_total = 0;
            a.fought_at = 0;
            a.body.wounds.clear();
            a.activity = Activity::Talking {
                with: other,
                ticks_left: 20,
            };
        }
        assert_eq!(person_pose(&w, &w.agents[0], false, None), POSE_TALK);

        w.agents[0].activity = Activity::Eating {
            item: ItemId(1),
            ticks_left: 20,
        };
        assert_eq!(person_pose(&w, &w.agents[0], false, None), POSE_EAT);
        assert_ne!(person_pose(&w, &w.agents[0], false, None), 5);
        w.agents[0].species = hf_sim::agents::Species::Deer;
        assert_eq!(person_pose(&w, &w.agents[0], false, None), 5);
        w.agents[0].species = hf_sim::agents::Species::Person;

        w.agents[0].activity = Activity::Talking {
            with: other,
            ticks_left: 20,
        };
        let pos = w.agents[0].pos;
        w.agents[0].path.push(pos.offset(1, 0, 0));
        w.agents[0].move_total = 1;
        assert_eq!(person_pose(&w, &w.agents[0], false, None), 1);
        assert_eq!(person_pose(&w, &w.agents[0], true, None), 3);
        assert_eq!(person_pose(&w, &w.agents[0], false, Some(9)), 9);
    }

    /// #485: drinking and tending have rows of their own, only for the real activity and
    /// order, with the hands free of the spear and of the builder's bundle.
    #[test]
    fn drinking_and_tending_have_their_own_rows_with_free_hands() {
        let mut w = World::opening_world(485);
        let patient = w.agents[1].id;
        {
            let a = &mut w.agents[0];
            a.path.clear();
            a.from = a.pos;
            a.move_ticks = 0;
            a.move_total = 0;
            a.fought_at = 0;
            a.body.wounds.clear();
            a.weapon = Some(ItemId(1));
            a.activity = Activity::Drinking {
                ticks_left: 20,
                back: None,
            };
        }
        let row = agent_row(&w, &w.agents[0], None);
        assert_eq!(row[15], POSE_DRINK, "drinking has its own pose");
        assert_eq!(row[19], -1, "no spear through a drink");
        w.agents[0].activity = Activity::Idle;
        assert_eq!(
            agent_row(&w, &w.agents[0], None)[15],
            0,
            "idle is not drinking"
        );
        assert_eq!(
            agent_row(&w, &w.agents[0], None)[19],
            7,
            "the spear is back in free hands"
        );

        let id = w.alloc_designation_id();
        let pos = w.agents[0].pos;
        w.designations.push(hf_sim::work::Designation {
            id,
            kind: hf_sim::work::DesignationKind::Tend(patient),
            pos,
            requirements: Vec::new(),
            work_left: 100,
            work_total: 100,
            max_workers: 1,
            priority: 5,
            created: 0,
            failures: 0,
            done: false,
            community: None,
            owner: None,
        });
        w.agents[0].activity = Activity::Working {
            plan: hf_sim::work::Plan {
                designation: id,
                steps: vec![hf_sim::work::Step::Work {
                    designation: id,
                    station: None,
                }],
                cursor: 0,
                label: "tending".into(),
            },
        };
        let row = agent_row(&w, &w.agents[0], None);
        assert_eq!(row[15], 2, "tending is work");
        assert_eq!(row[20], WORK_TEND, "the client is told it is tending");
        assert_eq!(
            row[19], -1,
            "a healer's hands hold neither bundle nor spear"
        );
    }

    #[test]
    fn beast_shaped_eater_emits_animal_row_without_person_equipment() {
        let mut w = World::opening_world(485);
        let id = w.agents[0].id;
        {
            let a = &mut w.agents[0];
            a.path.clear();
            a.from = a.pos;
            a.move_ticks = 0;
            a.move_total = 0;
            a.fought_at = 0;
            a.body.wounds.clear();
            a.weapon = Some(ItemId(1));
            a.activity = Activity::Eating {
                item: ItemId(2),
                ticks_left: 20,
            };
        }
        let person = agent_row(&w, &w.agents[0], None);
        assert_eq!(person.len(), 25);
        assert_eq!(person[4], hf_sim::agents::Species::Person as i32);
        assert_eq!(person[15], POSE_EAT);
        assert_eq!(person[19], -1, "eating still suppresses the spear");
        assert_eq!(person[24], 0, "eating has no work emote");

        w.creatures.shifted.push((id, w.meta.tick + 100));
        let wolf = agent_row(&w, &w.agents[0], None);
        assert_eq!(wolf[4], hf_sim::agents::Species::Wolf as i32);
        assert_eq!(wolf[15], 5, "the wolf row keeps the animal eating pose");
        assert_eq!(wolf[19], -1, "shape change does not restore equipment");
    }

    /// CPU time the calling thread has spent (hf-sim tests/support/cpu.rs `thread_cpu`, #582);
    /// wall time where there is no such clock (Windows).
    #[cfg(unix)]
    fn thread_cpu() -> std::time::Duration {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: clock_gettime writes one timespec through a valid pointer.
        let r = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
        assert_eq!(r, 0, "clock_gettime(CLOCK_THREAD_CPUTIME_ID)");
        std::time::Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
    }

    #[cfg(not(unix))]
    fn thread_cpu() -> std::time::Duration {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed()
    }

    /// True, with a line saying so, when the one-minute load average per core is over 1.0:
    /// a wall-clock budget then measures the machine, not the code (#582; the same rule as
    /// hf-sim's tests/support/cpu.rs `too_busy_for_wall_time`).
    fn too_busy_for_wall_time(test: &str) -> bool {
        let load = std::fs::read_to_string("/proc/loadavg")
            .ok()
            .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok());
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
        match load.map(|l| l / cores) {
            Some(l) if l > 1.0 => {
                eprintln!("{test}: wall-clock budget not checked, load {l:.2} per core is over 1.0 (#582)");
                true
            }
            _ => false,
        }
    }

    #[test]
    fn valley_previews_are_made_off_the_caller_and_handed_over_once() {
        // #560: the wizard asks for a preview and goes on drawing; the preview arrives later,
        // the same as one made in place, and asking again while it is made starts no second.
        let mut jobs = PreviewJobs::default();
        let seed = 3;
        let m = jobs.map(seed);
        let region = hf_sim::realm::embark::default_region(seed, &m) as i64;
        let t = std::time::Instant::now();
        let cpu0 = thread_cpu();
        let ready = jobs.begin(seed, None, region);
        let cpu = thread_cpu().saturating_sub(cpu0);
        let took = t.elapsed();
        assert!(!ready, "nothing is made before it is asked for");
        // The guard (#582): the CPU time of the calling thread, which a busy machine does not
        // inflate, stays under 20 ms (making a valley here takes several hundred), and the preview is
        // made on another thread. The wall-clock reading, which would also catch a begin that
        // waited on a lock, is checked only when the machine is quiet enough for it to mean
        // anything, and never loosened.
        assert!(
            cpu.as_millis() < 20,
            "begin returns at once ({cpu:?} of the calling thread CPU)"
        );
        if !too_busy_for_wall_time("valley_previews_are_made_off_the_caller") {
            assert!(took.as_millis() < 20, "begin returns at once ({took:?})");
        }
        let mut got = None;
        for _ in 0..600 {
            if jobs.begin(seed, None, region) {
                got = jobs.take(seed, None, region);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let got = got.expect("the preview is made within six seconds");
        let (made, on) = *jobs.made_on.lock().unwrap();
        assert_eq!(made, 1, "one preview made, not a second while it was made");
        assert_ne!(
            on,
            Some(std::thread::current().id()),
            "made off the calling thread"
        );
        let direct = hf_sim::setup::preview_in(seed, &m, None, Some(region as usize));
        assert_eq!(got.tiles, direct.tiles);
        assert_eq!(got.founding, direct.founding);
        assert!(jobs.take(seed, None, region).is_none(), "handed over once");
        // Another seed's map replaces the kept one.
        let other = jobs.map(seed + 1);
        assert_eq!(other.size, m.size);
        assert!(!std::sync::Arc::ptr_eq(&other, &m));
        assert!(std::sync::Arc::ptr_eq(&other, &jobs.map(seed + 1)));
    }

    #[test]
    fn upper_house_note_describes_upper_storey_room_and_walls() {
        let mut w = World::generate(48321, hf_sim::world::gen::GenParams::small());
        w.flatten(2, 2, 60, 60, 1);
        w.plants.clear();
        w.meta.founding_site = TilePos::new(20, 20, 2);
        w.meta.tick = 14 * hf_sim::time::TICKS_PER_HOUR;
        w.rebuild_caches();
        w.found_party(4);
        let floor: Vec<TilePos> = (40..=42)
            .flat_map(|y| (40..=42).map(move |x| TilePos::new(x, y, 2)))
            .collect();
        let lower_stair = TilePos::new(40, 40, 2);
        let upper_stair = lower_stair.offset(0, 0, 1);
        let upper_floor: Vec<TilePos> = floor.iter().map(|p| p.offset(0, 0, 1)).collect();
        for p in &floor {
            w.grid.set(
                *p,
                if *p == lower_stair {
                    hf_sim::world::Tile {
                        material: hf_sim::world::Material::WOOD,
                        shape: hf_sim::world::Shape::StairsUp,
                        flags: hf_sim::world::TileFlags::NONE,
                    }
                } else {
                    hf_sim::world::Tile::floor(hf_sim::world::Material::WOOD)
                },
            );
            w.built_of.insert(*p, hf_sim::world::Material::WOOD);
        }
        for (p, _) in hf_sim::houses::walls(&floor) {
            if p == TilePos::new(41, 43, 2) {
                w.grid.set(
                    p,
                    hf_sim::world::Tile {
                        material: hf_sim::world::Material::WOOD,
                        shape: hf_sim::world::Shape::Wall,
                        flags: hf_sim::world::TileFlags(hf_sim::world::TileFlags::DOOR),
                    },
                );
            } else {
                w.grid
                    .set(p, hf_sim::world::Tile::solid(hf_sim::world::Material::WOOD));
            }
            w.built_of.insert(p, hf_sim::world::Material::WOOD);
        }
        for p in &upper_floor {
            w.grid.set(
                *p,
                if *p == upper_stair {
                    hf_sim::world::Tile {
                        material: hf_sim::world::Material::WOOD,
                        shape: hf_sim::world::Shape::StairsDown,
                        flags: hf_sim::world::TileFlags::NONE,
                    }
                } else {
                    hf_sim::world::Tile::floor(hf_sim::world::Material::WOOD)
                },
            );
            w.built_of.insert(*p, hf_sim::world::Material::WOOD);
            if *p != upper_stair {
                w.roofs.insert(
                    *p,
                    hf_sim::building::Roof {
                        material: hf_sim::world::Material::WOOD,
                        quality: 0.8,
                        built: 0,
                        snow: 0.0,
                    },
                );
            }
        }
        for (p, _) in hf_sim::houses::walls(&upper_floor) {
            w.grid
                .set(p, hf_sim::world::Tile::solid(hf_sim::world::Material::WOOD));
            w.built_of.insert(p, hf_sim::world::Material::WOOD);
        }
        w.communities[0].house_sites = vec![TilePos::new(41, 41, 2)];
        w.communities[0].house_plans = vec![hf_sim::society::HousePlan {
            floor,
            doors: vec![TilePos::new(41, 43, 2)],
            upper_floor,
            upper_stair: Some(lower_stair),
            upper_reason: None,
        }];
        w.communities[0].house_stage = vec![3];
        w.rebuild_caches();

        assert_eq!(
            SimHost::upper_house_note(&w, TilePos::new(42, 41, 3)).as_deref(),
            Some("Upper storey (1 above ground), roofed.")
        );
        assert_eq!(SimHost::upper_house_note(&w, TilePos::new(41, 41, 2)), None);
    }

    #[test]
    fn feed_events_carry_simulation_backed_because_lines() {
        let mut w = World::opening_world(1);
        let mut chron = Chronicle::in_memory().unwrap();
        let pos = w.agents[0].pos;
        let tick = w.meta.tick;
        let flood_id = w.events.push(
            tick,
            Some(pos),
            Severity::Major,
            hf_sim::events::EventKind::Flooded { tiles: 6 },
        );
        let drenched_id = w.events.push(
            tick,
            Some(pos),
            Severity::Notable,
            hf_sim::events::EventKind::Drenched {
                at: pos,
                what: "grainsheaf".into(),
                count: 4,
                cause: flood_id,
            },
        );
        let events = w.events.drain();
        chron.record(&events).unwrap();
        let drenched_ev = events
            .iter()
            .find(|e| e.id == drenched_id)
            .expect("drenched event");
        let line = narrate::line(&w, drenched_ev);
        let because = event_because(&w, &chron, drenched_ev, &line);
        assert!(
            because.contains("flood") || because.contains("water"),
            "causal EventId resolves its chronicle narration as the because-line: {because}"
        );
    }

    /// FIXLOG 2026-11-14: a person's mood is the because-line only of what mood drives. Finishing
    /// a hearth is not "because: torn from the old world"; a mind giving way may be.
    #[test]
    fn the_feed_gives_a_mood_as_the_cause_only_of_what_mood_drives() {
        let mut w = World::opening_world(1);
        let chron = Chronicle::in_memory().unwrap();
        let i = w
            .agents
            .iter()
            .position(|a| a.alive && a.species == hf_sim::agents::Species::Person)
            .expect("a person");
        let id = w.agents[i].id;
        let pos = w.agents[i].pos;
        let tick = w.meta.tick;
        let mood = hf_sim::view::why::because(&w, &w.agents[i]);
        assert!(!mood.is_empty(), "a new arrival has a mood to give");
        w.events.push(
            tick,
            Some(pos),
            Severity::Notable,
            hf_sim::events::EventKind::Broke {
                agent: id,
                kind: "a daze".into(),
                explanation: None,
            },
        );
        w.events.push(
            tick,
            Some(pos),
            Severity::Notable,
            hf_sim::events::EventKind::Tended {
                healer: id,
                patient: id,
            },
        );
        let events = w.events.drain();
        let broke = &events[events.len() - 2];
        let tended = &events[events.len() - 1];
        let b = event_because(&w, &chron, broke, &narrate::line(&w, broke));
        assert_eq!(b, mood, "a mind giving way: the mood is its because-line");
        let t = event_because(&w, &chron, tended, &narrate::line(&w, tended));
        assert!(
            !t.contains(&mood) || mood.is_empty(),
            "tending is not because of a mood: {t}"
        );
    }
}
