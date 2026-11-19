//! The tick pipeline: one system per file, run in a fixed order (DECISIONS D7). Adding a
//! system means adding a line here, in the position the design calls for.

use crate::world_state::World;

pub mod act; //      carry out the committed activity (eat, sleep, forage, talk, loiter)
pub mod animals; //  wildlife: graze, flee, hunt, breed
pub mod arms; //     spears and armour taken up when strangers threaten, laid by after
pub mod beliefs; //  talk spreads beliefs, outcomes confirm or shake them
pub mod childhood; // children learn by watching, play, and come of age (#296)
pub mod combat; //   blows, nerve, standing together against beasts and raiders (C75)
pub mod conscience; // a quarrel judged afterwards: regret or rationalising (#568)
pub mod crafts; //  crafts as personal knowledge (C70): who may work a bench, loss, lessons
pub mod culture; //  customs form, spread, are observed, fade
pub mod decide;
pub mod expedition; // caravans form, leave, travel in day-steps, return (hourly) //   idle agents score goals from needs × traits × knowledge and commit
pub mod feel; //    what is felt at what is seen, and its fading (design-feelings.md)
pub mod flush; //    drain events into last_events (chronicle writer pulls from there)
pub mod hazards; //  fire, flood, cold
pub mod health; //   wounds, bleeding, infection, illness, ailments (hourly/daily)
pub mod learn; //    watching a technique work teaches it
pub mod life; //     pairing, births, aging, death
pub mod moods; //    fey moods: an artisan taken by a work, for a recorded cause
pub mod movement; // agents follow their paths, one tile per step cost
pub mod moving; // moving camp, and the end of a community (C60)
pub mod nav; //      keep navigation regions current with the grid
pub mod needs; //    needs decay by activity
pub mod perceive; // local perception into knowledge (striped)
pub mod plants; //   growth by season
pub mod projects; // personal projects proposed: carvings, brewing, workshops, journeys (daily)
pub mod rescue; //   a way dug down to someone cut off below (#439)
pub mod rites; //   the dead laid to rest (staged, C68); a felled tree, a new kind joining (called)
pub mod settlement; // the community planner posts designations (hourly)
pub mod society; //    roles, newcomers, cohesion, splits, trade (hourly)
pub mod spoil; //    food goes off where it lies
pub mod strain; //   thoughts, stress, breaks (hourly; observe after flush)
pub mod water_quality; // rot and the dead foul the water; flow and time clear it (part 2 of #453)
pub mod weather; //  seasonal temperature and sky
pub mod work; //     execute work plans; lease sweep; designation effects

/// One named system of the pipeline.
pub type Stage = (&'static str, fn(&mut World));

/// The pipeline as data, so the plain and the timed runners cannot drift apart.
pub const PIPELINE: &[Stage] = &[
    ("deity", crate::deity::step), // what the deity notices, and how it leans (never an event of its own)
    ("spells", crate::spells::step), // a heat wave, a drought, an ill wind... (#278)
    ("weather", weather::step),
    ("plants", plants::step),
    ("thermal", crate::thermal::step), // frost in the fields, hardening to the cold (#346)
    ("cave_life", crate::cave_life::step), // mushrooms spread over dark cave floors (#271)
    ("hazards", hazards::step),
    ("water_flow", crate::aquifer::flow), // water let in from the rock moves (#323)
    ("water_quality", water_quality::step), // rot and the dead foul the water; flow and time clear it (#453 part 2)
    ("building", crate::building::step),    // snow on roofs, and roofs that lose their wall
    ("nav", nav::step),
    ("projects", projects::step), // what people want done, proposed with why
    ("settlement", settlement::step),
    ("rescue", rescue::step), // a way dug down to someone cut off below (#439)
    ("society", society::step),
    ("societies", crate::societies::step), // secret societies, and betrayal (#322)
    ("social", crate::social::step),       // good turns repaid (#295)
    ("crafts", crafts::step),              // crafts as personal knowledge: loss, lessons (C70)
    ("fostering", crate::fostering::step), // orphans taken in (#341)
    ("ties", crate::ties::step),           // visits across a divided loyalty (#348)
    ("expedition", expedition::step),
    ("journeys", crate::journeys::step), // away on their own and back, still one of them (#347)
    ("threats", crate::threat::step),    // hungry places beyond the valley, and the bands they send
    ("work_sweep", work::sweep),
    ("perceive", perceive::step),
    ("needs", needs::step),
    ("decide", decide::step),
    ("act", act::step),
    ("animals", animals::step),
    ("livestock", crate::livestock::step), // the herd grows (#285)
    ("traps", crate::traps::step),         // snares take game, or a stranger by the leg (#316)
    ("movement", movement::step),
    ("paths", crate::paths::step), // ground walked often wears to a path (#84)
    ("searches", crate::opening::resolve_pending_searches), // whoever went looking has arrived, or given up
    ("reunions", crate::opening::reunite), // the lost and the party meeting by sight
    ("life", life::step),
    ("rites", rites::step), // the dead laid to rest once someone has come to them (C68)
    ("health", health::step),
    ("feel", feel::step), // feelings fade; the strong carried as thoughts
    ("conscience", conscience::step), // a quarrel judged once the other is seen hurt (#568)
    ("childhood", childhood::step), // watching, playing, coming of age (#296)
    ("strain", strain::step),
    ("justice", crate::justice::step), // who is followed, and the evening's judgements (C77)
    ("keepers", crate::keepers::step), // the keeper of the observances named, and challenged (#297)
    ("heresy", crate::heresy::step), // those holding the signs apart, brought back or split off (#297)
    ("observances", crate::observances::step), // the season's observance, kept or let lapse (#297)
    ("shrines", crate::shrines::step), // a shrine taken as a community's, wearing, lost (#297)
    ("culture", culture::step),
    ("legend", crate::legend::step),
    ("relics", crate::relics::step), // the revered dead's things kept as relics, and lost (#297)
    ("pilgrimage", crate::pilgrimage::step), // to the shrine or a relic in hard times (#297)
    ("arms", arms::step),            // spears and armour taken up when strangers threaten
    ("moods", moods::step),          // fey moods taken and passed
    ("creatures", crate::creatures::step), // dens, migrations, carrion, the Hart
    ("ecology", crate::ecology::step), // stumps green over, kinds come back over the hills (#486)
    ("underground", crate::underground::step), // light below, the dark, the burrower
    ("conditions", crate::conditions::step), // the restless dead, curses (P11)
    ("thin", crate::thin::step),     // the thin place at the portal stone
    ("lore", crate::lore::step),
    ("accounts", crate::accounts::step), // graves of the colony's dead read by those who came after (#452)
    ("left_sites", crate::left_sites::step), // the camps left behind weather; who comes on them wonders (C72) // ruins wondered at; bands at their people's graves // the dead leave their tools: heirlooms to kin, the rest where they fell
    ("spoil", spoil::step),                  // food left lying goes off
    ("cascade", crate::cascade::step),       // flood -> rot -> vermin -> bite -> fester (#319)
    ("realm", crate::realm::live::step),     // the world goes on beyond the valley, once a year
    ("fate", crate::fate::step),             // how each community is faring (#353)
    ("arcs", crate::arcs::step),             // the troubles going on at once, told as one story
    ("flush", flush::step),
    ("culture_observe", culture::observe),
    ("strain_observe", strain::observe),
    ("justice_observe", crate::justice::observe), // deeds seen, deference given (C77)
    ("feuds_observe", crate::feuds::observe),     // feuds passed on, kept alive, ended (#404)
    ("feel_observe", feel::observe),              // events felt by those they touch
    ("beliefs_observe", beliefs::observe),
    ("learn_observe", learn::observe),
    ("hardship_observe", moving::observe),
    ("tiers_observe", crate::tiers::observe), // a harder stone first cut (#274)
    ("omens_observe", crate::omens::observe), // great events read as signs (#297)
    ("faith_observe", crate::faith::observe), // the deity read aloud, heeded or scoffed at (#297)
    ("fate_observe", crate::fate::observe),   // the dead tallied by cause (#353)
    ("social_observe", crate::social::observe), // gossip, favours, making up (#295)
    ("nemesis_observe", crate::nemesis::observe), // a rival who fell (nemesis.rs)
    ("aquifer_observe", crate::aquifer::observe), // a cut into wet rock lets the water in (#272)
    ("known_observe", crate::known::observe), // the peoples beyond, as the valley has seen them (#349)
    ("ties_observe", crate::ties::observe),   // a split leaves some torn (#348)
    ("delving_observe", crate::delving::observe), // the deep reached, a cavern looked into, a breach answered (#405)
];

pub fn run_pipeline(w: &mut World) {
    for (_, f) in PIPELINE {
        f(w);
    }
    if crate::debuglog::enabled() {
        crate::debuglog::after_tick(w);
    }
}

/// Same order, with wall-clock milliseconds per system (profiling only; not used in the
/// simulation, so determinism is untouched).
pub fn run_pipeline_timed(w: &mut World, out: &mut [f64]) {
    for (i, (_, f)) in PIPELINE.iter().enumerate() {
        let t0 = std::time::Instant::now();
        f(w);
        out[i] = t0.elapsed().as_secs_f64() * 1000.0;
    }
    if crate::debuglog::enabled() {
        crate::debuglog::after_tick(w);
    }
}
