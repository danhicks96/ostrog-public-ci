//! Work: what the community wants done (designations), how one person commits to doing a
//! piece of it (plans of steps), and the leases that keep two people from grabbing the same
//! log (reservations). Executors for each job live in `systems::act`.

pub mod designation;
pub mod plan;
pub mod planner;
pub mod recipes;
pub mod reservation;

pub use designation::{split_off, Designation, DesignationId, DesignationKind, Requirement};
pub use plan::{Plan, Step};
pub use reservation::{Lease, ResKey, ReservationTable};
