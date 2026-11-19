//! Customs: beliefs a community acts on. Every custom has an origin event (a real, recorded
//! thing that happened), a narrative the community tells about it, an intensity that fades
//! unless renewed, and a behavioural hook that changes what members do.

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, CommunityId, EventId};
use crate::items::ItemKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CustomId(pub u32);

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CustomKind {
    /// Each season on this day the community rests, remembering someone who died.
    RestDay {
        day_of_season: u64,
        remembered: AgentId,
    },
    /// Members avoid eating this kind of food.
    FoodTaboo(ItemKind),
    /// After hunger took someone, the community plants and cooks more than it needs.
    StoreFood,
    /// The welcome norm changed because of what happened.
    Welcome(crate::society::WelcomeNorm),
    /// At dusk they gather at the hearth and someone tells what they remember.
    HearthNight,
    /// Their way of building (C58): the wood they build of and the roof they lay, as their
    /// first house was built.
    Building {
        wood: Option<crate::world::Material>,
        roof: crate::world::Material,
    },
    /// The community favours building upward when a household needs another room (#483).
    /// The custom's origin and narrative retain why that preference was learned.
    UpperStorey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Custom {
    pub id: CustomId,
    pub community: CommunityId,
    pub kind: CustomKind,
    /// The recorded event this custom grew from.
    pub origin: EventId,
    pub origin_tick: u64,
    pub narrative: String,
    /// 0..1; renewed by observance, fading otherwise; lapses below 0.1.
    pub intensity: f32,
    pub adopted_by: Vec<AgentId>,
    pub formed: u64,
    pub lapsed: bool,
    /// How many times it was taught onward (versions in the taught history).
    pub version: u32,
}

impl Custom {
    pub fn label(&self) -> String {
        match &self.kind {
            CustomKind::RestDay { day_of_season, .. } => {
                format!("rest on day {} of each season", day_of_season)
            }
            // Read after "a people who" and "now hold that we".
            CustomKind::FoodTaboo(k) => format!("eat no {}", k.label()),
            CustomKind::StoreFood => "store food against hunger".into(),
            CustomKind::HearthNight => "gather at the hearth at dusk".into(),
            CustomKind::Building { wood, roof } => {
                let roof = if *roof == crate::world::Material::THATCH {
                    "thatch"
                } else {
                    "shingles"
                };
                match wood {
                    Some(m) => format!("build of {} and roof with {roof}", wood_name(*m)),
                    None => format!("roof with {roof}"),
                }
            }
            CustomKind::UpperStorey => "build upward when another room is needed".into(),
            CustomKind::Welcome(n) => (match n {
                crate::society::WelcomeNorm::Open => "take in every stranger",
                crate::society::WelcomeNorm::Closed => "take in no strangers",
                crate::society::WelcomeNorm::WhenFed => "take in strangers when fed",
            })
            .to_string(),
        }
    }
}

fn wood_name(m: crate::world::Material) -> &'static str {
    use crate::world::Material as M;
    match m {
        M::OAK => "oak",
        M::BIRCH => "birch",
        M::PINE => "pine",
        M::FUNGUS_WOOD => "fungus wood",
        _ => "timber",
    }
}
