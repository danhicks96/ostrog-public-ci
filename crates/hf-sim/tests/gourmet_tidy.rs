//! OST-1102: the Gourmet and Tidy traits are drawn from the genome, so people can have them.

use hf_sim::genetics::Genome;
use hf_sim::mind::Trait;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[test]
fn gourmet_and_tidy_appear_within_the_trait_cap_and_deterministically() {
    let mut r = ChaCha8Rng::seed_from_u64(1102);
    let (mut gourmet, mut tidy) = (0, 0);
    for _ in 0..500 {
        let g = Genome::random(&mut r);
        let t = g.traits();
        assert!(t.list.len() <= 3, "never over the trait cap: {:?}", t.list);
        assert_eq!(
            t.list,
            g.traits().list,
            "the same genome gives the same traits"
        );
        gourmet += t.has(Trait::Gourmet) as u32;
        tidy += t.has(Trait::Tidy) as u32;
    }
    assert!(gourmet >= 10, "Gourmet drawn on {gourmet} of 500");
    assert!(tidy >= 10, "Tidy drawn on {tidy} of 500");
}
