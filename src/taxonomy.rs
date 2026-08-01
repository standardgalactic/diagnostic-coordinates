//! The three-way refusal/escalation taxonomy that replaces the single
//! overloaded failure condition in Diagnostic Coordinates §7.

use crate::domain::Intervention;

#[derive(Debug, Clone, PartialEq)]
pub enum RefusalBranch {
    /// Branch (1): an admissible reference class exists, but `J(a) ≥ 0`
    /// for every non-null candidate — no intervention beats refusal.
    /// This is pure vertical repair territory; nothing escalates.
    NoInterventionBeatsRefusal { best_candidate: Option<Intervention>, best_j: f64 },

    /// Branch (2): `R_Γ(h) = ∅` and every boundary fact was classified —
    /// the present repair frame supplies no admissible comparison class
    /// at all. This is a genuine finding about Γ_h and B(h) together, not
    /// a diagnostic gap. Repair has become horizontal: the theory (Γ_h),
    /// its vocabulary, or an "irreversible" classification in B(h) must
    /// give.
    EmptyReferenceClass,

    /// Branch (3): at least one boundary fact is unclassified, so it
    /// can't yet be determined whether an obstruction belongs to `B(h)`
    /// or to `Γ_h`. This is a diagnostic-coordinate deficiency in its own
    /// right, distinct from either component actually being at fault.
    /// Escalation here means classifying the fact, not revising Γ_h.
    CoordinateDeficiency { unclassified_fact_ids: Vec<String> },
}

impl RefusalBranch {
    pub fn label(&self) -> &'static str {
        match self {
            RefusalBranch::NoInterventionBeatsRefusal { .. } => {
                "(1) no intervention beats refusal"
            }
            RefusalBranch::EmptyReferenceClass => "(2) empty reference class",
            RefusalBranch::CoordinateDeficiency { .. } => "(3) coordinate deficiency",
        }
    }
}
