//! The "Why?" lens (#567): a bounded read model of one selected person's recent decisions.
//!
//! The viewer names one person to trace (`World::trace_why`). While someone is named,
//! `systems::decide` writes each of that person's decision cycles here: the conditions it
//! checked with their values, every option it weighed with its named score terms (the base the
//! goal scored, the emotion lean, the secured gate), why a rejected option was not a choice, the
//! option chosen, and what was put off. Nothing here is read by the simulation, nothing is
//! saved, and no random draw is made for it: the digest is the same with tracing on and off
//! (guarded in `tests/why_lens.rs`). Only the traced person's records are kept, at most
//! `CYCLES` cycles of at most `OPTIONS` options each.

use std::collections::VecDeque;

use crate::ids::AgentId;

/// Decision cycles kept for the traced person, newest last.
pub const CYCLES: usize = 8;
/// Options kept per cycle: the best scoring, with the chosen one always among them.
pub const OPTIONS: usize = 10;
/// Lines of things put off kept per cycle.
pub const PUT_OFF: usize = 6;

/// How the person came to decide.
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// Nothing was under way: an ordinary, deliberate choice.
    Deliberate,
    /// A pressing need cut into what they were doing (`was`), for the reason given (`by`).
    Interrupted { was: String, by: String },
}

/// One condition checked before weighing, with its value and whether it held.
#[derive(Clone, Debug, PartialEq)]
pub struct Condition {
    pub name: &'static str,
    pub value: f32,
    /// `true` when the condition is met (secured, not pressing).
    pub ok: bool,
}

/// One option weighed, with its named score terms. `score` is
/// `base * emotion * gate * learned * spirits * temperament`.
#[derive(Clone, Debug, PartialEq)]
pub struct Considered {
    /// The stable name of the cycle the option belongs to ("go_for_meals", "talk", "work").
    pub cycle: &'static str,
    /// What it would have them do, in words.
    pub what: String,
    /// What the goal scored from need, disposition, memory, skill and distance together.
    pub base: f32,
    /// The lean of what they feel now (docs/design-feelings.md), 0.7 to 1.4.
    pub emotion: f32,
    /// The secured gate (#415, rule 7): leisure weighs little until food and shelter are safe.
    pub gate: f32,
    /// What judging an earlier outcome taught them (#568): regret or rationalising toward this
    /// person, fading; 1 when nothing was learned.
    pub learned: f32,
    /// What their mood does to the weight of this option (#885): the slot for the inner-life
    /// lean that #886 onward fills. 1 for every option until then, and printed only when not 1.
    pub spirits: f32,
    /// What a disposition (industrious, lazy, and the rest) does to the weight of this option
    /// (#906): the slot for the terms later slices move out of `base`. 1 for every option until
    /// then, and printed only when not 1.
    pub temperament: f32,
    pub score: f32,
    /// Why it was not a choice at all, when it was not.
    pub rejected: Option<&'static str>,
    pub chosen: bool,
}

/// One decision cycle of the traced person.
#[derive(Clone, Debug, PartialEq)]
pub struct Cycle {
    pub tick: u64,
    pub mode: Mode,
    pub conditions: Vec<Condition>,
    pub options: Vec<Considered>,
    /// The chosen option's words, or "nothing" when no option scored.
    pub chosen: String,
    /// The reasons for the choice, built from the conditions and the winning terms.
    pub because: Vec<String>,
    /// What they put off: options the secured gate held back that would otherwise have won, and
    /// work they gave up on for now.
    pub put_off: Vec<String>,
}

/// Percepts kept for the traced person, newest last (#569).
pub const PERCEPTS: usize = 8;

/// A stimulus an expert's attention could have caught (#569): what it was, its salience to this
/// person against `mind::attention::NOTICE`, and whether it was noticed. A stimulus present but
/// not noticed is shown here and nowhere else: it never enters their ledger.
#[derive(Clone, Debug, PartialEq)]
pub struct Percept {
    pub tick: u64,
    pub domain: &'static str,
    pub what: String,
    pub salience: f32,
    pub noticed: bool,
    pub habituated: bool,
}

/// The traced person and their recent cycles. Transient: never saved, never read by a system.
#[derive(Clone, Debug, PartialEq)]
pub struct WhyTrace {
    pub agent: AgentId,
    pub cycles: VecDeque<Cycle>,
    pub percepts: VecDeque<Percept>,
}

impl WhyTrace {
    pub fn new(agent: AgentId) -> Self {
        Self {
            agent,
            cycles: VecDeque::new(),
            percepts: VecDeque::new(),
        }
    }

    /// Keep a percept, dropping the oldest past the cap (#569).
    pub fn perceive(&mut self, p: Percept) {
        // The same cue read the same way again refreshes its line instead of filling the list.
        if let Some(last) = self
            .percepts
            .iter_mut()
            .rev()
            .find(|x| x.domain == p.domain)
        {
            if last.what == p.what && last.noticed == p.noticed && last.habituated == p.habituated {
                *last = p;
                return;
            }
        }
        if self.percepts.len() >= PERCEPTS {
            self.percepts.pop_front();
        }
        self.percepts.push_back(p);
    }

    /// Keep a finished cycle, trimming its options and dropping the oldest cycle past the cap.
    pub fn push(&mut self, mut c: Cycle) {
        trim(&mut c.options);
        c.put_off.truncate(PUT_OFF);
        if self.cycles.len() >= CYCLES {
            self.cycles.pop_front();
        }
        self.cycles.push_back(c);
    }

    pub fn latest(&self) -> Option<&Cycle> {
        self.cycles.back()
    }
}

/// The best `OPTIONS` options by score, the chosen one always kept, rejected ones after the
/// eligible ones of the same score; a stable order so the same cycle always reads the same.
fn trim(options: &mut Vec<Considered>) {
    options.sort_by(|a, b| {
        b.chosen
            .cmp(&a.chosen)
            .then(a.rejected.is_some().cmp(&b.rejected.is_some()))
            .then(b.score.total_cmp(&a.score))
            .then(a.what.cmp(&b.what))
    });
    options.truncate(OPTIONS);
}

/// Fill in the chosen option's words, the reasons for the choice and what the secured gate put
/// off, from the cycle's own conditions and terms: nothing outside what was weighed.
pub fn explain(c: &mut Cycle) {
    let chosen = c.options.iter().find(|o| o.chosen).cloned();
    c.chosen = chosen
        .as_ref()
        .map(|o| o.what.clone())
        .unwrap_or_else(|| "nothing (rest a moment)".to_string());
    let mut because = Vec::new();
    if let Mode::Interrupted { was, by } = &c.mode {
        because.push(format!("{by}, so they stopped {was}"));
    }
    for k in c.conditions.iter().filter(|k| !k.ok) {
        because.push(match k.name {
            "hunger" => format!("hungry ({:.0}% empty)", k.value * 100.0),
            "thirst" => format!("thirsty ({:.0}% dry)", k.value * 100.0),
            "tiredness" => "worn out".to_string(),
            "days of food in store" => format!(
                "the store holds {:.1} days of food, under the two that secure tomorrow",
                k.value
            ),
            "night near with no bed or roof" => {
                "night is near and they know no bed or roof".to_string()
            }
            "arrival urge" => format!(
                "fresh from the arrival: food, warmth and shelter weigh x{:.2}",
                k.value
            ),
            "leisure weight" => format!(
                "talk, wandering and rest weigh x{:.2} until safe and settled",
                k.value
            ),
            other => other.to_string(),
        });
    }
    if let Some(o) = &chosen {
        let mut terms = format!("{} won at {:.2}: base {:.2}", o.what, o.score, o.base);
        if (o.emotion - 1.0).abs() > 0.005 {
            terms += &format!(", feelings x{:.2}", o.emotion);
        }
        if o.gate < 1.0 {
            terms += &format!(", held back x{:.2}", o.gate);
        }
        if (o.learned - 1.0).abs() > 0.005 {
            terms += &format!(", learned x{:.2}", o.learned);
        }
        // Mood's lean (#885): said only when it moved the score, so at 1.0 nothing is added.
        if (o.spirits - 1.0).abs() > 0.005 {
            terms += &format!(", spirits x{:.2}", o.spirits);
        }
        // A disposition's lean (#906): said only when it moved the score, so at 1.0 nothing is
        // added.
        if (o.temperament - 1.0).abs() > 0.005 {
            terms += &format!(", temperament x{:.2}", o.temperament);
        }
        because.push(terms);
        if let Some(next) = c
            .options
            .iter()
            .filter(|x| !x.chosen && x.rejected.is_none())
            .max_by(|a, b| a.score.total_cmp(&b.score).then(b.what.cmp(&a.what)))
        {
            because.push(format!("over {} at {:.2}", next.what, next.score));
        }
        // Options the gate held back that would have won without it: put off, not refused.
        let mut put_off: Vec<String> = c
            .options
            .iter()
            .filter(|x| {
                !x.chosen && x.rejected.is_none() && x.gate < 1.0 && x.base * x.emotion > o.score
            })
            .map(|x| {
                if x.gate <= crate::systems::decide::LEISURE_UNSECURED {
                    format!("{}: put off until food and shelter are secure", x.what)
                } else {
                    format!("{}: put off while the arrival is fresh", x.what)
                }
            })
            .collect();
        put_off.sort();
        put_off.dedup();
        c.put_off.extend(put_off);
    }
    c.because = because;
}

impl crate::world_state::World {
    /// Trace this person's decisions for the Why? lens, or stop tracing (`None`). Choosing
    /// another person starts a fresh trace; choosing the same one keeps what was gathered.
    pub fn trace_why(&mut self, agent: Option<AgentId>) {
        match agent {
            None => self.why = None,
            Some(id) if self.why.as_ref().is_some_and(|t| t.agent == id) => {}
            Some(id) => self.why = Some(WhyTrace::new(id)),
        }
    }

    /// The person being traced, if anyone.
    pub fn why_target(&self) -> Option<AgentId> {
        self.why.as_ref().map(|t| t.agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(what: &str, score: f32, chosen: bool, rejected: Option<&'static str>) -> Considered {
        Considered {
            cycle: "test",
            what: what.to_string(),
            base: score,
            emotion: 1.0,
            gate: 1.0,
            learned: 1.0,
            spirits: 1.0,
            temperament: 1.0,
            score,
            rejected,
            chosen,
        }
    }

    fn cycle(tick: u64, options: Vec<Considered>) -> Cycle {
        Cycle {
            tick,
            mode: Mode::Deliberate,
            conditions: Vec::new(),
            options,
            chosen: String::new(),
            because: Vec::new(),
            put_off: Vec::new(),
        }
    }

    #[test]
    fn the_trace_is_bounded_and_keeps_the_chosen_option() {
        let mut t = WhyTrace::new(AgentId(1));
        for tick in 0..(CYCLES as u64 + 5) {
            let mut options: Vec<Considered> = (0..30)
                .map(|n| option(&format!("o{n}"), n as f32, false, None))
                .collect();
            options.push(option("low but chosen", 0.01, true, None));
            t.push(cycle(tick, options));
        }
        assert_eq!(t.cycles.len(), CYCLES);
        assert_eq!(
            t.cycles.front().unwrap().tick,
            5,
            "the oldest cycles go first"
        );
        let last = t.latest().unwrap();
        assert_eq!(last.options.len(), OPTIONS);
        assert!(last.options[0].chosen, "the chosen option is never trimmed");
        assert_eq!(last.options[1].what, "o29", "then the best scores");
    }

    #[test]
    fn eligible_options_come_before_rejected_ones() {
        let mut t = WhyTrace::new(AgentId(1));
        t.push(cycle(
            0,
            vec![
                option("blocked", 5.0, false, Some("no way there")),
                option("open", 1.0, false, None),
            ],
        ));
        let c = t.latest().unwrap();
        assert_eq!(c.options[0].what, "open");
        assert_eq!(c.options[1].rejected, Some("no way there"));
    }

    #[test]
    fn spirits_are_said_only_when_they_are_not_one() {
        let said = |spirits: f32| {
            let mut won = option("rest", 1.0, true, None);
            won.spirits = spirits;
            let mut c = cycle(0, vec![won]);
            explain(&mut c);
            c.because.join("; ")
        };
        assert!(!said(1.0).contains("spirits"), "{}", said(1.0));
        assert!(said(0.7).contains("spirits x0.70"), "{}", said(0.7));
    }

    #[test]
    fn temperament_is_said_only_when_it_is_not_one() {
        let said = |temperament: f32| {
            let mut won = option("work", 1.0, true, None);
            won.temperament = temperament;
            let mut c = cycle(0, vec![won]);
            explain(&mut c);
            c.because.join("; ")
        };
        assert!(!said(1.0).contains("temperament"), "{}", said(1.0));
        assert!(said(1.3).contains("temperament x1.30"), "{}", said(1.3));
    }
}
