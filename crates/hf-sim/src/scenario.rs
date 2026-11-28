//! Scenario builders: small deterministic worlds for tests, and the staged representative
//! scene used for art review only (DECISIONS D13: never evidence of autonomy).
use crate::fmath::Det;

use rand::Rng;

use crate::agents::{Agent, Species};
use crate::ids::TilePos;
use crate::items::{ItemKind, ItemPlace};
use crate::plants::{Plant, PlantKind};
use crate::rng::{purpose, stream};
use crate::structures::{Structure, StructureKind};
use crate::world::gen::GenParams;
use crate::world::{Material, Shape, Tile, TileFlags, WorldGrid};
use crate::World;

impl World {
    pub fn spawn_agent(&mut self, species: Species, pos: TilePos) -> crate::ids::AgentId {
        let id = self.alloc_agent_id();
        let mut r = stream(self.meta.seed, &[purpose::AGENT_BIRTH, id.0 as u64]);
        let genome = crate::genetics::Genome::random(&mut r);
        let appearance = genome.appearance(&mut r);
        let name = match species {
            // Checked against every name in `self.agents`, living or dead: a dead person's
            // record stays in the list rather than being removed, so a death does not free
            // their name for reuse within this valley (#862).
            Species::Person => crate::names::unique_person_name(self.meta.seed, id.0, |n| {
                self.agents
                    .iter()
                    .any(|a| a.species == Species::Person && a.name == n)
            }),
            _ => format!("{species:?}").to_lowercase(),
        };
        let pushed_at = self.agents.len();
        let human = species == Species::Person;
        // A grown person is 16 to 45. A beast is a few years old, from a stream of its own so
        // the draws after this one are the same for every species.
        let person_years = r.gen_range(16..45);
        let years = if human {
            person_years
        } else {
            stream(self.meta.seed, &[purpose::AGENT_BIRTH, id.0 as u64, 0xA6E]).gen_range(1..5)
        };
        self.agents.push(Agent {
            id,
            name,
            species,
            pos,
            came: self.meta.tick,
            died_alone: false,
            born: self.meta.tick as i64
                - ((crate::time::TICKS_PER_YEAR * years) as f64 / self.meta.life_speed as f64)
                    as i64,
            appearance,
            alive: true,
            facing: r.gen_range(0..4),
            path: Vec::new(),
            from: pos,
            move_ticks: 0,
            move_total: 0,
            path_blocked: false,
            idle_ticks: 0,
            needs: crate::mind::Needs {
                hunger: r.gen_range(0.5..0.95),
                rest: r.gen_range(0.6..1.0),
                ..Default::default()
            },
            // Temperament and skills are a person's: a beast has neither.
            traits: if human {
                genome.traits()
            } else {
                Default::default()
            },
            dispositions: human.then_some(genome.dispositions()),
            skills: if human {
                let mut sk = genome.aptitudes();
                for st in sk.states.iter_mut() {
                    let lvl: f32 = r.gen_range(0.0..6.0);
                    st.xp = -120.0 * (1.0 - lvl / 20.0).d_ln();
                    st.level = lvl;
                }
                sk
            } else {
                Default::default()
            },
            knowledge: Default::default(),
            learning: Default::default(),
            problems: Default::default(),
            activity: Default::default(),
            activity_ticks: 0,
            switches: 0,
            last_meal: None,
            last_sleep: self.meta.tick,
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
            genome,
            mother: None,
            father: None,
            partner: None,
            children: Vec::new(),
            starving_ticks: 0,
            last_birth: 0,
            death: None,
            age_scale: self.meta.life_speed,
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
            // A beast whose body is not a skeleton is born whole, its size within its plan's
            // count by its genome (#423).
            body: crate::body::Body {
                form: crate::plan::beast_plan(species).and_then(|p| {
                    crate::body::Form::whole_of(
                        p,
                        ((genome.size_roll() >> 16) % 1000) as f32 / 999.0,
                    )
                }),
                ..Default::default()
            },
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
        });
        if human {
            let now = self.meta.tick;
            let a = &mut self.agents[pushed_at];
            a.techniques = crate::mind::Techniques::common(&a.skills, &a.traits, now);
        }
        id
    }

    /// A child is born to `mother` and `father` at the mother's position: genome inherited
    /// with variation, looks and temperament from it, skills at zero, no memories.
    pub fn give_birth(
        &mut self,
        mother: crate::ids::AgentId,
        father: crate::ids::AgentId,
    ) -> Option<crate::ids::AgentId> {
        let id = self.alloc_agent_id();
        let tick = self.meta.tick;
        let mut r = stream(self.meta.seed, &[purpose::AGENT_BIRTH, id.0 as u64, tick]);
        let (mg, fg, pos, community, kind, mother_kind, mother_rite, mother_culture, body) = {
            let m = self.agents.iter().find(|a| a.id == mother)?;
            let f = self.agents.iter().find(|a| a.id == father)?;
            // A child of two kinds is one parent's kind whole, drawn even (P9, decision 1):
            // that parent's species and body block. Raised to the mother's rite either way.
            let of = if m.people != f.people && r.gen_bool(0.5) {
                f
            } else {
                m
            };
            (
                m.genome,
                f.genome,
                m.pos,
                m.community,
                of.people,
                of.kind,
                m.rite,
                m.culture,
                (m.people != f.people).then_some(of.genome.body),
            )
        };
        let mut genome = crate::genetics::Genome::inherit(&mg, &fg, &mut r);
        if let Some(b) = body {
            genome.body = b;
        }
        let appearance = genome.appearance(&mut r);
        let name = crate::names::unique_person_name(self.meta.seed, id.0, |n| {
            self.agents
                .iter()
                .any(|a| a.species == Species::Person && a.name == n)
        });
        self.agents.push(Agent {
            id,
            name,
            species: Species::Person,
            pos,
            came: tick,
            died_alone: false,
            born: tick as i64,
            appearance,
            alive: true,
            facing: 0,
            path: Vec::new(),
            from: pos,
            move_ticks: 0,
            move_total: 0,
            path_blocked: false,
            idle_ticks: 0,
            needs: crate::mind::Needs::default(),
            traits: genome.traits(),
            dispositions: Some(genome.dispositions()),
            skills: genome.aptitudes(),
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
            mood: 0.2,
            community,
            role: None,
            duties: Default::default(),
            leaving: false,
            bound_for: None,
            genome,
            mother: Some(mother),
            father: Some(father),
            partner: None,
            children: Vec::new(),
            starving_ticks: 0,
            last_birth: 0,
            death: None,
            age_scale: self.meta.life_speed,
            injured_until: 0,
            caravan: None,
            last_caravan: 0,
            fought_at: 0,
            people: kind,
            kind: mother_kind,
            rite: mother_rite,
            culture: mother_culture,
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
        });
        if let Some(c) = community {
            self.join_community(id, c);
        }
        // Upbringing: a child starts with what their parents hold, lightly.
        let inherited = {
            let ps: Vec<(crate::ids::AgentId, &crate::mind::Beliefs)> = self
                .agents
                .iter()
                .filter(|a| a.id == mother || a.id == father)
                .map(|a| (a.id, &a.beliefs))
                .collect();
            crate::mind::Beliefs::inherit(&ps, tick)
        };
        if let Some(c) = self.agents.iter_mut().find(|a| a.id == id) {
            c.beliefs = inherited;
        }
        for p in [mother, father] {
            if let Some(a) = self.agents.iter_mut().find(|a| a.id == p) {
                a.children.push(id);
                a.last_birth = tick;
                a.relate(id, 0.6, 0.5, tick);
            }
        }
        if let Some(a) = self.agents.iter_mut().find(|a| a.id == id) {
            a.relate(mother, 0.6, 0.5, tick);
            a.relate(father, 0.5, 0.4, tick);
            // A child of another kind is drawn in that kind's tones, read from the genome as
            // their parents' were, so kin look alike (the review of C38-C44).
            if kind != crate::species::SpeciesId::HUMAN {
                let def = mother_kind
                    .filter(|_| kind.0 >= crate::species::GENERATED)
                    .unwrap_or_else(|| crate::species::get(kind));
                let tone = def.art.skin
                    [a.genome.loci[crate::genetics::locus::SKIN] as usize % def.art.skin.len()]
                .as_str();
                a.appearance.skin = crate::species::SKIN_NAMES
                    .iter()
                    .position(|k| *k == tone)
                    .unwrap_or(0) as u8;
            }
        }
        // A newborn is kin to grandparents and siblings from the first day, not a stranger
        // waiting on a chance encounter (FIXLOG 2026-11-15, #1804): each living grandparent
        // (the parents' own mothers and fathers) and each living sibling (shares the mother or
        // the father) relates to the child, and the child relates back, as the parent ties do.
        let kin: Vec<crate::ids::AgentId> = {
            let grandparents = [mother, father].into_iter().flat_map(|p| {
                self.agents
                    .iter()
                    .find(|a| a.id == p)
                    .into_iter()
                    .flat_map(|a| [a.mother, a.father])
                    .flatten()
            });
            let siblings = self
                .agents
                .iter()
                .filter(|a| a.id != id && (a.mother == Some(mother) || a.father == Some(father)))
                .map(|a| a.id);
            grandparents
                .chain(siblings)
                .filter(|&k| self.agents.iter().any(|a| a.id == k && a.alive))
                .collect()
        };
        for k in &kin {
            if let Some(a) = self.agents.iter_mut().find(|a| a.id == *k) {
                a.relate(id, 0.4, 0.3, tick);
            }
        }
        if let Some(a) = self.agents.iter_mut().find(|a| a.id == id) {
            for k in &kin {
                a.relate(*k, 0.3, 0.2, tick);
            }
        }
        self.events.push(
            tick,
            Some(pos),
            crate::events::Severity::Major,
            crate::events::EventKind::Born {
                agent: id,
                mother,
                father: Some(father),
            },
        );
        Some(id)
    }

    pub fn place_structure(
        &mut self,
        kind: StructureKind,
        pos: TilePos,
        material: Material,
        active: bool,
    ) {
        let id = self.alloc_structure_id();
        self.structures.push(Structure {
            id,
            kind,
            pos,
            material,
            progress: 1.0,
            active,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }

    pub fn drop_item(&mut self, kind: ItemKind, count: u32, pos: TilePos) {
        self.create_item(kind, count, ItemPlace::Ground(pos), 0.6, "scenario");
    }

    /// Found a settlement: a party of `n` people arrives at the founding site with the
    /// provisions a travelling group would carry (recorded as Arrived events).
    pub fn found_party(&mut self, n: usize) -> Vec<crate::ids::AgentId> {
        let fs = self.meta.founding_site;
        let cid = self.found_community(fs);
        let ids = self.arrive_party(n, fs, Some(cid));
        crate::systems::animals::populate(self);
        // Provisions: about four days of food for the party, dropped beside the site.
        let per_person_meals = 2;
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
        self.drop_item(ItemKind::Meal, (n * per_person_meals) as u32, s(0));
        self.drop_item(ItemKind::Berries, 25, s(1));
        self.drop_item(ItemKind::Root, 20, s(2));
        self.drop_item(ItemKind::GrainSheaf, 15, s(3));
        self.drop_item(ItemKind::Log, 12, s(4));
        self.drop_item(ItemKind::Tool, 1, s(5));
        if let Some(id) = self
            .items
            .iter()
            .rev()
            .find(|it| it.kind == ItemKind::Tool)
            .map(|it| it.id)
        {
            crate::legend::ensure(self, id, None, "brought by the founders");
        }
        ids
    }

    /// Create a community at `home` with a generated name.
    pub fn found_community(&mut self, home: TilePos) -> crate::ids::CommunityId {
        let id = crate::ids::CommunityId(self.next_community_id);
        self.next_community_id += 1;
        let name = crate::names::community_name(self.meta.seed, id.0);
        self.communities.push(crate::society::Community::new(
            id,
            name,
            home,
            self.meta.tick,
        ));
        id
    }

    /// Spawn `n` people around `at`, joining `community` if given (Arrived events).
    pub fn arrive_party(
        &mut self,
        n: usize,
        at: TilePos,
        community: Option<crate::ids::CommunityId>,
    ) -> Vec<crate::ids::AgentId> {
        let tick = self.meta.tick;
        let mut ids = Vec::new();
        for i in 0..n as i32 {
            let p = self.grid.surface(
                (at.x + (i % 4) - 2).clamp(1, self.grid.width - 2),
                (at.y + (i / 4)).clamp(1, self.grid.height - 2),
            );
            let id = self.spawn_agent(Species::Person, p);
            if let Some(c) = community {
                self.join_community(id, c);
            }
            self.events.push(
                tick,
                Some(p),
                crate::events::Severity::Notable,
                crate::events::EventKind::Arrived {
                    agent: id,
                    community,
                },
            );
            ids.push(id);
        }
        ids
    }

    pub fn join_community(&mut self, who: crate::ids::AgentId, cid: crate::ids::CommunityId) {
        if let Some(a) = self.agents.iter_mut().find(|a| a.id == who) {
            if let Some(old) = a.community {
                if let Some(c) = self.communities.iter_mut().find(|c| c.id == old) {
                    c.members.retain(|m| *m != who);
                }
            }
            a.community = Some(cid);
        }
        if let Some(c) = self.communities.iter_mut().find(|c| c.id == cid) {
            if !c.members.contains(&who) {
                c.members.push(who);
            }
        }
    }

    /// Flatten a rectangle of columns to a ground height and clear plants there.
    pub fn flatten(&mut self, x0: i32, y0: i32, w: i32, h: i32, gz: i32) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                for z in (gz + 1)..=self.grid.z_max {
                    self.grid.set(TilePos::new(x, y, z), Tile::AIR);
                }
                for z in self.grid.z_min..=gz {
                    let t = self.grid.get(TilePos::new(x, y, z));
                    if t.shape != Shape::Wall {
                        let m = if z == gz {
                            Material::GRASS
                        } else {
                            Material::SOIL
                        };
                        self.grid.set(TilePos::new(x, y, z), Tile::solid(m));
                    }
                }
                let top = TilePos::new(x, y, gz);
                let mut t = self.grid.get(top);
                t.material = Material::GRASS;
                t.flags = TileFlags::NONE;
                self.grid.set(top, t);
            }
        }
        self.plants
            .retain(|p| !(p.pos.x >= x0 && p.pos.x < x0 + w && p.pos.y >= y0 && p.pos.y < y0 + h));
    }

    /// Build a walled room with a floor, a door on the south wall, at surface level.
    #[allow(clippy::too_many_arguments)]
    pub fn build_house(
        &mut self,
        x0: i32,
        y0: i32,
        w: i32,
        h: i32,
        gz: i32,
        wall: Material,
        floor: Material,
    ) {
        let z = gz + 1;
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let edge = x == x0 || y == y0 || x == x0 + w - 1 || y == y0 + h - 1;
                let p = TilePos::new(x, y, z);
                if edge {
                    let door = y == y0 + h - 1 && x == x0 + w / 2;
                    let mut t = Tile::solid(wall);
                    if door {
                        t.flags = TileFlags(TileFlags::DOOR);
                    }
                    self.grid.set(p, t);
                } else {
                    // Floor replaces the ground top so the interior is a built floor, and a
                    // roof goes over it (shingles on stone, else thatch).
                    self.grid.set(TilePos::new(x, y, gz), Tile::solid(floor));
                    self.grid.set(p, Tile::AIR);
                    self.roofs.insert(
                        p,
                        crate::building::Roof {
                            material: if wall == Material::STONE_BLOCK {
                                Material::WOOD
                            } else {
                                Material::THATCH
                            },
                            quality: 0.7,
                            built: self.meta.tick,
                            snow: 0.0,
                        },
                    );
                }
            }
        }
    }

    /// The staged representative scene: a small homestead by water with a field, a mine shaft
    /// down to a lit cave gallery, people at work, animals, items, and a hearth. ART REVIEW ONLY.
    ///
    /// The default game opening: a world whose ten survivors are generated, three begin at the
    /// landing site, seven are scattered. `opening_world_with` takes the setup screen's choice.
    /// `rerolls` are the setup screen's per-slot counters, so the three who step onto the map are
    /// exactly the three the cards showed. Without this, rerolling would be theatre.
    pub fn opening_world_with(
        seed: u64,
        chosen: &[u8; crate::opening::STARTING],
        rerolls: &[u32; crate::opening::PARTY],
    ) -> World {
        let mut s = crate::setup::WorldSetup::new(seed);
        s.chosen = *chosen;
        s.rerolls = *rerolls;
        World::from_setup(&s)
    }

    pub fn opening_world(seed: u64) -> World {
        Self::opening_world_with(seed, &[0, 1, 2], &[0; crate::opening::PARTY])
    }

    /// The staged representative scene: a small homestead by water with a field, a mine shaft
    /// down to a lit cave gallery, people at work, animals, items, and a hearth. ART REVIEW ONLY.
    pub fn stage_demo(seed: u64) -> World {
        let mut p = GenParams::small();
        p.width = 96;
        p.height = 96;
        let mut w = World::generate(seed, p);
        let fs = w.meta.founding_site;
        let gz = fs.z - 1;
        let (cx, cy) = (fs.x, fs.y);
        // Homestead clearing.
        w.flatten(cx - 12, cy - 9, 24, 18, gz);
        // Two houses and a workshop.
        w.build_house(cx - 10, cy - 8, 7, 6, gz, Material::WOOD, Material::WOOD);
        w.build_house(
            cx - 1,
            cy - 8,
            8,
            6,
            gz,
            Material::STONE_BLOCK,
            Material::STONE_BLOCK,
        );
        w.build_house(cx + 4, cy + 1, 6, 5, gz, Material::WOOD, Material::SOIL);
        let z = gz + 1;
        // Furniture.
        w.place_structure(
            StructureKind::Bed,
            TilePos::new(cx - 9, cy - 7, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Bed,
            TilePos::new(cx - 7, cy - 7, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Chest,
            TilePos::new(cx - 5, cy - 7, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Table,
            TilePos::new(cx - 8, cy - 5, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Stool,
            TilePos::new(cx - 9, cy - 5, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Stool,
            TilePos::new(cx - 7, cy - 5, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Hearth,
            TilePos::new(cx + 2, cy - 7, z),
            Material::STONE_BLOCK,
            true,
        );
        w.place_structure(
            StructureKind::CookingPot,
            TilePos::new(cx + 3, cy - 7, z),
            Material::STONE_BLOCK,
            true,
        );
        w.place_structure(
            StructureKind::Bed,
            TilePos::new(cx, cy - 7, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Bed,
            TilePos::new(cx + 5, cy - 7, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Shelf,
            TilePos::new(cx + 5, cy - 5, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Table,
            TilePos::new(cx + 2, cy - 5, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Workbench,
            TilePos::new(cx + 5, cy + 2, z),
            Material::WOOD,
            false,
        );
        w.place_structure(
            StructureKind::Anvil,
            TilePos::new(cx + 7, cy + 2, z),
            Material::STONE_BLOCK,
            false,
        );
        w.place_structure(
            StructureKind::Barrel,
            TilePos::new(cx + 8, cy + 3, z),
            Material::WOOD,
            false,
        );
        // Outside: well, lamp posts, crates, fence line, stockpile.
        w.place_structure(
            StructureKind::Well,
            TilePos::new(cx - 2, cy + 2, z),
            Material::STONE_BLOCK,
            false,
        );
        w.place_structure(
            StructureKind::Lamp,
            TilePos::new(cx - 4, cy - 1, z),
            Material::WOOD,
            true,
        );
        w.place_structure(
            StructureKind::Lamp,
            TilePos::new(cx + 3, cy - 1, z),
            Material::WOOD,
            true,
        );
        for i in 0..6 {
            w.place_structure(
                StructureKind::Stockpile,
                TilePos::new(cx - 11 + i, cy + 4, z),
                Material::SOIL,
                false,
            );
        }
        // Stacks of particular materials (C32), so a review frame shows them side by side.
        let g = |dx: i32, dy: i32| ItemPlace::Ground(TilePos::new(cx + dx, cy + dy, z));
        w.create_item_of(ItemKind::Log, Material::OAK, 12, g(-11, 4), 0.6, "scenario");
        w.create_item_of(
            ItemKind::Log,
            Material::BIRCH,
            7,
            g(-10, 4),
            0.6,
            "scenario",
        );
        w.create_item_of(
            ItemKind::StoneBlock,
            Material::GRANITE,
            20,
            g(-9, 4),
            0.6,
            "scenario",
        );
        for (i, (kind, m, n)) in [
            (ItemKind::StoneBlock, Material::MARBLE, 8),
            (ItemKind::StoneBlock, Material::SANDSTONE, 12),
            (ItemKind::Log, Material::PINE, 9),
            (ItemKind::Ore, Material::TIN_ORE, 6),
            (ItemKind::Gem, Material::GARNET, 2),
            (ItemKind::Hide, Material::WOLF_PELT, 1),
        ]
        .into_iter()
        .enumerate()
        {
            w.place_structure(
                StructureKind::Stockpile,
                TilePos::new(cx - 11 + i as i32, cy + 5, z),
                Material::SOIL,
                false,
            );
            w.create_item_of(kind, m, n, g(-11 + i as i32, 5), 0.6, "scenario");
        }
        // And the kiln's, the tanner's and the brewer's (C34).
        for (i, (kind, m, n)) in [
            (ItemKind::Vessel, Material::CLAY, 3),
            (ItemKind::Vessel, Material::GLASS, 2),
            (ItemKind::Leather, Material::DEER_HIDE, 2),
            (ItemKind::Ale, Material::AIR, 4),
            (ItemKind::Meal, Material::MEAT, 3),
            (ItemKind::Clay, Material::CLAY, 6),
        ]
        .into_iter()
        .enumerate()
        {
            w.place_structure(
                StructureKind::Stockpile,
                TilePos::new(cx - 11 + i as i32, cy + 6, z),
                Material::SOIL,
                false,
            );
            w.create_item_of(kind, m, n, g(-11 + i as i32, 6), 0.6, "scenario");
        }
        w.drop_item(ItemKind::Berries, 9, TilePos::new(cx - 8, cy + 4, z));
        w.drop_item(ItemKind::Firewood, 15, TilePos::new(cx - 7, cy + 4, z));
        w.drop_item(ItemKind::Meal, 3, TilePos::new(cx - 6, cy + 4, z));
        for i in 0..8 {
            w.place_structure(
                StructureKind::FencePost,
                TilePos::new(cx - 12 + i, cy + 7, z),
                Material::WOOD,
                false,
            );
        }
        w.place_structure(
            StructureKind::Crate,
            TilePos::new(cx + 1, cy + 5, z),
            Material::WOOD,
            false,
        );
        // A field of grain at mixed stages.
        for fy in 0..3 {
            for fx in 0..7 {
                let pos = TilePos::new(cx + 3 + fx, cy + 7 + fy, z);
                self_set_soil(&mut w.grid, pos);
                let stage = ((fx + fy) % 4) as f32 / 3.0;
                w.plants.push(Plant::new(
                    pos,
                    PlantKind::Grain,
                    0.25 + 0.75 * stage,
                    (fx * 7 + fy) as u32,
                ));
            }
        }
        // Mine entrance: a shaft two levels down (through the soil layer into rock) so the
        // gallery runs under solid ground, not under open sky.
        let mine = TilePos::new(cx + 10, cy - 3, z);
        let gal = gz - 1;
        w.grid.set(
            mine,
            Tile {
                material: Material::STONE_BLOCK,
                shape: Shape::StairsDown,
                flags: TileFlags::NONE,
            },
        );
        w.grid.set(
            TilePos::new(mine.x, mine.y, gz),
            Tile {
                material: Material::GRANITE,
                shape: Shape::StairsBoth,
                flags: TileFlags::NONE,
            },
        );
        w.grid.set(
            TilePos::new(mine.x, mine.y, gal),
            Tile {
                material: Material::GRANITE,
                shape: Shape::StairsUp,
                flags: TileFlags::NONE,
            },
        );
        let carve = |w: &mut World, p: TilePos| {
            w.grid.set(
                p,
                Tile {
                    material: Material::AIR,
                    shape: Shape::Open,
                    flags: TileFlags::NONE,
                },
            );
        };
        for dx in 0..10 {
            carve(&mut w, TilePos::new(mine.x + 1 + dx, mine.y, gal));
        }
        for dy in -2..=2 {
            for dx in 10..16 {
                carve(&mut w, TilePos::new(mine.x + 1 + dx, mine.y + dy, gal));
            }
        }
        for dy in -3..=3 {
            if dy % 2 == 0 {
                w.grid.set(
                    TilePos::new(mine.x + 17, mine.y + dy, gal),
                    Tile::solid(Material::IRON_ORE),
                );
            }
        }
        w.grid.set(
            TilePos::new(mine.x + 12, mine.y - 3, gal),
            Tile::solid(Material::COAL),
        );
        // A real gallery water tile keeps the art-only seepage overlay reviewable in this scene.
        w.grid.set(
            TilePos::new(mine.x + 15, mine.y + 2, gal),
            Tile {
                material: Material::WATER,
                shape: Shape::Open,
                flags: TileFlags(TileFlags::WATER),
            },
        );
        w.place_structure(
            StructureKind::Lamp,
            TilePos::new(mine.x + 4, mine.y, gal),
            Material::WOOD,
            true,
        );
        w.place_structure(
            StructureKind::Lamp,
            TilePos::new(mine.x + 13, mine.y + 1, gal),
            Material::WOOD,
            true,
        );
        w.drop_item(
            ItemKind::IronOre,
            6,
            TilePos::new(mine.x + 14, mine.y - 1, gal),
        );
        w.drop_item(
            ItemKind::Coal,
            4,
            TilePos::new(mine.x + 11, mine.y + 2, gal),
        );
        // People, placed at activities (the client animates poses by `facing`/kind hints later).
        let people = [
            (cx - 8, cy - 6, 0u8), // in house, by table
            (cx + 3, cy - 6, 3),   // cook at hearth, facing north
            (cx - 3, cy + 1, 0),   // hauling near stockpile
            (cx + 6, cy + 3, 1),   // at workbench
            (cx + 4, cy + 8, 0),   // field
            (cx + 7, cy + 9, 2),   // field
            (cx - 1, cy + 3, 2),   // by the well
            (cx - 6, cy + 2, 1),   // talking pair
            (cx - 5, cy + 2, 1),
            (cx + 9, cy - 1, 2), // walking to the mine
        ];
        for (x, y, f) in people {
            let id = w.spawn_agent(Species::Person, TilePos::new(x, y, z));
            w.agents.iter_mut().find(|a| a.id == id).unwrap().facing = f;
        }
        for (dx, dy) in [(9, 0), (14, 1), (15, -1)] {
            w.spawn_agent(Species::Person, TilePos::new(mine.x + dx, mine.y + dy, gal));
        }
        // Animals at the clearing edge and in the woods.
        w.spawn_agent(
            Species::Deer,
            TilePos::new(cx - 14, cy - 3, z).with_surface(&w.grid),
        );
        w.spawn_agent(
            Species::Deer,
            TilePos::new(cx - 15, cy - 1, z).with_surface(&w.grid),
        );
        w.spawn_agent(
            Species::Hare,
            TilePos::new(cx + 13, cy + 6, z).with_surface(&w.grid),
        );
        w.spawn_agent(
            Species::Boar,
            TilePos::new(cx + 14, cy + 9, z).with_surface(&w.grid),
        );
        // One of them has taken up an iron spear and mail (C33), for the review frame.
        if let Some(i) = w
            .agents
            .iter()
            .position(|a| a.species == Species::Person && a.age_years(0) >= 16)
        {
            let who = w.agents[i].id;
            for (kind, m) in [
                (ItemKind::Weapon, Material::IRON),
                (ItemKind::Armour, Material::IRON),
            ] {
                let id = w.create_lone_item(kind, 1, ItemPlace::Held(who), 0.7, "scenario");
                w.items.last_mut().expect("just made").material = m;
                match kind {
                    ItemKind::Weapon => w.agents[i].weapon = Some(id),
                    _ => w.agents[i].armour = Some(id),
                }
            }
        }
        // Mid-morning light.
        w.meta.tick = crate::time::TICKS_PER_HOUR * 10 + crate::time::TICKS_PER_DAY * 7;
        w.rebuild_caches();
        w
    }
}

fn self_set_soil(grid: &mut WorldGrid, pos: TilePos) {
    let top = TilePos::new(pos.x, pos.y, pos.z - 1);
    let mut t = grid.get(top);
    t.material = Material::SOIL;
    grid.set(top, t);
}

trait SurfaceSnap {
    fn with_surface(self, grid: &WorldGrid) -> TilePos;
}
impl SurfaceSnap for TilePos {
    fn with_surface(self, grid: &WorldGrid) -> TilePos {
        if grid.in_bounds(self) {
            grid.surface(self.x, self.y)
        } else {
            self
        }
    }
}
