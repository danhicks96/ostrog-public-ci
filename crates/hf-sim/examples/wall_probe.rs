//! The state of every community's wall in a save (#403): each ring tile not standing, its
//! order and materials, the outline's walkable tiles off the ring, and planks and crafts.
//! STEPDAYS=n runs the save on n days first; COL=x lists the outline's column x.
//! Run: cargo run --release -p hf-sim --example wall_probe <save dir>
use hf_sim::world::Shape;

fn main() {
    let dir = std::env::args().nth(1).expect("save dir");
    let (mut w, _c) = hf_sim::save::read(std::path::Path::new(&dir)).expect("read");
    let days: u64 = std::env::var("STEPDAYS")
        .ok()
        .and_then(|d| d.parse().ok())
        .unwrap_or(0);
    for _ in 0..days * hf_sim::time::TICKS_PER_DAY {
        w.step();
    }
    w.ensure_nav();
    for c in &w.communities {
        let knows = |t: hf_sim::mind::Technique| {
            w.agents
                .iter()
                .filter(|a| a.alive && a.community == Some(c.id) && a.techniques.knows(t))
                .count()
        };
        println!(
            "{}: palisade {} carpenters {} masons {}",
            c.name,
            c.palisade.is_some(),
            knows(hf_sim::mind::Technique::Carpentry),
            knows(hf_sim::mind::Technique::Masonry)
        );
    }
    println!(
        "tick {} day {}",
        w.meta.tick,
        w.meta.tick / hf_sim::time::TICKS_PER_DAY
    );
    let comms = w.communities.clone();
    for c in &comms {
        let Some(pal) = &c.palisade else { continue };
        println!(
            "{} (dissolved {}) home {:?} houses {} begun day {} stood {:?} closed {} ring {}",
            c.name,
            c.dissolved,
            c.home,
            c.house_sites.len(),
            pal.begun / hf_sim::time::TICKS_PER_DAY,
            pal.stood,
            pal.closed,
            pal.ring.len()
        );
        let nav = w.nav.as_ref().unwrap();
        let mut up = 0;
        for q in &pal.ring {
            let t = w.grid.get(*q);
            let standing = (*q == pal.gate && t.flags.has(hf_sim::world::TileFlags::DOOR))
                || (*q != pal.gate && t.shape == Shape::Wall);
            if standing {
                up += 1;
                continue;
            }
            let reach = nav.reach(c.home, *q);
            let ds: Vec<String> = w
                .designations
                .iter()
                .filter(|d| d.pos == *q && !d.done)
                .map(|d| {
                    format!(
                        "{:?} pri {} fail {} comm {:?} mats_ok {} reqs {:?}",
                        d.kind,
                        d.priority,
                        d.failures,
                        d.community,
                        d.materials_complete(),
                        d.requirements
                    )
                })
                .collect();
            println!(
                "  open {:?} gate {} shape {:?} walkable {} reach {} items {} designations {:?}",
                q,
                *q == pal.gate,
                t.shape,
                w.grid.walkable(*q),
                reach,
                w.items
                    .iter()
                    .filter(|it| it.place == hf_sim::items::ItemPlace::Ground(*q))
                    .count(),
                ds
            );
        }
        println!("  standing {up} of {}", pal.ring.len());
        // Outline tiles not in the ring and walkable (the `sealed` test).
        let (x0, x1) = (
            pal.ring.iter().map(|q| q.x).min().unwrap_or(0),
            pal.ring.iter().map(|q| q.x).max().unwrap_or(0),
        );
        let (y0, y1) = (
            pal.ring.iter().map(|q| q.y).min().unwrap_or(0),
            pal.ring.iter().map(|q| q.y).max().unwrap_or(0),
        );
        for x in x0..=x1 {
            for y in y0..=y1 {
                if !(x == x0 || x == x1 || y == y0 || y == y1) {
                    continue;
                }
                let s = w.grid.surface(x, y);
                if !pal.ring.iter().any(|q| q.x == x && q.y == y) && w.grid.walkable(s) {
                    let st: Vec<_> = w
                        .structures
                        .iter()
                        .filter(|st| st.pos == s)
                        .map(|st| st.kind)
                        .collect();
                    println!(
                        "  unsealed {:?} water {} structures {:?}",
                        s,
                        w.grid.get(s).is_water(),
                        st
                    );
                }
            }
        }
        let wall_orders = w
            .designations
            .iter()
            .filter(|d| {
                !d.done
                    && d.community == Some(c.id)
                    && matches!(
                        d.kind,
                        hf_sim::work::DesignationKind::BuildWall(_)
                            | hf_sim::work::DesignationKind::BuildDoor
                    )
            })
            .count();
        println!("  open wall/door orders of this community: {wall_orders}");
        println!("  grid {}x{}", w.grid.width, w.grid.height);
        for o in &c.house_sites {
            let plan = hf_sim::houses::plan(&w, *o);
            let ws = hf_sim::houses::walls(&plan.floor);
            let xs: Vec<i32> = ws.iter().map(|(q, _)| q.x).collect();
            let ys: Vec<i32> = ws.iter().map(|(q, _)| q.y).collect();
            println!(
                "  house {:?} x {}..{} y {}..{}",
                o,
                xs.iter().min().unwrap_or(&0),
                xs.iter().max().unwrap_or(&0),
                ys.iter().min().unwrap_or(&0),
                ys.iter().max().unwrap_or(&0)
            );
        }
        let count = |k: hf_sim::items::ItemKind| -> u32 {
            w.items
                .iter()
                .filter(|it| it.kind == k)
                .map(|it| it.count)
                .sum()
        };
        println!(
            "  planks {} logs {} in the valley",
            count(hf_sim::items::ItemKind::Plank),
            count(hf_sim::items::ItemKind::Log)
        );
        let crafts: Vec<String> = w
            .designations
            .iter()
            .filter(|d| !d.done && matches!(d.kind, hf_sim::work::DesignationKind::Craft(_)))
            .map(|d| format!("{:?} fail {}", d.kind, d.failures))
            .collect();
        println!("  craft orders {:?}", crafts);
        let mem: Vec<usize> = (0..w.agents.len())
            .filter(|&i| w.agents[i].alive && w.agents[i].community == Some(c.id))
            .collect();
        println!("  members {}", mem.len());
        let nav = w.nav.as_ref().unwrap();
        let mut by_why = std::collections::BTreeMap::<&str, u32>::new();
        for it in w
            .items
            .iter()
            .filter(|it| it.kind == hf_sim::items::ItemKind::Plank)
        {
            let why = match it.place {
                hf_sim::items::ItemPlace::Ground(p) => {
                    let known = mem.iter().any(|&i| {
                        w.agents[i]
                            .knowledge
                            .get(hf_sim::mind::knowledge::Known::Item(it.id))
                            .is_some()
                    });
                    let site = w.designations.iter().any(|d| d.pos == p && !d.done);
                    let reach = nav.reach(c.home, p);
                    let free = w.reservations.is_free_for(
                        hf_sim::work::ResKey::Item(it.id),
                        hf_sim::ids::AgentId(u32::MAX),
                    );
                    if !reach {
                        "unreachable"
                    } else if !known {
                        "unknown"
                    } else if site {
                        "on a site"
                    } else if !free {
                        "reserved"
                    } else {
                        "free"
                    }
                }
                _ => "not on ground",
            };
            *by_why.entry(why).or_default() += it.count;
        }
        println!("  planks by state {:?}", by_why);
        let logs_known: u32 = w
            .items
            .iter()
            .filter(|it| it.kind == hf_sim::items::ItemKind::Log)
            .filter(|it| {
                mem.iter().any(|&i| {
                    w.agents[i]
                        .knowledge
                        .get(hf_sim::mind::knowledge::Known::Item(it.id))
                        .is_some()
                })
            })
            .map(|it| it.count)
            .sum();
        println!("  logs known {logs_known} crafts {:?}", c.crafts);
        let near: Vec<String> = w
            .structures
            .iter()
            .filter(|s| s.pos.dist(c.home) <= 25)
            .map(|s| format!("{:?}@{},{} {:.1}", s.kind, s.pos.x, s.pos.y, s.progress))
            .collect();
        println!("  structures near {:?}", near);
        let crafts_ours: Vec<String> = w
            .designations
            .iter()
            .filter(|d| {
                !d.done
                    && d.community == Some(c.id)
                    && matches!(d.kind, hf_sim::work::DesignationKind::Craft(_))
            })
            .map(|d| {
                format!(
                    "{:?} at {:?} fail {} reqs {:?}",
                    d.kind, d.pos, d.failures, d.requirements
                )
            })
            .collect();
        println!("  our craft orders {:?}", crafts_ours);
        if let Ok(col) = std::env::var("COL") {
            let x: i32 = col.parse().unwrap();
            for y in y0 - 1..=y1 + 1 {
                let s = w.grid.surface(x, y);
                let below = w.grid.get(s.offset(0, 0, -1));
                let st: Vec<_> = w
                    .structures
                    .iter()
                    .filter(|st| st.pos.x == x && st.pos.y == y)
                    .map(|st| (st.kind, st.pos.z))
                    .collect();
                println!(
                    "   col {x},{y}: surface z {} shape {:?} below {:?} walk {} ring {} indoors {} structs {:?}",
                    s.z,
                    w.grid.get(s).shape,
                    below.shape,
                    w.grid.walkable(s),
                    pal.ring.iter().any(|q| q.x == x && q.y == y),
                    hf_sim::systems::hazards::indoors(&w, s),
                    st
                );
            }
            for o in w.communities.iter() {
                println!(
                    "   community {} home {:?} houses {:?}",
                    o.name, o.home, o.house_sites
                );
            }
        }
    }
}
