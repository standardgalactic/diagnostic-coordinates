//! `Γ_h`, `Ext_Γ(B(h))`, and the reference class `R_Γ(h)`.
//!
//! `Γ_h` is the governing constraint structure. Its defining property, per
//! Diagnostic Coordinates, is that its provenance is independent of `h` —
//! it must not be inferable solely from the damaged trajectory it's being
//! used to repair. That independence isn't something the type system can
//! enforce; it's a property of how you construct `ConstraintStructure`
//! values, not of the struct itself. Keep it that way when you wire in a
//! real Γ.

use crate::domain::{BoundaryData, Continuation};

/// A governing constraint structure `Γ_h`, represented here as a generator
/// function from boundary data to admissible continuations. This is the
/// seam you replace with your actual constraint semantics (a grammar, a
/// type system, a physical law set, whatever the theory under test uses).
pub struct ConstraintStructure {
    pub id: String,
    generator: Box<dyn Fn(&BoundaryData) -> Vec<Continuation>>,
}

impl ConstraintStructure {
    pub fn new(
        id: impl Into<String>,
        generator: impl Fn(&BoundaryData) -> Vec<Continuation> + 'static,
    ) -> Self {
        ConstraintStructure { id: id.into(), generator: Box::new(generator) }
    }
}

/// Outcome of `Ext_Γ(B(h))`. This is the three-way split the taxonomy in
/// Diagnostic Coordinates hinges on:
/// - `NonEmpty`: vertical repair is well-posed against this reference class.
/// - `EmptyDetermined`: every boundary fact was fully classified
///   (Irreversible or Revisable) and Γ_h still yields nothing — the
///   emptiness is a real finding, not an artifact of missing diagnosis.
/// - `Undetermined`: at least one boundary fact is `Unclassified`, so an
///   empty (or any) result can't yet be trusted — repair doesn't know
///   whether the obstruction is in `B(h)` or in `Γ_h`.
#[derive(Debug, Clone, PartialEq)]
pub enum ExtOutcome {
    NonEmpty(Vec<Continuation>),
    EmptyDetermined,
    Undetermined,
}

/// Compute `Ext_Γ(B(h))`, i.e. attempt to derive the reference class
/// `R_Γ(h)` from the constraint structure and the repair-time boundary data.
pub fn ext(gamma: &ConstraintStructure, boundary: &BoundaryData) -> ExtOutcome {
    if boundary.has_unclassified() {
        return ExtOutcome::Undetermined;
    }
    let continuations = (gamma.generator)(boundary);
    if continuations.is_empty() {
        ExtOutcome::EmptyDetermined
    } else {
        ExtOutcome::NonEmpty(continuations)
    }
}
