//! #349: the valley knows the peoples beyond it only by what its people saw.

use hf_sim::events::EventKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

#[test]
fn strangers_sighted_put_their_people_on_the_valleys_map_but_not_their_home() {
    let mut w = World::opening_world(3);
    w.meta.autoplan = true;
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
    }
    assert!(w.communities[0].known_others.is_empty(), "nobody seen yet");
    let s = w.overworld.sites.iter_mut().find(|s| s.people > 0).unwrap();
    s.food = -(s.people as f32) * 4.0;
    s.hungry_days = 3;
    let mut word = false;
    for _ in 0..12 * TICKS_PER_DAY {
        w.step();
        word |= w
            .last_events
            .iter()
            .any(|e| matches!(e.kind, EventKind::FirstWord { .. }));
        if word {
            break;
        }
    }
    assert!(word, "strangers seen, their people known of");
    let cid = w.communities[0].id;
    let k = w.communities[0].known_others[0].clone();
    assert!(k.most > 0);
    let visited = w
        .overworld
        .sites
        .iter()
        .any(|s| s.id == k.site && s.known_by.contains(&cid));
    assert_eq!(
        k.located(&w, cid),
        visited,
        "where they live, only if someone has been"
    );
}
