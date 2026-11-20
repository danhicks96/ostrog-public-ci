//! Typed simulation events. One ring per tick batch; drained into the chronicle, the client
//! feed, and the short-term memory of agents who perceived them.

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, CommunityId, EventId, ItemId, StructureId, TilePos};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Severity {
    /// Routine: never shown in the feed, kept for memory and statistics only.
    Trace,
    /// Minor: feed at low priority, aggregated.
    Minor,
    /// Notable: feed line.
    Notable,
    /// Major: feed line + history + director attention.
    Major,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum EventKind {
    WorldCreated {
        seed: u64,
    },
    /// Off the map: a place ran out of food (true history; nobody here knows yet).
    SiteHungry {
        site: String,
    },
    /// Off the map: hungry people set out from a place toward the valley.
    BandSetOut {
        band: u32,
        site: String,
        size: u32,
    },
    /// Someone saw the band camped.
    StrangersSighted {
        agent: AgentId,
        band: u32,
        size: u32,
        at: TilePos,
    },
    /// Food taken from the stores by a band.
    Raided {
        band: u32,
        what: String,
        count: u32,
        /// Taken openly, after they were told they could.
        #[serde(default)]
        open: bool,
    },
    /// Someone told the strangers they could eat.
    Welcomed {
        agent: AgentId,
        band: u32,
    },
    /// The band left the valley.
    BandLeft {
        band: u32,
        site: String,
        took: u32,
        lost: u32,
        peaceful: bool,
    },
    /// An animal went for a person.
    Attacked {
        animal: AgentId,
        agent: AgentId,
        driven_off: bool,
    },
    /// The deity leaned one of the world's mechanisms (deity.rs). Never shown in the feed.
    DeityAnswered {
        deity: String,
        trigger: String,
        lever: String,
        place: String,
        strength: f32,
        days: u32,
    },
    /// A new trouble began while others were still going (arcs.rs).
    ArcTurn {
        now: String,
        during: Vec<String>,
        because: Option<String>,
    },
    /// A hard stretch eased (arcs.rs).
    ArcEased {
        days: u32,
        what: Vec<String>,
        lost: u32,
    },
    /// Creatures (creatures.rs).
    CrowsGathered {
        at: TilePos,
        crows: u32,
    },
    HartSighted {
        agent: AgentId,
    },
    BearDenned {
        bear: AgentId,
    },
    BearWoke {
        bear: AgentId,
    },
    DucksLeft {
        count: u32,
    },
    DucksReturned {
        count: u32,
    },
    /// A named beast was killed: its pelt is a trophy.
    BeastSlain {
        agent: AgentId,
        beast: String,
        hurt: u32,
    },
    /// Someone studied a record or tool from the old valley (C13).
    Studied {
        agent: AgentId,
        what: String,
        site: String,
        learned: String,
        success: bool,
    },
    /// Someone picked up a technique by watching it work.
    LearnedByWatching {
        agent: AgentId,
        from: AgentId,
        technique: String,
    },
    /// Someone who did not know a craft tried it at the bench (C70): worked it out, or
    /// spoiled the materials.
    CraftTried {
        agent: AgentId,
        craft: String,
        success: bool,
        spoiled: String,
    },
    /// The last of a community who knew a craft is gone (C70): the craft with them.
    CraftLost {
        community: CommunityId,
        craft: String,
        last: String,
        how: String,
    },
    /// A craft the community had lost is known among them again (C72, #259).
    CraftRegained {
        community: CommunityId,
        craft: String,
        by: String,
    },
    /// A community's wall stands whole round its houses (C73).
    Walled {
        community: CommunityId,
        gate: TilePos,
        side: String,
    },
    /// A craft taught at the bench (C70).
    Taught {
        teacher: AgentId,
        pupil: AgentId,
        craft: String,
    },
    /// A newcomer brought a tale of the old valley from a neighbouring place (lore.rs).
    TaleBrought {
        agent: AgentId,
        from: String,
        version: String,
        /// Their own family's memory, not something heard.
        #[serde(default)]
        kin: bool,
    },
    /// A tale passed from one to another; `changed` if it drifted in the telling.
    TaleTold {
        teller: AgentId,
        listener: AgentId,
        version: String,
        changed: bool,
    },
    /// Someone stood in a ruin nobody had explained to them and guessed.
    RuinWondered {
        agent: AgentId,
        site: String,
        guess: String,
    },
    /// A band stopped at the graves of the hamlet their people came from.
    BandRemembered {
        band: u32,
        from: String,
        site: String,
    },
    /// The valley's past (deep.rs), recorded before the arrival.
    OldFounded {
        site: String,
        by: String,
        /// The people they were of (the world's nearest, M3), or "" in a world without one.
        #[serde(default)]
        people: String,
    },
    OldTrouble {
        site: String,
        what: String,
    },
    /// The deity leaned on an old hamlet's fortunes in the valley's past (deep.rs).
    OldAnswered {
        site: String,
        deity: String,
        trigger: String,
        lever: String,
    },
    /// The founders' choices at the arrival (setup.rs): who watches, how deep the past runs,
    /// the valley, and what they hold in common.
    SetupChosen {
        deity: String,
        history_years: u64,
        valley: String,
        held: String,
        /// The world's peoples as chosen (P5): "the Grisk left out; the Skarri rare".
        #[serde(default)]
        peoples: String,
    },
    OldDied {
        who: String,
        site: String,
        cause: String,
    },
    OldEnded {
        site: String,
        why: String,
    },
    /// A stretch of roof came down (building.rs).
    RoofFell {
        at: TilePos,
        tiles: u32,
        hurt: Vec<AgentId>,
        because: String,
    },
    /// A house reached a stage: "marked out", "walls up", "roofed", "mended".
    HouseStage {
        community: CommunityId,
        at: TilePos,
        stage: String,
        by: Option<AgentId>,
    },
    /// A creature became known by a name.
    Named {
        agent: AgentId,
        name: String,
        because: String,
    },
    /// Someone beat out the flames on a tile.
    BeatOut {
        agent: AgentId,
        at: TilePos,
    },
    /// A moment of a survivor's life before the portal (past.rs), recorded at the arrival with
    /// the tick it happened at, which is below the world's epoch.
    Lived {
        name: String,
        slot: u8,
        age: u8,
        what: crate::past::Formative,
    },
    Arrived {
        agent: AgentId,
        community: Option<CommunityId>,
    },
    Born {
        agent: AgentId,
        mother: AgentId,
        father: Option<AgentId>,
    },
    Died {
        agent: AgentId,
        cause: String,
    },
    Ate {
        agent: AgentId,
        item: Option<ItemId>,
        quality: f32,
    },
    Slept {
        agent: AgentId,
        hours: f32,
        /// In a bed (FIXLOG 2026-10-23: read back from short memory after consolidation had
        /// cleared it, so every night was a night on the ground).
        #[serde(default)]
        in_bed: bool,
    },
    Foraged {
        agent: AgentId,
        at: TilePos,
        item: ItemId,
    },
    Harvested {
        agent: AgentId,
        at: TilePos,
    },
    Hauled {
        agent: AgentId,
        item: ItemId,
        to: TilePos,
    },
    Built {
        agent: AgentId,
        structure: StructureId,
        at: TilePos,
    },
    Mined {
        agent: AgentId,
        at: TilePos,
        found: Option<String>,
        /// The rock cut (#274); none in old saves.
        #[serde(default)]
        rock: Option<crate::world::Material>,
    },
    Cooked {
        agent: AgentId,
        item: ItemId,
    },
    Talked {
        a: AgentId,
        b: AgentId,
        warmth: f32,
    },
    Quarrelled {
        a: AgentId,
        b: AgentId,
    },
    PlanFailed {
        agent: AgentId,
        goal: String,
        reason: String,
    },
    Discovered {
        agent: AgentId,
        at: TilePos,
        what: String,
    },
    Starving {
        agent: AgentId,
    },
    Exhausted {
        agent: AgentId,
    },
    Weather {
        kind: String,
    },
    SeasonChanged {
        season: String,
        year: u64,
    },
    Dreamt {
        agent: AgentId,
        consolidated: u32,
    },
    Joined {
        agent: AgentId,
        community: CommunityId,
    },
    /// People who fled a settlement beyond the valley came to it (P9): of that settlement's
    /// people, with the cause the world's record gives.
    Refugees {
        agents: Vec<AgentId>,
        from: String,
        cause: String,
    },
    /// A stranger would not stay among people of another kind (P9).
    WouldNotStay {
        agent: AgentId,
        community: CommunityId,
        because: String,
    },
    TurnedAway {
        agent: AgentId,
        community: CommunityId,
        reason: String,
    },
    Split {
        from: CommunityId,
        to: CommunityId,
        leader: AgentId,
        members: u32,
        cause: String,
    },
    /// How a community answered a proposal to move (C68, #214): one line for the feed, the
    /// answers themselves being Minor rows of their own.
    Answered {
        leader: AgentId,
        went: u32,
        stayed: u32,
        /// Up to three of the stayers' reasons, each with a name.
        reasons: Vec<String>,
    },
    /// A whole community moved camp within the valley (C60), led by who proposed it.
    MovedCamp {
        community: CommunityId,
        leader: AgentId,
        from: TilePos,
        to: TilePos,
        cause: String,
    },
    /// The last of a community walked out of the valley to settle in the world (C60).
    LeftTheValley {
        community: CommunityId,
        leader: AgentId,
        people: u32,
        to: String,
        cause: String,
    },
    /// Nobody of a community is left in the valley (C60): died out, or gone.
    CommunityEnded {
        community: CommunityId,
        name: String,
        how: String,
    },
    Traded {
        from: CommunityId,
        to: CommunityId,
        carrier: AgentId,
        gave: String,
        received: String,
    },
    /// A choice visibly shaped by memory or relationship (so the viewer can see the cause).
    Chose {
        agent: AgentId,
        what: String,
        because: String,
    },
    RoleTaken {
        agent: AgentId,
        role: String,
    },
    /// Walked out of the valley (turned away, or no reason to stay).
    Left {
        agent: AgentId,
    },
    Paired {
        a: AgentId,
        b: AgentId,
    },
    /// A community made a custom out of what happened (`origin` is the recorded event).
    /// At the hearth in the evening someone told what they remember to whoever sat there.
    HearthTale {
        teller: AgentId,
        listeners: Vec<AgentId>,
        what: String,
    },
    /// A people came to be known by something: a custom, or a conviction most of them share.
    IdentityFormed {
        community: CommunityId,
        line: String,
    },
    /// Something a people was known by is no longer true of them.
    IdentityLapsed {
        community: CommunityId,
        line: String,
    },
    CustomFormed {
        community: CommunityId,
        custom: u32,
        label: String,
        origin: EventId,
        narrative: String,
    },
    CustomAdopted {
        agent: AgentId,
        community: CommunityId,
        custom: u32,
        origin: EventId,
        narrative: String,
        version: u32,
    },
    CustomLapsed {
        community: CommunityId,
        custom: u32,
        label: String,
    },
    RestDay {
        community: CommunityId,
        remembered: AgentId,
        custom: u32,
    },
    Hunted {
        agent: AgentId,
        prey: String,
    },
    CaveIn {
        agent: AgentId,
        at: TilePos,
        hurt: bool,
    },
    /// The roof of a dug excavation came down over every tile it no longer held
    /// (underground.rs); `sank` when the ground above sank into it.
    Collapse {
        by: AgentId,
        at: TilePos,
        tiles: u32,
        rock: String,
        sank: bool,
        hurt: Vec<AgentId>,
    },
    /// The air at the portal stone has thinned (thin.rs).
    ThinStirred {
        at: TilePos,
    },
    /// Two or more dreamt the same dream, of the home they left.
    SharedDream {
        dreamers: Vec<AgentId>,
        of: String,
    },
    /// A thing from home lay by the portal stone.
    ThingFromHome {
        item: crate::ids::ItemId,
        maker: String,
        from: String,
    },
    /// Someone knew a thing from home for what it was.
    Recognised {
        agent: AgentId,
        item: crate::ids::ItemId,
        maker: String,
    },
    /// An echo stood by the portal stone at dusk: one of the seven who died alone.
    EchoSeen {
        agent: AgentId,
        of: String,
        cause: String,
        /// Whether their fate was already known before this sighting.
        #[serde(default)]
        known: bool,
    },
    /// A patch of the wrong season near the stone.
    WrongWeather {
        at: TilePos,
        cold: bool,
    },
    /// Someone sat by the portal stone trying to understand it.
    ThinStudied {
        agent: AgentId,
        learned: bool,
    },
    /// The warding stones have quieted the thin place.
    ThinQuieted {
        at: TilePos,
    },
    /// Something in the wider world, from the realm's own history (realm/, M3): its ids are
    /// the realm's sites, figures, realms and peoples (World::realm), referenced in the
    /// chronicle by kind.
    Realm {
        what: crate::realm::sites::RealmEventKind,
    },
    /// Food left lying went off today (systems/spoil.rs): "30 grainsheaf, 4 berries".
    Rotted {
        what: String,
    },
    /// Someone back from a place beyond the valley tells what happened out there (realm/live.rs).
    NewsHeard {
        teller: AgentId,
        from: String,
        what: String,
    },
    /// People came into a cavern and its bats went up.
    BatsStirred {
        agent: AgentId,
        cavern: String,
        bats: u32,
    },
    /// One of the dead lies unremembered: no rite, no carving, nobody who remembers (P11).
    Restless {
        agent: AgentId,
    },
    /// Someone awake at dusk saw one of the restless dead where they died (P11); the
    /// sighting tells how they died, truly.
    GhostSeen {
        of: AgentId,
        by: AgentId,
        cause: String,
    },
    /// The restless dead is at rest: laid by a rite, carved, or remembered (P11).
    AtRest {
        agent: AgentId,
        how: String,
    },
    /// A curse took someone, for a recorded cause (P11): "the Beast-hunger".
    Cursed {
        agent: AgentId,
        condition: String,
        because: String,
    },
    /// A bite passed a curse, or blood was drunk (P11).
    Bitten {
        by: AgentId,
        of: AgentId,
    },
    /// Someone was gone from where they sleep on the night of the full moon (P11).
    Missing {
        agent: AgentId,
    },
    /// A starving person ate of the unburied dead (P11).
    AteDead {
        agent: AgentId,
        of: AgentId,
    },
    /// Someone who touched the underworld's stone died standing, a statue (P11).
    TurnedToStone {
        agent: AgentId,
        cavern: String,
    },
    /// A curse was lifted by someone who knew the unbinding (P11).
    Unbound {
        agent: AgentId,
        by: AgentId,
        condition: String,
    },
    /// Someone came to suspect another, from what they saw (P11): a suspicion, which can be
    /// wrong.
    Suspected {
        by: AgentId,
        of: AgentId,
        because: String,
    },
    /// Someone of the valley stood in a deep cavern for the first time (P10).
    Breached {
        agent: AgentId,
        cavern: String,
    },
    /// People of the deep came up through an opened deep cavern (P10), from a settlement of
    /// theirs under the land.
    FromBelow {
        agents: Vec<AgentId>,
        from: String,
    },
    /// A beast of the deep lairs near an opened deep cavern (P10): those who hear of it know
    /// its name from the world's record.
    BeastStirred {
        beast: String,
        cavern: String,
    },
    /// The burrower's tunnel broke into a dug gallery.
    BurrowedIn {
        agent: AgentId,
        at: TilePos,
    },
    /// A miner set about propping a gallery before cutting further.
    Propped {
        agent: AgentId,
        at: TilePos,
    },
    CaravanFormed {
        caravan: u32,
        community: CommunityId,
        leader: AgentId,
        members: u32,
        purpose: String,
    },
    CaravanLeft {
        caravan: u32,
        community: CommunityId,
        leader: AgentId,
        members: u32,
    },
    CaravanReturned {
        caravan: u32,
        community: CommunityId,
        leader: AgentId,
        members: u32,
        days: u32,
        site: String,
        received: String,
        story: String,
    },
    CaravanLost {
        caravan: u32,
        community: CommunityId,
        members: u32,
        last: String,
    },
    Crafted {
        agent: AgentId,
        kind: crate::items::ItemKind,
        tier: u8,
    },
    ToolBroke {
        agent: AgentId,
        tier: u8,
        /// Its name, or what it was ("copper tool").
        #[serde(default)]
        what: String,
        /// ": Halgar made it in spring of year 1, and it raised the walls of a house" or "".
        #[serde(default)]
        history: String,
    },
    /// A piece made or used well enough to be called something (legend.rs).
    ItemNamed {
        agent: AgentId,
        name: String,
        what: String,
        because: String,
    },
    /// Someone has come to love the tool in their hand.
    Cherished {
        agent: AgentId,
        what: String,
        name: Option<String>,
    },
    /// A named tool with a history people tell.
    SpokenOf {
        name: String,
        what: String,
        deeds: Vec<String>,
    },
    /// A cherished tool passed to kin.
    Inherited {
        agent: AgentId,
        from: AgentId,
        what: String,
    },
    NewClothes {
        agent: AgentId,
        tier: u8,
        /// The set with a story they wore before, now worn through: its name, or "clothes";
        /// "" when the old set had no story (life.rs).
        #[serde(default)]
        worn_through: String,
        /// ": Halgar made it in spring of year 1" or "".
        #[serde(default)]
        history: String,
    },
    Wounded {
        agent: AgentId,
        part: String,
        kind: String,
        cause: String,
    },
    Healed {
        agent: AgentId,
        part: String,
    },
    FellIll {
        agent: AgentId,
        /// Which sickness (#328); empty in old saves (a fever).
        #[serde(default)]
        what: String,
    },
    Recovered {
        agent: AgentId,
        #[serde(default)]
        what: String,
    },
    Ailing {
        agent: AgentId,
        what: String,
    },
    Tended {
        healer: AgentId,
        patient: AgentId,
    },
    /// A mind gave way: a tantrum, withdrawal, wandering off, a fight.
    Broke {
        agent: AgentId,
        kind: String,
    },
    FurnitureBroken {
        agent: AgentId,
        what: String,
    },
    Fought {
        a: AgentId,
        b: AgentId,
    },
    FireStarted {
        at: TilePos,
        cause: String,
    },
    FireOut {
        at: TilePos,
        burned: u32,
    },
    Burned {
        at: TilePos,
        what: String,
    },
    Doused {
        agent: AgentId,
        at: TilePos,
    },
    Flooded {
        tiles: u32,
    },
    Receded,
    /// Appended (C33): someone took up a weapon or armour the smiths made, because strangers
    /// threatened their people.
    TookUpArms {
        agent: AgentId,
        /// Its name, or what it is ("iron spear").
        what: String,
    },
    /// Appended (C34): those at the hearth who do not hold against drink shared some.
    DrankTogether {
        drinkers: Vec<AgentId>,
        /// "ale" or "berry wine".
        what: String,
    },
    /// One of a new pair gave the other a ring from the store.
    GaveRing {
        giver: AgentId,
        to: AgentId,
        /// "a silver ring set with a garnet", or its name.
        what: String,
    },
    /// Appended (C35): an artisan taken by a work, for a cause in their own past.
    Possessed {
        agent: AgentId,
        /// "remembering Ada, their child".
        because: String,
        /// The record it comes from.
        cause: EventId,
    },
    /// Someone carved an event of the record into a wall (engrave.rs).
    Engraved {
        agent: AgentId,
        at: TilePos,
        /// The event's words.
        what: String,
    },
    /// Appended (P7): the dead laid to rest by a rite (systems/rites.rs).
    LaidToRest {
        agent: AgentId,
        rite: crate::realm::culture::Rite,
        by: AgentId,
        /// The rite given at home for one whose body could not be reached (C68, #220).
        #[serde(default)]
        in_absence: bool,
    },
    /// Appended (P7): a living tree felled where someone who holds it wrong could see.
    TreeFelled {
        agent: AgentId,
        at: TilePos,
    },
    /// Appended (P0): someone wants a piece of work done, and why (project.rs).
    Proposed {
        agent: AgentId,
        /// "to build a smelter".
        what: String,
        /// "learned the craft from Brannoc, the smith".
        because: String,
    },
    /// Appended (C75): people stood together against a beast that went for one of them.
    StoodAgainst {
        beast: AgentId,
        by: Vec<AgentId>,
        felled: bool,
    },
    /// Appended (C75): a raiding band lost its nerve and ran for home.
    Routed {
        site: String,
        lost: u32,
    },
    /// Appended (C75): someone with the Beast-hunger went out in a beast's shape.
    Shifted {
        agent: AgentId,
    },
    /// Appended (C75): the beast of the deep came up through the breach.
    Rose {
        beast: AgentId,
        cavern: String,
    },
    /// Appended (C76): a host from a place with a grudge camped and laid siege.
    Besieged {
        site: String,
        size: u32,
    },
    /// Appended (C76): the siege is over. Since #402 only a siege the host gave up: a host that
    /// went home with what it came for is `HostSated`.
    SiegeLifted {
        site: String,
        days: u32,
        taken: u32,
        captives: u32,
        /// Why it was lifted (#402): "the wall held", "driven off", "tired of waiting". Empty
        /// in older records.
        #[serde(default)]
        why: String,
    },
    /// Appended (C76): someone of the valley was carried off.
    Captured {
        agent: AgentId,
        site: String,
    },
    /// Appended (C76): a raider brought down was kept.
    Held {
        agent: AgentId,
    },
    /// Appended (C76): their own people paid for one kept.
    Ransomed {
        agent: AgentId,
        site: String,
        food: u32,
    },
    /// Appended (C76): one kept, whom nobody came for, was taken in.
    TookIn {
        agent: AgentId,
    },
    /// Appended (C76): someone carried off was let go and came home.
    Returned {
        agent: AgentId,
        site: String,
    },
    /// Appended (C77): a community now follows this person in a domain.
    Followed {
        agent: AgentId,
        domain: String,
    },
    /// Appended (C77): an offence laid at someone's door.
    Accused {
        by: AgentId,
        of: AgentId,
        what: String,
    },
    /// Appended (C77): a judgement given.
    Judged {
        judge: AgentId,
        of: AgentId,
        what: String,
        verdict: String,
        /// They had not done it (the record knows; the judge did not).
        wrongly: bool,
    },
    /// Appended (C77): sent out of the community and the valley.
    Exiled {
        agent: AgentId,
        to: String,
    },
    /// Appended (C77): two people set against each other.
    Feud {
        a: AgentId,
        b: AgentId,
    },
    /// Appended (#273): what a breached cavern is.
    CavernFound {
        cavern: String,
        paces: u32,
        pool: bool,
        underworld: bool,
    },
    /// Appended (#273): a breach walled up again.
    Sealed {
        cavern: String,
    },
    /// Appended (#273): a beast of the deep found the breach walled.
    Barred {
        beast: String,
        cavern: String,
    },
    /// Appended (#278): a spell over the valley began.
    SpellBegan {
        what: String,
        how: String,
    },
    /// Appended (#278): it lifted.
    SpellLifted {
        what: String,
        days: u32,
    },
    /// Appended (#353): how a community is faring changed.
    Faring {
        community: String,
        now: String,
        why: String,
    },
    /// Appended (#353): one of a community left.
    LastOne {
        agent: AgentId,
        community: String,
    },
    /// Appended (#347): someone set out on their own, still of their community.
    WentAway {
        agent: AgentId,
        to: String,
        why: String,
    },
    /// Appended (#347): and came back.
    CameBack {
        agent: AgentId,
        from: String,
    },
    /// Appended (#338): how a rite for the dead went, when it was more than plain.
    RiteOutcome {
        agent: AgentId,
        by: AgentId,
        outcome: String,
        how: String,
    },
    /// Appended (#284): a limb gone bad taken to save its owner.
    LimbTaken {
        agent: AgentId,
        by: AgentId,
        part: String,
    },
    /// Appended (#284): a made limb fitted for a lost one.
    LimbMade {
        agent: AgentId,
        by: AgentId,
        what: String,
    },
    /// Appended (#295): two at odds made it up.
    MadeUp {
        a: AgentId,
        b: AgentId,
    },
    /// Appended (#295): a good turn repaid with food.
    Repaid {
        by: AgentId,
        to: AgentId,
    },
    /// Appended (nemesis): a stranger who fought someone of the valley and lived.
    RivalMade {
        name: String,
        foe: AgentId,
    },
    /// Appended (nemesis): a rival come back with their people.
    RivalReturned {
        name: String,
        title: String,
        foe: AgentId,
    },
    /// Appended (nemesis): a rival and their foe met again.
    RivalMet {
        name: String,
        title: String,
        foe: AgentId,
        meetings: u32,
    },
    /// Appended (nemesis): a rival fell.
    RivalFell {
        name: String,
        title: String,
        foe: AgentId,
        meetings: u32,
    },
    /// Appended (#299): a morning at the water.
    Fished {
        agent: AgentId,
        count: u32,
        ice: bool,
    },
    /// Appended (#316): a snare set.
    SnareSet {
        agent: AgentId,
    },
    /// Appended (#316): a snare took something, or someone.
    Snared {
        agent: AgentId,
    },
    /// Appended (#270): a beast drawn in by the smell of the stores.
    Drawn {
        animal: AgentId,
    },
    /// Appended (#270): a beast in the stores.
    StoresRaided {
        animal: AgentId,
        took: u32,
        broke: String,
    },
    /// Appended (#296): a child came of age.
    CameOfAge {
        agent: AgentId,
        best: String,
    },
    /// Appended (#346): a killing frost took the growing crop.
    Frosted {
        community: CommunityId,
        plants: u32,
    },
    /// Appended (#341): an orphan taken in.
    Fostered {
        child: AgentId,
        by: AgentId,
    },
    /// Appended (#274): a harder stone than any before, first cut.
    NewStone {
        agent: AgentId,
        stone: String,
    },
    /// Appended (#322): a secret society formed against someone.
    SocietyFormed {
        against: AgentId,
        members: u32,
    },
    /// Appended (#322): one of a society gave it away.
    Betrayed {
        by: AgentId,
        to: AgentId,
        members: u32,
    },
    /// Appended (#272): a cut let water in from the rock.
    StruckWater {
        agent: AgentId,
        tiles: u32,
    },
    /// Appended (#285): an animal tamed into a herd.
    Tamed {
        agent: AgentId,
        animal: AgentId,
    },
    /// Appended (#349): the valley first knows of a people beyond it.
    FirstWord {
        community: CommunityId,
        people: String,
    },
    /// Appended (#348): one who went with a split, kin left behind.
    Torn {
        agent: AgentId,
        with: CommunityId,
        and: CommunityId,
    },
    /// Appended (#297): one followed in the spirit read a sign.
    Omen {
        reader: AgentId,
        of: String,
        good: bool,
    },
    /// Appended (#390): standing water broke into a cut and filled the workings under it.
    WaterBrokeIn {
        agent: AgentId,
        tiles: u32,
    },
    /// Appended (#297): one followed in the spirit read the deity's answer aloud.
    DeityRead {
        reader: AgentId,
        deity: String,
        words: String,
        good: bool,
    },
    /// Appended (#297): one who holds signs are only weather said so to the reader.
    Scoffed {
        by: AgentId,
        at: AgentId,
    },
    /// Appended (#297): a sign read was borne out (a warning) or belied (a blessing) by a death.
    SignSettled {
        reader: AgentId,
        of: String,
        good: bool,
        dead: AgentId,
    },
    /// Appended (#297): someone came to heed the signs, or stopped.
    FaithTurned {
        agent: AgentId,
        heeds: bool,
    },
    /// Appended (#297): a community kept the season's observance.
    Observance {
        by: AgentId,
        community: CommunityId,
        season: String,
        came: u32,
        /// Kept at the community's shrine (shrines.rs).
        #[serde(default)]
        at_shrine: bool,
    },
    /// Appended (#297): a community let the season's observance lapse.
    ObservanceLapsed {
        community: CommunityId,
        season: String,
        why: String,
    },
    /// Appended (#322): someone cut out the wall over a way down to the deep.
    GateOpened {
        agent: AgentId,
        cavern: String,
    },
    /// Appended (#322): a society against someone came to terms with a people of the deep.
    DeepPact {
        against: AgentId,
        site: String,
        members: u32,
        /// The child they mean the deep's people to take (on its own record).
        #[serde(default)]
        child: Option<AgentId>,
    },
    /// Appended (#322): one of such a society let the deep's people up through the way.
    LetIn {
        by: AgentId,
        from: String,
        against: AgentId,
        /// The child they came up for (on its own record).
        #[serde(default)]
        child: Option<AgentId>,
    },
    /// Appended (#322): a child carried down into the deep.
    TakenBelow {
        agent: AgentId,
        from: String,
    },
    /// Appended (#322): the morning after, the way down found open.
    GateFoundOpen {
        cavern: String,
        child: Option<AgentId>,
    },
    /// Appended (#322): a society that let the deep up came out.
    SocietyExposed {
        against: AgentId,
        members: u32,
        from: String,
    },
    /// Appended (#393): a river in spate carried off what lay loose in its flood.
    CarriedOff {
        at: TilePos,
        things: u32,
    },
    /// Appended (#393): a falling flood left water standing in a hollow.
    PondLeft {
        at: TilePos,
        tiles: u32,
    },
    /// Appended (#393): a stream ran dry in a drought.
    RanDry {
        at: TilePos,
        water: String,
    },
    /// Appended (#393): a dry stream ran again after rain.
    RanAgain {
        at: TilePos,
        water: String,
    },
    /// Appended (#393): a body of water in spate; `spring` when the thaw raised it.
    HighWater {
        water: String,
        spring: bool,
    },
    /// Appended (#322): a party went down into the deep after a child taken there.
    WentDownAfter {
        agents: Vec<AgentId>,
        child: AgentId,
        to: String,
    },
    /// Appended (#322): the party came up again: "brought back", "lost" (one of them, `lost`),
    /// "hurt" or "nothing found".
    CameUpFromBelow {
        child: AgentId,
        by: AgentId,
        outcome: String,
        lost: Option<AgentId>,
        from: String,
    },
    /// Appended (#322): a child taken below seen again, grown and one of the deep's people.
    GrownBelow {
        agent: AgentId,
        from: String,
        years: u32,
    },
    /// Appended (#402): a host went home with all it came for; the siege was not lifted.
    HostSated {
        site: String,
        days: u32,
        taken: u32,
        captives: u32,
    },
    /// Appended (#297): a community's shrine was lost: "it burned", "it fell to ruin,
    /// untended", "it was left behind at the old camp".
    ShrineLost {
        community: CommunityId,
        why: String,
    },
    /// Appended (#297): someone was named keeper of a community's shrine and observances.
    KeeperNamed {
        agent: AgentId,
        community: CommunityId,
    },
    /// Appended (#297): `by` challenged `keeper` for the keeping; `won` when the challenger
    /// took it. How many stood with each.
    KeeperChallenged {
        by: AgentId,
        keeper: AgentId,
        community: CommunityId,
        won: bool,
        with_by: u32,
        with_keeper: u32,
    },
    /// Appended (#274): a besieging host brought down a stretch of the wall, of `of` (the
    /// stone or wood it was built of), after battering it `days` days.
    WallBroken {
        site: String,
        of: String,
        days: u32,
    },
    /// Appended (#404): one of a family in a feud takes it up from `from`, dead or gone.
    FeudInherited {
        agent: AgentId,
        from: AgentId,
        against: AgentId,
    },
    /// Appended (#404): a feud between two families is over, and how (`by` names a judge).
    FeudEnded {
        a: AgentId,
        b: AgentId,
        how: String,
        by: Option<AgentId>,
    },
    /// Appended (#404): `agent` died of a wound `by`, one of the valley, struck in a fight.
    Killing {
        agent: AgentId,
        by: AgentId,
    },
    /// Appended (#404): `agent` lost `part` to a wound `by`, one of the valley, struck in a fight.
    Maiming {
        agent: AgentId,
        by: AgentId,
        part: String,
    },
    /// Appended (#422): a beast heavy enough broke down a door (or a wall or fence) of `of`
    /// to get at the people inside.
    BrokeIn {
        beast: AgentId,
        of: String,
        door: bool,
    },
    /// Appended (#297): a thing of `of`, revered (`why`), became a relic of `community`.
    RelicMade {
        community: CommunityId,
        of: AgentId,
        what: String,
        why: String,
    },
    /// Appended (#297): a relic of `community` was lost: "it is gone", "it was carried out of
    /// the valley" (by `by`), "it was taken by those who split away" (by their leader).
    RelicLost {
        community: CommunityId,
        what: String,
        why: String,
        by: Option<AgentId>,
    },
    /// Appended (#297): `agent` set out in hard times (`why`) to pray at `to`.
    Pilgrimage {
        agent: AgentId,
        to: String,
        why: String,
    },
    /// Appended (#297): `leader`, turned back in a challenge for the keeping, and those who
    /// stood with them hold the signs apart from `keeper`: `members` in all.
    FactionFormed {
        leader: AgentId,
        keeper: AgentId,
        community: CommunityId,
        members: u32,
    },
    /// Appended (#297): a faction kept the season's observance on its own, led by `by`.
    FactionObservance {
        by: AgentId,
        community: CommunityId,
        came: u32,
    },
    /// Appended (#297): a faction is over without splitting off: "came back to the keeper",
    /// "took the keeping", "lost the one who led them".
    FactionEnded {
        leader: AgentId,
        community: CommunityId,
        how: String,
    },
    /// Appended (#322): `agent`, taken below `years` years ago and grown among the people of
    /// `from`, came up to seek their family.
    SoughtKin {
        agent: AgentId,
        from: String,
        years: u32,
    },
    /// Appended (#322): one grown below met their family (`by`, or nobody): "taken back",
    /// "known, and turned back below", "not known, and turned back below".
    KinMet {
        agent: AgentId,
        by: Option<AgentId>,
        outcome: String,
    },
    /// Appended (#322): `by` saw `members` of a society gathered apart by night, one that meets
    /// against `against`, and suspects them.
    MeetingSeen {
        by: AgentId,
        against: AgentId,
        members: u32,
    },
    /// Appended (#433): someone has had nothing to drink for too long.
    Parched {
        agent: AgentId,
    },
    /// Appended (#308): the true-dream cried out in a sleeper's night, of `what` — true to
    /// the record; `cause` is their curse's row.
    TrueDream {
        agent: AgentId,
        what: String,
        cause: EventId,
    },
    /// Appended (#319): food standing under flood water rotted; `cause` is the flood's row.
    Drenched {
        at: TilePos,
        what: String,
        count: u32,
        cause: EventId,
    },
    /// Appended (#319): vermin drawn to flood-rotted food; `cause` is the rot's row.
    VerminDrawn {
        at: TilePos,
        cause: EventId,
    },
    /// Appended (#319): a sleeper bitten by vermin; `cause` is the outbreak's row.
    VerminBite {
        agent: AgentId,
        at: TilePos,
        cause: EventId,
    },
    /// Appended (#319): a vermin bite festered; `cause` is the bite's row.
    Festered {
        agent: AgentId,
        cause: EventId,
    },
    /// Appended (#405): a community set about sinking its shaft below the quarry, for `why`
    /// ("iron and copper for the smelter").
    MineSunk {
        community: CommunityId,
        why: String,
    },
    /// Appended (#405): a community's shaft first came down into the deep's rock.
    ReachedDeep {
        community: CommunityId,
        rock: String,
    },
    /// Appended (#405): after its people broke into a deep cavern, a community walled the way
    /// (`walled`) or dug on.
    DeepChosen {
        community: CommunityId,
        cavern: String,
        walled: bool,
    },
    /// Appended (#452): `teller` told `listener` how `dead` died; `version` is the telling as
    /// told, and `changed` says it was not the teller's own version (accounts.rs).
    AccountTold {
        teller: AgentId,
        listener: AgentId,
        dead: AgentId,
        version: String,
        changed: bool,
    },
    /// Appended (#452): someone who never knew the dead stood at a grave of the colony's and
    /// read it as `guess` (accounts.rs).
    GraveRead {
        agent: AgentId,
        at: TilePos,
        guess: String,
    },
    /// Appended (#566): a person was given no rite, and why ("none of their own people",
    /// "nobody of their people was left", "nobody came to them in a day").
    LeftUnrested {
        agent: AgentId,
        why: String,
    },
    /// Appended (#454): someone no longer holds the role they had (duties.rs): they stopped
    /// doing that work.
    RoleLeft {
        agent: AgentId,
        role: String,
    },
    /// Appended (#455): something moved below `cavern`, heard by those near the way (stirring.rs).
    StirBelow {
        cavern: String,
        heard: Vec<AgentId>,
    },
    /// Appended (#423): a blow, the cold or hunger took some of a beast whose body is not a
    /// skeleton: members of a swarm, segments of a chain, part of a soft mass. `part` is the
    /// plan's word for them; `left` how many are left (a soft body's hundredths).
    Thinned {
        agent: AgentId,
        part: String,
        lost: u32,
        left: u32,
        cause: String,
    },
    /// Appended (#486): a wild animal of a kind gone from the valley came back over the hills,
    /// there being food for it again (ecology.rs `incomers`).
    CameOver {
        agent: AgentId,
    },
    /// Appended (#319): a community's stores ran short in the cold months, `food_hours` a head
    /// left; the root of the lean-winter cascade.
    StoresFailed {
        community: CommunityId,
        food_hours: u32,
    },
    /// Appended (#319): one of a community whose stores failed is weak from hunger; `cause` is
    /// the stores' row.
    Weakened {
        agent: AgentId,
        cause: EventId,
    },
    /// Appended (#319): the weakened caught a sickness; `cause` is the weakening's row.
    SickenedWeak {
        agent: AgentId,
        what: String,
        cause: EventId,
    },
    /// Appended (#319): rain came in where a roof had burned or fallen and spoiled the food
    /// under it; `cause` is the row of the roof's burning or fall.
    RainSpoiled {
        at: TilePos,
        what: String,
        count: u32,
        cause: EventId,
    },
    /// Appended (#319): the last healer of a community died; `cause` is the death's row.
    HealerLost {
        community: CommunityId,
        healer: AgentId,
        cause: EventId,
    },
    /// Appended (#319): a wound festered with no healer left to dress it; `cause` is the
    /// `HealerLost` row.
    WoundFestered {
        agent: AgentId,
        cause: EventId,
    },
    /// Appended (#308): someone came back from sitting by the portal stone unsettled and tired;
    /// nobody can say why (resonance.rs).
    CameBackUnsettled {
        agent: AgentId,
    },
    /// Appended (#308): someone would not cut the face at `at`, and could not say why.
    WouldNotCut {
        agent: AgentId,
        at: TilePos,
    },
    /// Appended (#423): a blow cut a soft body in two, and the piece it took lives on as `into`.
    Divided {
        agent: AgentId,
        into: AgentId,
    },
    /// Appended (#423): a beast got past what stops a walker, by what its body can do: "climbed
    /// up", "climbed down", "squeezed through" (the gaps of a built wall, `at`).
    Crept {
        agent: AgentId,
        at: TilePos,
        how: String,
    },
    /// Appended (#483): a supported upper room lost its lower support and came down
    /// (building.rs).
    UpperFloorFell {
        at: TilePos,
        tiles: u32,
        hurt: Vec<AgentId>,
        because: String,
    },
    /// Appended (#1830): heavy traffic over a young tree killed it, as Dwarf Fortress does
    /// (paths.rs). A trace only; no feed line.
    SaplingTrampled {
        at: TilePos,
    },
    /// Appended: what a breakdown was over, pushed right after its `Broke` at the same
    /// tick. `about` is the person it was about, if any; `cause` is a few words.
    BrokeOver {
        agent: AgentId,
        about: Option<AgentId>,
        cause: String,
    },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub tick: u64,
    pub at: Option<TilePos>,
    pub severity: Severity,
    pub kind: EventKind,
}

impl Event {
    /// Agents directly involved (for memory and chronicle entity refs).
    pub fn participants(&self) -> Vec<AgentId> {
        use EventKind::*;
        match &self.kind {
            Arrived { agent, .. }
            | Ate { agent, .. }
            | Slept { agent, .. }
            | Foraged { agent, .. }
            | Harvested { agent, .. }
            | Hauled { agent, .. }
            | Built { agent, .. }
            | Mined { agent, .. }
            | Cooked { agent, .. }
            | PlanFailed { agent, .. }
            | Discovered { agent, .. }
            | Starving { agent }
            | Parched { agent }
            | Exhausted { agent }
            | Dreamt { agent, .. }
            | Joined { agent, .. }
            | TurnedAway { agent, .. }
            | WouldNotStay { agent, .. }
            | Chose { agent, .. }
            | RoleTaken { agent, .. }
            | Left { agent }
            | CustomAdopted { agent, .. }
            | Hunted { agent, .. }
            | CaveIn { agent, .. }
            | Propped { agent, .. }
            | BatsStirred { agent, .. }
            | Breached { agent, .. }
            | Restless { agent }
            | AtRest { agent, .. }
            | Cursed { agent, .. }
            | Missing { agent }
            | TurnedToStone { agent, .. }
            | Recognised { agent, .. }
            | EchoSeen { agent, .. }
            | ThinStudied { agent, .. }
            | CaravanFormed { leader: agent, .. }
            | CaravanLeft { leader: agent, .. }
            | CaravanReturned { leader: agent, .. }
            | Crafted { agent, .. }
            | ToolBroke { agent, .. }
            | TookUpArms { agent, .. }
            | Engraved { agent, .. }
            | Proposed { agent, .. }
            | TreeFelled { agent, .. }
            | Possessed { agent, .. }
            | NewClothes { agent, .. }
            | Wounded { agent, .. }
            | Healed { agent, .. }
            | FellIll { agent, .. }
            | Recovered { agent, .. }
            | Ailing { agent, .. }
            | Died { agent, .. } => vec![*agent],
            Tended { healer, patient } => vec![*healer, *patient],
            Broke { agent, .. } | FurnitureBroken { agent, .. } => vec![*agent],
            BrokeOver { agent, about, .. } => std::iter::once(*agent).chain(*about).collect(),
            LaidToRest { agent, by, .. } => vec![*agent, *by],
            Fought { a, b } => vec![*a, *b],
            StoodAgainst { beast, by, .. } => {
                let mut v = by.clone();
                v.push(*beast);
                v
            }
            Shifted { agent } => vec![*agent],
            Rose { beast, .. } => vec![*beast],
            Routed { .. } => vec![],
            SocietyFormed { against, .. } => vec![*against],
            Betrayed { by, to, .. } => vec![*by, *to],
            NewStone { agent, .. } => vec![*agent],
            Omen { reader, .. } => vec![*reader],
            WaterBrokeIn { agent, .. } => vec![*agent],
            CarriedOff { .. } | PondLeft { .. } | RanDry { .. } | RanAgain { .. } => vec![],
            HighWater { .. } => vec![],
            DeityRead { reader, .. } => vec![*reader],
            Scoffed { by, at } => vec![*by, *at],
            SignSettled { reader, dead, .. } => vec![*reader, *dead],
            FaithTurned { agent, .. } => vec![*agent],
            Observance { by, .. } => vec![*by],
            ObservanceLapsed { .. } => vec![],
            FirstWord { .. } => vec![],
            Torn { agent, .. } => vec![*agent],
            Besieged { .. } | SiegeLifted { .. } | HostSated { .. } => vec![],
            Captured { agent, .. }
            | Held { agent }
            | Ransomed { agent, .. }
            | TookIn { agent }
            | Returned { agent, .. } => vec![*agent],
            Followed { agent, .. } | Exiled { agent, .. } => vec![*agent],
            Accused { by, of, .. } => vec![*of, *by],
            Judged { judge, of, .. } => vec![*of, *judge],
            Feud { a, b } => vec![*a, *b],
            CavernFound { .. } | Sealed { .. } | Barred { .. } => vec![],
            GateOpened { agent, .. } => vec![*agent],
            DeepPact { against, child, .. } => [*against].into_iter().chain(*child).collect(),
            SocietyExposed { against, .. } => vec![*against],
            FeudInherited { agent, from, .. } => vec![*agent, *from],
            FeudEnded { a, b, .. } => vec![*a, *b],
            Killing { agent, by } | Maiming { agent, by, .. } => vec![*agent, *by],
            BrokeIn { beast, .. } => vec![*beast],
            RelicMade { of, .. } => vec![*of],
            RelicLost { by, .. } => by.iter().copied().collect(),
            Pilgrimage { agent, .. } => vec![*agent],
            FactionFormed { leader, keeper, .. } => vec![*leader, *keeper],
            FactionObservance { by, .. } => vec![*by],
            FactionEnded { leader, .. } => vec![*leader],
            SoughtKin { agent, .. } => vec![*agent],
            KinMet { agent, by, .. } => [*agent].into_iter().chain(*by).collect(),
            MeetingSeen { by, against, .. } => vec![*by, *against],
            LetIn {
                by, against, child, ..
            } => [*by, *against].into_iter().chain(*child).collect(),
            TakenBelow { agent, .. } => vec![*agent],
            GateFoundOpen { child, .. } => child.iter().copied().collect(),
            WentDownAfter { agents, child, .. } => agents.iter().copied().chain([*child]).collect(),
            CameUpFromBelow {
                child, by, lost, ..
            } => [*child, *by].into_iter().chain(*lost).collect(),
            GrownBelow { agent, .. } => vec![*agent],
            ShrineLost { .. } => vec![],
            KeeperNamed { agent, .. } => vec![*agent],
            KeeperChallenged { by, keeper, .. } => vec![*by, *keeper],
            WallBroken { .. } => vec![],
            SpellBegan { .. } | SpellLifted { .. } => vec![],
            Faring { .. } => vec![],
            LastOne { agent, .. } => vec![*agent],
            WentAway { agent, .. } | CameBack { agent, .. } => vec![*agent],
            RiteOutcome { agent, by, .. } => vec![*agent, *by],
            LimbTaken { agent, by, .. } | LimbMade { agent, by, .. } => vec![*agent, *by],
            MadeUp { a, b } => vec![*a, *b],
            Repaid { by, to } => vec![*by, *to],
            RivalMade { foe, .. }
            | RivalReturned { foe, .. }
            | RivalMet { foe, .. }
            | RivalFell { foe, .. } => vec![*foe],
            Fished { agent, .. } => vec![*agent],
            Tamed { agent, animal } => vec![*agent, *animal],
            SnareSet { agent } | Snared { agent } => vec![*agent],
            Drawn { animal } | StoresRaided { animal, .. } => vec![*animal],
            CameOfAge { agent, .. } => vec![*agent],
            Frosted { .. } => vec![],
            MineSunk { .. } | ReachedDeep { .. } | DeepChosen { .. } => vec![],
            StirBelow { heard, .. } => heard.clone(),
            CameBackUnsettled { agent } | WouldNotCut { agent, .. } => vec![*agent],
            AccountTold {
                teller, listener, ..
            } => vec![*teller, *listener],
            GraveRead { agent, .. } => vec![*agent],
            RoleLeft { agent, .. } => vec![*agent],
            Thinned { agent, .. } => vec![*agent],
            CameOver { agent } => vec![*agent],
            StoresFailed { .. } | RainSpoiled { .. } => vec![],
            HealerLost { healer, .. } => vec![*healer],
            Weakened { agent, .. } | SickenedWeak { agent, .. } | WoundFestered { agent, .. } => {
                vec![*agent]
            }
            Divided { agent, into } => vec![*agent, *into],
            Crept { agent, .. } => vec![*agent],
            Fostered { child, by } => vec![*child, *by],
            LeftUnrested { agent, .. } => vec![*agent],
            StruckWater { agent, .. } => vec![*agent],
            Doused { agent, .. } | BeatOut { agent, .. } => vec![*agent],
            StrangersSighted { agent, .. } | Welcomed { agent, .. } | Named { agent, .. } => {
                vec![*agent]
            }
            Attacked { animal, agent, .. } => vec![*agent, *animal],
            SiteHungry { .. } | BandSetOut { .. } | Raided { .. } | BandLeft { .. } => vec![],
            DeityAnswered { .. } | ArcTurn { .. } | ArcEased { .. } => vec![],
            RoofFell { hurt, .. } | UpperFloorFell { hurt, .. } => hurt.clone(),
            SaplingTrampled { .. } => vec![],
            ItemNamed { agent, .. } | Cherished { agent, .. } => vec![*agent],
            Inherited { agent, from, .. } => vec![*agent, *from],
            SpokenOf { .. } => vec![],
            Drenched { .. } | VerminDrawn { .. } => vec![],
            VerminBite { agent, .. } | Festered { agent, .. } => vec![*agent],
            HartSighted { agent } | BeastSlain { agent, .. } => vec![*agent],
            BearDenned { bear } | BearWoke { bear } => vec![*bear],
            CrowsGathered { .. } | DucksLeft { .. } | DucksReturned { .. } => vec![],
            TaleBrought { agent, .. } | RuinWondered { agent, .. } => vec![*agent],
            TaleTold {
                teller, listener, ..
            } => vec![*teller, *listener],
            BandRemembered { .. } => vec![],
            Studied { agent, .. } => vec![*agent],
            LearnedByWatching { agent, from, .. } => vec![*agent, *from],
            CraftTried { agent, .. } => vec![*agent],
            CraftLost { .. } | CraftRegained { .. } | Walled { .. } => vec![],
            Taught { teacher, pupil, .. } => vec![*teacher, *pupil],
            OldFounded { .. } | OldTrouble { .. } | OldDied { .. } | OldEnded { .. } => vec![],
            OldAnswered { .. } | SetupChosen { .. } => vec![],
            HouseStage { by, .. } => by.iter().copied().collect(),
            FireStarted { .. } | FireOut { .. } | Burned { .. } | Flooded { .. } | Receded => {
                vec![]
            }
            CaravanLost { .. } => vec![],
            TrueDream { agent, .. } => vec![*agent],
            Born {
                agent,
                mother,
                father,
            } => {
                let mut v = vec![*agent, *mother];
                if let Some(f) = father {
                    v.push(*f);
                }
                v
            }
            Talked { a, b, .. } | Quarrelled { a, b } | Paired { a, b } => vec![*a, *b],
            Split { leader, .. } | Answered { leader, .. } => vec![*leader],
            MovedCamp { leader, .. } | LeftTheValley { leader, .. } => vec![*leader],
            CommunityEnded { .. } => vec![],
            Traded { carrier, .. } => vec![*carrier],
            RestDay { remembered, .. } => vec![*remembered],
            WorldCreated { .. }
            | Lived { .. }
            | Weather { .. }
            | SeasonChanged { .. }
            | CustomFormed { .. }
            | CustomLapsed { .. } => vec![],
            HearthTale {
                teller, listeners, ..
            } => std::iter::once(*teller)
                .chain(listeners.iter().copied())
                .collect(),
            IdentityFormed { .. } | IdentityLapsed { .. } => vec![],
            BurrowedIn { .. } => vec![],
            SharedDream { dreamers, .. } => dreamers.clone(),
            FromBelow { agents, .. } => agents.clone(),
            GhostSeen { of, by, .. } => vec![*of, *by],
            Bitten { by, of } => vec![*by, *of],
            AteDead { agent, of } => vec![*agent, *of],
            Unbound { agent, by, .. } => vec![*agent, *by],
            Suspected { by, of, .. } => vec![*by, *of],
            BeastStirred { .. } => vec![],
            Refugees { agents, .. } => agents.clone(),
            DrankTogether { drinkers, .. } => drinkers.clone(),
            GaveRing { giver, to, .. } => vec![*giver, *to],
            ThinStirred { .. }
            | ThingFromHome { .. }
            | WrongWeather { .. }
            | ThinQuieted { .. } => {
                vec![]
            }
            Realm { .. } => vec![],
            NewsHeard { teller, .. } => vec![*teller],
            Rotted { .. } => vec![],
            Collapse { by, hurt, .. } => std::iter::once(*by).chain(hurt.iter().copied()).collect(),
        }
    }

    /// Things other than agents this event is about, as (kind, id) for the chronicle's
    /// entity references: the realm's sites, figures, realms ("polity") and peoples.
    pub fn refs(&self) -> Vec<(&'static str, u64)> {
        match &self.kind {
            EventKind::Realm { what } => crate::realm::chronicle::refs(what),
            _ => Vec::new(),
        }
    }

    /// Stable kind name for the chronicle.
    pub fn kind_name(&self) -> &'static str {
        use EventKind::*;
        match self.kind {
            WorldCreated { .. } => "world_created",
            Lived { .. } => "lived",
            BeatOut { .. } => "beat_out",
            SiteHungry { .. } => "site_hungry",
            DeityAnswered { .. } => "deity_answered",
            ArcTurn { .. } => "arc_turn",
            ArcEased { .. } => "arc_eased",
            RoofFell { .. } => "roof_fell",
            UpperFloorFell { .. } => "upper_floor_fell",
            SaplingTrampled { .. } => "sapling_trampled",
            ItemNamed { .. } => "item_named",
            CrowsGathered { .. } => "crows_gathered",
            HartSighted { .. } => "hart_sighted",
            BearDenned { .. } => "bear_denned",
            BearWoke { .. } => "bear_woke",
            DucksLeft { .. } => "ducks_left",
            DucksReturned { .. } => "ducks_returned",
            BeastSlain { .. } => "beast_slain",
            TaleBrought { .. } => "tale_brought",
            TaleTold { .. } => "tale_told",
            RuinWondered { .. } => "ruin_wondered",
            BandRemembered { .. } => "band_remembered",
            Studied { .. } => "studied",
            LearnedByWatching { .. } => "learned_by_watching",
            CraftTried { .. } => "craft_tried",
            CraftLost { .. } => "craft_lost",
            CraftRegained { .. } => "craft_regained",
            Walled { .. } => "walled",
            Taught { .. } => "taught",
            OldFounded { .. } => "old_founded",
            OldTrouble { .. } => "old_trouble",
            OldAnswered { .. } => "old_answered",
            SetupChosen { .. } => "setup_chosen",
            OldDied { .. } => "old_died",
            OldEnded { .. } => "old_ended",
            Cherished { .. } => "cherished",
            SpokenOf { .. } => "spoken_of",
            Inherited { .. } => "inherited",
            HouseStage { .. } => "house_stage",
            BandSetOut { .. } => "band_set_out",
            StrangersSighted { .. } => "strangers_sighted",
            Raided { .. } => "raided",
            Welcomed { .. } => "welcomed",
            BandLeft { .. } => "band_left",
            Attacked { .. } => "attacked",
            Named { .. } => "named",
            Arrived { .. } => "arrived",
            Born { .. } => "born",
            Died { .. } => "died",
            Ate { .. } => "ate",
            Slept { .. } => "slept",
            Foraged { .. } => "foraged",
            Harvested { .. } => "harvested",
            Hauled { .. } => "hauled",
            Built { .. } => "built",
            Mined { .. } => "mined",
            Cooked { .. } => "cooked",
            Talked { .. } => "talked",
            Quarrelled { .. } => "quarrelled",
            PlanFailed { .. } => "plan_failed",
            Discovered { .. } => "discovered",
            Starving { .. } => "starving",
            Parched { .. } => "parched",
            TrueDream { .. } => "true_dream",
            Drenched { .. } => "drenched",
            VerminDrawn { .. } => "vermin_drawn",
            VerminBite { .. } => "vermin_bite",
            Festered { .. } => "festered",
            Exhausted { .. } => "exhausted",
            Weather { .. } => "weather",
            SeasonChanged { .. } => "season_changed",
            Dreamt { .. } => "dreamt",
            Joined { .. } => "joined",
            TurnedAway { .. } => "turned_away",
            WouldNotStay { .. } => "would_not_stay",
            Refugees { .. } => "refugees",
            Split { .. } => "split",
            Answered { .. } => "answered",
            MovedCamp { .. } => "moved_camp",
            LeftTheValley { .. } => "left_the_valley",
            CommunityEnded { .. } => "community_ended",
            Traded { .. } => "traded",
            Chose { .. } => "chose",
            RoleTaken { .. } => "role",
            Left { .. } => "left",
            Paired { .. } => "paired",
            CustomFormed { .. } => "custom_formed",
            HearthTale { .. } => "hearth_tale",
            IdentityFormed { .. } => "identity_formed",
            IdentityLapsed { .. } => "identity_lapsed",
            CustomAdopted { .. } => "custom_adopted",
            CustomLapsed { .. } => "custom_lapsed",
            RestDay { .. } => "rest_day",
            Hunted { .. } => "hunted",
            CaveIn { .. } => "cave_in",
            Collapse { .. } => "collapse",
            Propped { .. } => "propped",
            BatsStirred { .. } => "bats_stirred",
            Breached { .. } => "breached",
            Restless { .. } => "restless",
            GhostSeen { .. } => "ghost_seen",
            AtRest { .. } => "at_rest",
            Cursed { .. } => "cursed",
            Bitten { .. } => "bitten",
            Missing { .. } => "missing",
            AteDead { .. } => "ate_dead",
            TurnedToStone { .. } => "turned_to_stone",
            Unbound { .. } => "unbound",
            Suspected { .. } => "suspected",
            FromBelow { .. } => "from_below",
            BeastStirred { .. } => "beast_stirred",
            ThinStirred { .. } => "thin_stirred",
            SharedDream { .. } => "shared_dream",
            ThingFromHome { .. } => "thing_from_home",
            Recognised { .. } => "recognised",
            EchoSeen { .. } => "echo_seen",
            WrongWeather { .. } => "wrong_weather",
            ThinStudied { .. } => "thin_studied",
            ThinQuieted { .. } => "thin_quieted",
            Realm { ref what } => crate::realm::chronicle::kind_name(what),
            NewsHeard { .. } => "news_heard",
            Rotted { .. } => "rotted",
            BurrowedIn { .. } => "burrowed_in",
            CaravanFormed { .. } => "caravan_formed",
            CaravanLeft { .. } => "caravan_left",
            CaravanReturned { .. } => "caravan_returned",
            CaravanLost { .. } => "caravan_lost",
            Crafted { .. } => "crafted",
            ToolBroke { .. } => "tool_broke",
            TookUpArms { .. } => "took_up_arms",
            DrankTogether { .. } => "drank_together",
            GaveRing { .. } => "gave_ring",
            Engraved { .. } => "engraved",
            Proposed { .. } => "proposed",
            LaidToRest { .. } => "laid_to_rest",
            TreeFelled { .. } => "tree_felled",
            Possessed { .. } => "possessed",
            NewClothes { .. } => "new_clothes",
            Wounded { .. } => "wounded",
            Healed { .. } => "healed",
            FellIll { .. } => "fell_ill",
            Recovered { .. } => "recovered",
            Ailing { .. } => "ailing",
            Tended { .. } => "tended",
            Broke { .. } => "broke",
            FurnitureBroken { .. } => "furniture_broken",
            Fought { .. } => "fought",
            StoodAgainst { .. } => "stood_against",
            Routed { .. } => "routed",
            Shifted { .. } => "shifted",
            Rose { .. } => "rose",
            SocietyFormed { .. } => "society_formed",
            Betrayed { .. } => "betrayed",
            NewStone { .. } => "new_stone",
            Omen { .. } => "omen",
            WaterBrokeIn { .. } => "water_broke_in",
            CarriedOff { .. } => "carried_off",
            PondLeft { .. } => "pond_left",
            RanDry { .. } => "ran_dry",
            RanAgain { .. } => "ran_again",
            HighWater { .. } => "high_water",
            DeityRead { .. } => "deity_read",
            Scoffed { .. } => "scoffed",
            SignSettled { .. } => "sign_settled",
            FaithTurned { .. } => "faith_turned",
            Observance { .. } => "observance",
            ObservanceLapsed { .. } => "observance_lapsed",
            FirstWord { .. } => "first_word",
            Torn { .. } => "torn",
            Besieged { .. } => "besieged",
            SiegeLifted { .. } => "siege_lifted",
            HostSated { .. } => "host_sated",
            Captured { .. } => "captured",
            Held { .. } => "held",
            Ransomed { .. } => "ransomed",
            TookIn { .. } => "took_in",
            Returned { .. } => "returned",
            Followed { .. } => "followed",
            Accused { .. } => "accused",
            Judged { .. } => "judged",
            Exiled { .. } => "exiled",
            Feud { .. } => "feud",
            CavernFound { .. } => "cavern_found",
            Sealed { .. } => "sealed",
            GateOpened { .. } => "gate_opened",
            DeepPact { .. } => "deep_pact",
            SocietyExposed { .. } => "society_exposed",
            FeudInherited { .. } => "feud_inherited",
            FeudEnded { .. } => "feud_ended",
            Killing { .. } => "killing",
            Maiming { .. } => "maiming",
            BrokeIn { .. } => "broke_in",
            RelicMade { .. } => "relic_made",
            RelicLost { .. } => "relic_lost",
            Pilgrimage { .. } => "pilgrimage",
            FactionFormed { .. } => "faction_formed",
            FactionObservance { .. } => "faction_observance",
            FactionEnded { .. } => "faction_ended",
            SoughtKin { .. } => "sought_kin",
            KinMet { .. } => "kin_met",
            MeetingSeen { .. } => "meeting_seen",
            LetIn { .. } => "let_in",
            TakenBelow { .. } => "taken_below",
            GateFoundOpen { .. } => "gate_found_open",
            WentDownAfter { .. } => "went_down_after",
            CameUpFromBelow { .. } => "came_up_from_below",
            GrownBelow { .. } => "grown_below",
            ShrineLost { .. } => "shrine_lost",
            KeeperNamed { .. } => "keeper_named",
            KeeperChallenged { .. } => "keeper_challenged",
            WallBroken { .. } => "wall_broken",
            Barred { .. } => "barred",
            SpellBegan { .. } => "spell_began",
            SpellLifted { .. } => "spell_lifted",
            Faring { .. } => "faring",
            LastOne { .. } => "last_one",
            WentAway { .. } => "went_away",
            CameBack { .. } => "came_back",
            RiteOutcome { .. } => "rite_outcome",
            LimbTaken { .. } => "limb_taken",
            LimbMade { .. } => "limb_made",
            MadeUp { .. } => "made_up",
            Repaid { .. } => "repaid",
            RivalMade { .. } => "rival_made",
            RivalReturned { .. } => "rival_returned",
            RivalMet { .. } => "rival_met",
            RivalFell { .. } => "rival_fell",
            Fished { .. } => "fished",
            Tamed { .. } => "tamed",
            SnareSet { .. } => "snare_set",
            Snared { .. } => "snared",
            Drawn { .. } => "drawn",
            StoresRaided { .. } => "stores_raided",
            CameOfAge { .. } => "came_of_age",
            Frosted { .. } => "frosted",
            MineSunk { .. } => "mine_sunk",
            ReachedDeep { .. } => "reached_deep",
            DeepChosen { .. } => "deep_chosen",
            StirBelow { .. } => "stir_below",
            CameBackUnsettled { .. } => "came_back_unsettled",
            WouldNotCut { .. } => "would_not_cut",
            AccountTold { .. } => "account_told",
            GraveRead { .. } => "grave_read",
            LeftUnrested { .. } => "left_unrested",
            RoleLeft { .. } => "role_left",
            Thinned { .. } => "thinned",
            CameOver { .. } => "came_over",
            StoresFailed { .. } => "stores_failed",
            Weakened { .. } => "weakened",
            SickenedWeak { .. } => "sickened_weak",
            RainSpoiled { .. } => "rain_spoiled",
            HealerLost { .. } => "healer_lost",
            WoundFestered { .. } => "wound_festered",
            Divided { .. } => "divided",
            Crept { .. } => "crept",
            Fostered { .. } => "fostered",
            StruckWater { .. } => "struck_water",
            FireStarted { .. } => "fire_started",
            FireOut { .. } => "fire_out",
            Burned { .. } => "burned",
            Doused { .. } => "doused",
            Flooded { .. } => "flooded",
            Receded => "receded",
            BrokeOver { .. } => "broke_over",
        }
    }
}

/// Per-tick event sink. The world owns one; systems push into it; the flush system drains it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventSink {
    pub next_id: u32,
    pub pending: Vec<Event>,
}

impl EventSink {
    pub fn push(
        &mut self,
        tick: u64,
        at: Option<TilePos>,
        severity: Severity,
        kind: EventKind,
    ) -> EventId {
        let id = EventId(self.next_id);
        self.next_id += 1;
        self.pending.push(Event {
            id,
            tick,
            at,
            severity,
            kind,
        });
        id
    }
    pub fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending)
    }
}
