//! The Legends browser (M3, C28): pages about the world that link only to things that exist,
//! and carry the record of each thing.

use hf_sim::chronicle::Chronicle;
use hf_sim::view::legends;
use hf_sim::Session;

fn links(page: &str) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    let mut rest = page;
    while let Some(k) = rest.find("[url=") {
        let after = &rest[k + 5..];
        let end = after.find(']').unwrap();
        let (kind, id) = after[..end].split_once(':').unwrap();
        out.push((kind.to_string(), id.parse().unwrap()));
        rest = &after[end..];
    }
    out
}

#[test]
fn every_page_links_only_to_what_exists_and_tells_its_record() {
    let w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(3));
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    s.write_pending().unwrap();
    let (w, c) = (&s.world, &s.chronicle);
    let r = &w.realm;
    let mut pages = vec!["world:0".to_string()];
    pages.extend((0..r.peoples.len()).map(|i| format!("people:{i}")));
    pages.extend((0..r.polities.len().min(5)).map(|i| format!("polity:{i}")));
    pages.extend((0..r.wars.len().min(5)).map(|i| format!("war:{i}")));
    pages.extend((0..r.sites.len()).step_by(37).map(|i| format!("site:{i}")));
    pages.extend(
        (0..r.figures.len())
            .step_by(211)
            .map(|i| format!("figure:{i}")),
    );
    let mut recorded = 0;
    for p in &pages {
        let text = legends::page(w, c, p);
        assert!(!text.is_empty(), "{p}");
        assert!(
            !text.contains("someone") && !text.contains("a place"),
            "{p} has a placeholder: {text}"
        );
        for (kind, id) in links(&text) {
            let ok = match kind.as_str() {
                "world" => true,
                // What the valley knows (#351): one page.
                "known" => id == 0,
                "people" => (id as usize) < r.peoples.len(),
                "site" => (id as usize) < r.sites.len(),
                "figure" => (id as usize) < r.figures.len(),
                "polity" => (id as usize) < r.polities.len(),
                "war" => (id as usize) < r.wars.len(),
                "culture" => (id as usize) < r.cultures.len(),
                "beast" => (id as usize) < r.beasts.len(),
                "species" => w.kinds().iter().any(|(k, _)| k.0 as u32 == id),
                _ => false,
            };
            assert!(ok, "{p} links to {kind}:{id}");
        }
        if text.contains("[b]The record[/b]") {
            recorded += 1;
        }
    }
    assert!(recorded >= 5, "pages with a record: {recorded}");
    // A settlement's page tells when it was founded, in years before the portal.
    let first = legends::page(w, c, "site:0");
    assert!(first.contains("years before the portal"), "{first}");
}

#[test]
fn the_map_shows_the_world_as_it_was_in_a_year() {
    let w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(3));
    let m = hf_sim::realm::generate(3, &hf_sim::realm::RealmParams::default());
    let early = legends::map_rgb(&w, &m, 10);
    let late = legends::map_rgb(&w, &m, w.realm.year);
    assert_eq!(early.len(), m.size * m.size * 3);
    // More of the world is settled at the end than in its tenth year.
    let marked = |px: &[u8]| {
        w.realm
            .sites
            .iter()
            .filter(|x| {
                let k = x.region * 3;
                let base = px[k..k + 3].to_vec();
                w.realm.peoples.iter().any(|p| p.colour.to_vec() == base)
            })
            .count()
    };
    assert!(
        marked(&late) > marked(&early) * 3,
        "{} then, {} now",
        marked(&early),
        marked(&late)
    );
}
