//! Minimal reference implementation of the repair/admissibility core
//! structures from Diagnostic Coordinates and the Time of Repair:
//! `Γ_h`, `B(h)`, `R_Γ(h) = Ext_{Γ_h}(B(h))`, the objective
//! `J(a|h) = E + λR + μΔ_dep − νM`, the Repair-Warrant Criterion, and the
//! three-way refusal/escalation taxonomy that generalizes entheai's
//! `repair_stop.rs`.
//!
//! Everything here is deliberately thin. The theory-specific content —
//! what `E`, `R`, `M` mean, what a feature vector actually represents,
//! what `Γ_h`'s generator does — is left as pluggable pieces you supply.
//! What's fixed is the *shape*: the three-way split at `Ext_Γ`, the
//! objective's algebraic form, and the Repair-Warrant Criterion.

pub mod domain;
pub mod ledger;
pub mod objective;
pub mod reference_class;
pub mod taxonomy;
