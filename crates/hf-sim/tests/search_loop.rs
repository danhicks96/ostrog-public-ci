//! The round trip: word reaches someone, they go to look, and what is there is what is found.

use hf_sim::opening::{Rumour, SurvivorState};
use hf_sim::world_state::World;

fn lost_one(w: &World) -> (u8, hf_sim::ids::AgentId) {
    w.survivor_pool
        .iter()
        .find_map(|s| match s.state {
            SurvivorState::Lost {
                agent,
                found: false,
            } => Some((s.index, agent)),
            _ => None,
        })
        .expect("seven are lost")
}

#[test]
fn someone_goes_and_finds_out() {
    let mut w = World::opening_world(21);
    let (who, id) = lost_one(&w);
    let at = w.agents.iter().find(|a| a.id == id).unwrap().pos;
    // Word reaches one of the party, from a stranger who passed them this morning.
    let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    {
        let now = w.meta.epoch + 9 * hf_sim::time::TICKS_PER_HOUR;
        w.meta.tick = now;
        let a = &mut w.agents[i];
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
        a.needs.social = 1.0;
        a.heard.push(Rumour {
            who,
            at,
            heard: now,
            from: None,
        });
    }
    let mut settled = false;
    for _ in 0..6000 {
        w.step();
        let s = w.survivor_pool.iter().find(|s| s.index == who).unwrap();
        if !matches!(s.state, SurvivorState::Lost { found: false, .. }) {
            settled = true;
            break;
        }
    }
    assert!(
        settled,
        "going there, or meeting them on the way, settled it"
    );
}

#[test]
fn no_word_means_nobody_goes() {
    let mut w = World::opening_world(23);
    for a in w.agents.iter_mut() {
        a.heard.clear();
    }
    for _ in 0..3000 {
        w.step();
        assert!(
            w.searches.iter().all(|s| w
                .agents
                .iter()
                .find(|a| a.id == s.by)
                .unwrap()
                .heard
                .iter()
                .any(|r| r.who == s.who)),
            "nobody searches without word"
        );
    }
}
