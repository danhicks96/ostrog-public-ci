//! Census of cave mouths (#482): for each seed, how many mouths the valley has, how long, and
//! how deep. Run: cargo run -p hf-sim --example mouth_census -- 1 12
use hf_sim::setup::WorldSetup;
use hf_sim::world::gen::generate;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let from: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let to: u64 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(12);
    let (mut worlds, mut with) = (0, 0);
    let mut total = 0;
    for seed in from..=to {
        let mut s = WorldSetup::new(seed);
        if s.realm_years > 0 {
            let m = hf_sim::realm::generate(s.seed, &hf_sim::realm::RealmParams::default());
            let r = s.region.unwrap_or(0);
            s.character = Some(hf_sim::realm::embark::character_of(&m, r));
        }
        let g = generate(seed, &s.params());
        worlds += 1;
        total += g.cave_mouths.len();
        if !g.cave_mouths.is_empty() {
            with += 1;
        }
        let list: Vec<String> = g
            .cave_mouths
            .iter()
            .map(|m| {
                format!(
                    "foot ({},{},z{}) -> entrance ({},{},z{}) len {} to cave ({},{},z{})",
                    m.foot.x,
                    m.foot.y,
                    m.foot.z,
                    m.entrance.x,
                    m.entrance.y,
                    m.entrance.z,
                    m.length,
                    m.cave.x,
                    m.cave.y,
                    m.cave.z
                )
            })
            .collect();
        println!(
            "seed {seed}: {} mouths {}",
            g.cave_mouths.len(),
            list.join("; ")
        );
    }
    println!("{with} of {worlds} worlds have a mouth; {total} in all");
}
