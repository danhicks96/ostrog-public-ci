//! The sim on its own thread (#603, docs/design-renderer.md stage 1). A step used to run on
//! Godot's main thread, so a slow tick (15.9 ms at 500 people, more on the daily planner's
//! ticks) was a stutter in the frame it landed in. Now the host hands the session, with what
//! the feed, the director and the sounds keep between ticks, to one worker thread, which runs
//! the ticks the client asked for and hands everything back with a snapshot: a copy of the
//! world as the ticks left it, which the view reads while the next ticks run.
//!
//! Nothing about the sim changes: the same `Session::step` runs the same ticks in the same
//! order, only on another thread, so a world is the same tick for tick as one stepped in place
//! (`tests` below, and the GDScript guards in tools/gd_test.gd). The worker is one thread; the
//! sim's own parallel parts still use its rayon pool, capped as before (par.rs).

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use hf_sim::ids::{AgentId, TilePos};
use hf_sim::view::bodies::Bodies;
use hf_sim::view::director::{Director, Suggestion};
use hf_sim::{Session, World};

/// Feed lines kept for the client: (tick, where, severity, text, kind, favourite, because, fold
/// key; #1483: "" for a row that is always its own news).
pub(crate) type Feed = Vec<(
    u64,
    TilePos,
    i32,
    String,
    &'static str,
    bool,
    String,
    String,
)>;

/// Everything a tick changes: the session and what the feed, director, sounds and bodies keep
/// between ticks.
pub(crate) struct Core {
    pub session: Session,
    pub feed: Feed,
    pub sounds: Vec<(&'static str, TilePos)>,
    pub bodies: Bodies,
    pub director: Director,
    pub published: Option<u32>,
    pub favourites: std::collections::BTreeSet<AgentId>,
    /// The director's suggestion as the last ticks left the world, and whether the client is
    /// asking for one (FIXLOG 2026-11-14, #797 finding 2).
    pub suggestion: Option<Suggestion>,
    pub want_suggestion: bool,
}

impl Core {
    /// Run `ticks` ticks, passing each tick's events on as the host always has.
    pub fn run(&mut self, ticks: u64) {
        let s = &mut self.session;
        for _ in 0..ticks {
            if s.step().is_err() {
                godot::global::godot_error!("chronicle write failed");
            }
            let tick = s.world.tick();
            crate::publish(
                &s.world,
                &s.chronicle,
                tick,
                &s.world.last_events,
                &self.favourites,
                &mut self.feed,
                &mut self.director,
                crate::Seen {
                    sounds: &mut self.sounds,
                    bodies: &mut self.bodies,
                },
                &mut self.published,
            );
        }
        if hf_sim::debuglog::enabled() {
            hf_sim::debuglog::flush();
        }
        if self.want_suggestion {
            self.suggest();
        }
    }

    /// Work out the director's suggestion here, beside the ticks, while the client asks for
    /// one. The Director travels with the session, and at any speed above paused the session is
    /// on this thread nearly every time the client asks, so a suggestion made only at home was
    /// never made and the camera never moved (FIXLOG 2026-11-14, #797 finding 2). Within the
    /// current suggestion's dwell the answer is the same one, so the people are only counted
    /// when a new one is due.
    pub fn suggest(&mut self) {
        let w = &self.session.world;
        let tick = w.tick();
        let due = match &self.director.current {
            Some(c) => tick.saturating_sub(self.director.since) >= c.dwell as u64,
            None => true,
        };
        if due || self.suggestion.is_none() {
            let people = hf_sim::view::director::people(w);
            let fallback = hf_sim::view::director::settlement_centre(w);
            self.suggestion = Some(self.director.suggest(tick, fallback, &people));
        }
    }
}

/// What the view reads while the session is away on the worker: the world and the bodies as
/// the last ticks left them.
pub(crate) struct Snapshot {
    pub world: World,
    pub bodies: Bodies,
}

/// A batch of ticks done.
pub(crate) struct Done {
    pub core: Box<Core>,
    pub snapshot: Arc<Snapshot>,
    pub step_ms: f64,
}

enum Msg {
    Run(Box<Core>, u64),
    /// An old snapshot, dropped here so freeing a world never costs the main thread.
    Drop(Arc<Snapshot>),
}

/// The worker and the one batch it may have in hand.
pub(crate) struct SimThread {
    tx: Sender<Msg>,
    rx: Receiver<Done>,
    pub in_flight: bool,
}

impl SimThread {
    pub fn spawn() -> SimThread {
        let (tx, jobs) = channel::<Msg>();
        let (done, rx) = channel::<Done>();
        std::thread::Builder::new()
            .name("hf-sim-step".into())
            .spawn(move || {
                for m in jobs {
                    match m {
                        Msg::Run(mut core, ticks) => {
                            let t0 = std::time::Instant::now();
                            core.run(ticks);
                            let step_ms = t0.elapsed().as_secs_f64() * 1000.0;
                            let snapshot = Arc::new(Snapshot {
                                world: core.session.world.clone(),
                                bodies: core.bodies.clone(),
                            });
                            if done
                                .send(Done {
                                    core,
                                    snapshot,
                                    step_ms,
                                })
                                .is_err()
                            {
                                return;
                            }
                        }
                        Msg::Drop(s) => drop(s),
                    }
                }
            })
            .expect("the sim's step thread");
        SimThread {
            tx,
            rx,
            in_flight: false,
        }
    }

    /// Hand `core` over to run `ticks` ticks.
    pub fn send(&mut self, core: Box<Core>, ticks: u64) {
        assert!(!self.in_flight, "one batch at a time");
        self.tx.send(Msg::Run(core, ticks)).expect("step thread");
        self.in_flight = true;
    }

    /// The batch, if it is done; never waits.
    pub fn try_take(&mut self) -> Option<Done> {
        if !self.in_flight {
            return None;
        }
        let d = self.rx.try_recv().ok()?;
        self.in_flight = false;
        Some(d)
    }

    /// The batch, waiting for it if it is still running (None when none was sent).
    pub fn wait(&mut self) -> Option<Done> {
        if !self.in_flight {
            return None;
        }
        let d = self.rx.recv().expect("step thread");
        self.in_flight = false;
        Some(d)
    }

    pub fn discard(&self, s: Arc<Snapshot>) {
        let _ = self.tx.send(Msg::Drop(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hf_sim::chronicle::Chronicle;

    fn core(seed: u64) -> Box<Core> {
        let w = World::opening_world(seed);
        let mut session = Session::new(w, Chronicle::in_memory().unwrap());
        session.write_pending().unwrap();
        Box::new(Core {
            session,
            feed: Vec::new(),
            sounds: Vec::new(),
            bodies: Bodies::default(),
            director: Director::default(),
            published: None,
            favourites: Default::default(),
            suggestion: None,
            want_suggestion: false,
        })
    }

    /// #603: ticks run on the step thread, in uneven batches handed back and forth, leave the
    /// world the same as the same ticks run in place, at every batch's end; and the snapshot is
    /// that world.
    #[test]
    fn the_step_thread_gives_the_world_ticks_in_place_give() {
        let mut here = core(3);
        let mut there = Some(core(3));
        let mut t = SimThread::spawn();
        for batch in [1u64, 7, 3, 40, 1, 20, 9] {
            here.run(batch);
            t.send(there.take().unwrap(), batch);
            let d = t.wait().expect("a batch");
            assert_eq!(d.core.session.world.tick(), here.session.world.tick());
            assert_eq!(
                hf_sim::digest::digest(&d.core.session.world),
                hf_sim::digest::digest(&here.session.world),
                "tick {}",
                here.session.world.tick()
            );
            assert_eq!(
                hf_sim::digest::digest(&d.snapshot.world),
                hf_sim::digest::digest(&here.session.world)
            );
            assert_eq!(d.core.feed.len(), here.feed.len(), "the same feed lines");
            t.discard(d.snapshot);
            there = Some(d.core);
        }
        assert!(t.wait().is_none(), "nothing in hand");
    }

    /// #797 finding 2: while the client asks, every batch the step thread hands back carries
    /// the director's suggestion for the world it ran to, the same one the director at home
    /// would give for those ticks; when nobody asks, none is worked out.
    #[test]
    fn the_step_thread_brings_the_directors_suggestion_home() {
        let mut here = core(3);
        let mut there = core(3);
        there.want_suggestion = true;
        let mut there = Some(there);
        let mut t = SimThread::spawn();
        for batch in [1u64, 20, 20, 7, 400, 20] {
            here.run(batch);
            let w = &here.session.world;
            let people = hf_sim::view::director::people(w);
            let fallback = hf_sim::view::director::settlement_centre(w);
            let want = here.director.suggest(w.tick(), fallback, &people);
            t.send(there.take().unwrap(), batch);
            let d = t.wait().expect("a batch");
            let got = d
                .core
                .suggestion
                .clone()
                .expect("a suggestion with every batch");
            assert_eq!(
                (got.at, got.kind, got.dwell),
                (want.at, want.kind, want.dwell),
                "tick {}",
                w.tick()
            );
            t.discard(d.snapshot);
            there = Some(d.core);
        }
        let mut idle = core(4);
        idle.run(20);
        assert!(
            idle.suggestion.is_none(),
            "nobody asked: no suggestion worked out"
        );
    }
}
