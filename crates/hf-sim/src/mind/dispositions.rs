//! Persistent dimensional dispositions and their context-sensitive surface expression (#488).
//! Acute state changes the expression and relevant choice weights, never the saved core values
//! (FIXLOG 2026-10-31 #488).

use serde::{Deserialize, Serialize};

use super::Needs;

pub const MAX_SURFACE_TRAITS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dispositions {
    pub sociability: f32,
    pub industry: f32,
    pub curiosity: f32,
    pub caution: f32,
    pub warmth: f32,
    pub cheerfulness: f32,
    pub order: f32,
    pub independence: f32,
}

impl Default for Dispositions {
    fn default() -> Self {
        Self {
            sociability: 0.5,
            industry: 0.5,
            curiosity: 0.5,
            caution: 0.5,
            warmth: 0.5,
            cheerfulness: 0.5,
            order: 0.5,
            independence: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceTrait {
    pub label: &'static str,
    pub salience: f32,
}

impl Dispositions {
    fn expressed(self, needs: &Needs, mood: f32, stress: f32, pain: f32) -> Self {
        let fatigue = 1.0 - needs.rest.clamp(0.0, 1.0);
        let stress = stress.clamp(0.0, 1.0);
        let pain = pain.clamp(0.0, 1.0);
        let mood = mood.clamp(-1.0, 1.0);
        Self {
            sociability: self.sociability - 0.20 * stress - 0.15 * pain - 0.10 * fatigue,
            industry: self.industry - 0.25 * fatigue - 0.15 * pain,
            curiosity: self.curiosity - 0.25 * stress,
            caution: self.caution + 0.25 * stress + 0.15 * pain,
            warmth: self.warmth - 0.30 * pain - 0.20 * stress - 0.10 * fatigue,
            cheerfulness: self.cheerfulness + 0.50 * mood - 0.25 * pain - 0.20 * stress,
            order: self.order,
            independence: self.independence,
        }
    }

    pub fn surface_traits(
        self,
        needs: &Needs,
        mood: f32,
        stress: f32,
        pain: f32,
    ) -> Vec<SurfaceTrait> {
        let current = self.expressed(needs, mood, stress, pain);
        let dimensions = [
            ("sociable", "solitary", current.sociability),
            ("industrious", "easygoing", current.industry),
            ("curious", "settled", current.curiosity),
            ("cautious", "bold", current.caution),
            ("warm", "reserved", current.warmth),
            ("cheerful", "downcast", current.cheerfulness),
            ("orderly", "flexible", current.order),
            ("self-reliant", "communal", current.independence),
        ];
        let mut surface = Vec::new();
        for (high, low, value) in dimensions {
            let value = value.clamp(0.0, 1.0);
            let salience = (value - 0.5).abs() * 2.0;
            if salience >= 0.35 {
                surface.push(SurfaceTrait {
                    label: if value > 0.5 { high } else { low },
                    salience,
                });
            }
        }
        let fatigue = 1.0 - needs.rest.clamp(0.0, 1.0);
        let irritability =
            0.65 * pain.clamp(0.0, 1.0) + 0.45 * stress.clamp(0.0, 1.0) + 0.25 * fatigue;
        if irritability >= 0.45 {
            surface.push(SurfaceTrait {
                label: "irritable",
                salience: irritability.min(1.0),
            });
        }
        if fatigue >= 0.65 {
            surface.push(SurfaceTrait {
                label: "exhausted",
                salience: fatigue,
            });
        }
        surface.sort_by(|a, b| {
            b.salience
                .total_cmp(&a.salience)
                .then_with(|| a.label.cmp(b.label))
        });
        surface.truncate(MAX_SURFACE_TRAITS);
        surface
    }
}

impl crate::agents::Agent {
    fn current_dispositions(&self) -> Option<Dispositions> {
        let pain = self.body.worst_wound().max(self.body.ill());
        self.dispositions.map(|d| {
            d.expressed(&self.needs, self.mood, self.stress, pain)
                .into_bounded()
        })
    }

    pub fn surface_traits(&self) -> Vec<SurfaceTrait> {
        let pain = self.body.worst_wound().max(self.body.ill());
        match self.dispositions {
            Some(d) => d.surface_traits(&self.needs, self.mood, self.stress, pain),
            None => self
                .traits
                .list
                .iter()
                .take(MAX_SURFACE_TRAITS)
                .map(|t| SurfaceTrait {
                    label: t.label(),
                    salience: 1.0,
                })
                .collect(),
        }
    }

    pub fn social_rate(&self) -> f32 {
        match self.dispositions {
            Some(d) => 0.55 + 0.9 * d.sociability.clamp(0.0, 1.0),
            None => self.traits.social_rate(),
        }
    }

    pub fn talk_weight(&self) -> f32 {
        match self.current_dispositions() {
            Some(d) => (0.55 + 0.9 * d.sociability) * (0.9 + 0.2 * d.warmth),
            None => self.traits.talk_weight(),
        }
    }

    /// Appended (#1760): a cheerful person is the one who stops for a word; 1.0 for a legacy
    /// person (no core profile).
    pub fn cheer_weight(&self) -> f32 {
        match self.current_dispositions() {
            Some(d) => 0.85 + 0.3 * d.cheerfulness,
            None => 1.0,
        }
    }

    pub fn work_weight(&self) -> f32 {
        // Fix-site (FIXLOG 2026-11-16 #908): 0.7-1.3 told a worker from an idler too little
        // (working-share ratio 1.4 across the industry quartiles, #1518); 0.4-1.9 tells them
        // apart the way Dwarf Fortress and RimWorld both do.
        match self.current_dispositions() {
            Some(d) => 0.4 + 1.5 * d.industry,
            None => self.traits.work_weight(),
        }
    }

    pub fn wander_weight(&self) -> f32 {
        // Fix-site (FIXLOG 2026-11-01 #488 wander): the floor is 0.6, not 0.5. Legacy
        // weights gave everyone at least 1.0; a floor of 0.5 pushed most of a valley's
        // wander score under Loiter's 0.2, so a calm valley hardly wandered and the
        // fear lean (x0.7) had nothing left to show against shifted sleep and idle time.
        match self.current_dispositions() {
            Some(d) => (0.6 + 0.8 * d.curiosity) * (1.1 - 0.2 * d.caution),
            None => self.traits.wander_weight(),
        }
    }

    /// Fix-site (FIXLOG 2026-11-16 #907): the hardest worker loiters about as much as
    /// anyone, because Loiter's floor was the same for everyone. 0.5 at full industry,
    /// 1.5 at none.
    pub fn loiter_weight(&self) -> f32 {
        match self.current_dispositions() {
            Some(d) => 0.5 + 1.0 * (1.0 - d.industry.clamp(0.0, 1.0)),
            None => self.traits.loiter_weight(),
        }
    }

    pub fn haul_fit(&self) -> f32 {
        match self.dispositions {
            Some(d) => 0.3 * d.order.clamp(0.0, 1.0),
            None if self.traits.has(crate::mind::Trait::Tidy) => 0.3,
            None => 0.0,
        }
    }

    /// Appended (#1761): how tidy they are weighs a haul or a cook order; legacy people
    /// (no core profile) are unaffected.
    pub fn order_weight(&self) -> f32 {
        match self.dispositions {
            Some(d) => 0.8 + 0.4 * d.order.clamp(0.0, 1.0),
            None => 1.0,
        }
    }

    /// Appended (#1759): how self-reliant they are, read where a choice needs it. 0.5 (the
    /// midpoint of a fresh core profile) for a legacy person, so a formula built around that
    /// midpoint reads as neutral for them too.
    pub fn independence(&self) -> f32 {
        match self.current_dispositions() {
            Some(d) => d.independence,
            None => 0.5,
        }
    }

    /// Appended (#1758): when the stores run short, the hard workers are the first out with a
    /// basket; 1.0 for a legacy person.
    pub fn stock_weight(&self) -> f32 {
        match self.current_dispositions() {
            Some(d) => 0.6 + 0.8 * d.industry,
            None => 1.0,
        }
    }
}

impl Dispositions {
    fn into_bounded(mut self) -> Self {
        self.sociability = self.sociability.clamp(0.0, 1.0);
        self.industry = self.industry.clamp(0.0, 1.0);
        self.curiosity = self.curiosity.clamp(0.0, 1.0);
        self.caution = self.caution.clamp(0.0, 1.0);
        self.warmth = self.warmth.clamp(0.0, 1.0);
        self.cheerfulness = self.cheerfulness.clamp(0.0, 1.0);
        self.order = self.order.clamp(0.0, 1.0);
        self.independence = self.independence.clamp(0.0, 1.0);
        self
    }
}
