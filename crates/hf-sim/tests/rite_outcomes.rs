//! #338: a rite for the dead is fitting, plain or marred, and a marred one leaves them unrested.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn rites_go_well_and_badly_and_a_marred_one_leaves_the_dead_unrested() {
    let (mut fit, mut mar) = (0, 0);
    for seed in 0..60u64 {
        let mut w = World::generate(3380 + seed, GenParams::small());
        w.flatten(2, 2, 60, 60, 1);
        w.meta.founding_site = TilePos::new(30, 30, 2);
        w.meta.tick = TICKS_PER_HOUR * 10;
        w.rebuild_caches();
        let ids = w.found_party(5);
        w.meta.autoplan = false;
        let cid = w.communities[0].id;
        // Rain on half of them.
        if seed % 2 == 0 {
            let mut wx = hf_sim::systems::weather::current(&w);
            wx.sky = hf_sim::systems::weather::Sky::Rain;
            w.weather = wx;
        }
        hf_sim::systems::life::die(&mut w, ids[0], "a test".into());
        w.events.pending.clear();
        assert!(hf_sim::systems::rites::lay_to_rest(
            &mut w, ids[0], cid, None, false
        ));
        for e in &w.events.pending {
            if let EventKind::RiteOutcome { outcome, agent, .. } = &e.kind {
                let d = w.agents.iter().find(|a| a.id == *agent).unwrap();
                let unrested = d
                    .conditions
                    .iter()
                    .any(|c| c.kind == hf_sim::conditions::ConditionKind::Unrested);
                if outcome == "marred" {
                    mar += 1;
                    assert!(unrested, "a marred rite: the dead do not rest");
                } else {
                    fit += 1;
                    assert!(!unrested);
                    assert!(w.agents.iter().any(|a| a.alive
                        && a.thoughts
                            .list
                            .iter()
                            .any(|t| t.kind == hf_sim::mind::ThoughtKind::FittingRite)));
                }
            }
        }
    }
    assert!(fit > 0 && mar > 0, "fitting {fit}, marred {mar}");
}
