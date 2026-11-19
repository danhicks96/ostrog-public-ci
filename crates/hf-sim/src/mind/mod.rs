//! The inhabitant's mind: needs, traits, skills, perception, knowledge, and the activity it
//! is currently committed to. Decisions are made in `systems::decide` from these parts.

pub mod activity;
pub mod attention;
pub mod beliefs;
pub mod conscience;
pub mod dispositions;
pub mod feelings;
pub mod fragment;
pub mod intentions;
pub mod knowledge;
pub mod learning;
pub mod memory;
pub mod needs;
pub mod perception;
pub mod problems;
pub mod skills;
pub mod techniques;
pub mod telling;
pub mod thoughts;
pub mod traits;
pub mod witnessed;

pub use activity::Activity;
pub use beliefs::Beliefs;
pub use dispositions::{Dispositions, SurfaceTrait};
pub use knowledge::{Knowledge, Known};
pub use learning::{
    Evidence, Learning, Lesson, Observation, ObservationKind, OutcomeKind, Reading,
};
pub use memory::{MemKind, Memories, Memory, Record, Relation, Subject};
pub use needs::Needs;
pub use problems::{
    HeardProblem, Problem, ProblemId, ProblemKind, ProblemStatus, Problems, ReactionAttempt,
    ReactionKind, ATTEMPT_CAP, PROBLEM_CAP,
};
pub use skills::{Skill, Skills};
pub use techniques::{Technique, Techniques};
pub use thoughts::{Thought, ThoughtKind, Thoughts};
pub use traits::{Trait, Traits};
