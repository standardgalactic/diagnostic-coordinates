//! `RepairLedger`: wires `Ext_Γ`, `J`, and the refusal taxonomy into one
//! decision procedure.
//!
//! This generalizes entheai's `repair_stop.rs`. That implementation's
//! `StopReason` (`MaxAttempts`, `NoMarginalValue`, `WeakVerifier`) all live
//! inside taxonomy branch (1) — they're different ways of saying "no
//! intervention beat refusal," under a single scalar margin rather than
//! the full `J`. `StopReason` below keeps those three variants (so entheai
//! call sites can map onto it without a rewrite) and adds `Branch2` /
//! `Branch3` as first-class outcomes entheai currently has no
//! representation for at all.

use crate::domain::{BoundaryData, History, Intervention};
use crate::objective::{j, ObjectiveTerms, Weights};
use crate::reference_class::{ext, ConstraintStructure, ExtOutcome};
use crate::taxonomy::RefusalBranch;

#[derive(Debug, Clone, PartialEq)]
pub enum StopReason {
    /// entheai-compatible: attempt budget exhausted. Only meaningful once
    /// you've already established branch (1) applies.
    MaxAttempts,
    /// entheai-compatible: best candidate's marginal value didn't clear
    /// the bar. Direct analogue of branch (1) under the full `J`.
    NoMarginalValue,
    /// entheai-compatible: a verifier rejected the best candidate.
    WeakVerifier,
    /// New: repair escalated to branch (2) or (3) rather than terminating
    /// within vertical repair at all.
    Escalated(RefusalBranch),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepairOutcome {
    /// Some non-null intervention has `J(a|h) < 0`: repair is warranted.
    Warranted { intervention: Intervention, j_value: f64 },
    /// No warranted intervention; see `StopReason` for why.
    Refused(StopReason),
}

pub struct RepairLedger<'a, T: ObjectiveTerms> {
    pub gamma: &'a ConstraintStructure,
    pub terms: &'a T,
    pub weights: Weights,
    pub tau: f64,
}

impl<'a, T: ObjectiveTerms> RepairLedger<'a, T> {
    pub fn new(gamma: &'a ConstraintStructure, terms: &'a T, weights: Weights, tau: f64) -> Self {
        RepairLedger { gamma, terms, weights, tau }
    }

    /// Runtime check of the Repair-Warrant Criterion's first clause:
    /// `J(∅|h) = 0` exactly, for any `h`. Call this once per `ObjectiveTerms`
    /// impl (e.g. in a test) rather than per repair attempt.
    pub fn check_null_intervention(&self, h: &History, reference: &[crate::domain::Continuation]) -> bool {
        let null = Intervention::null();
        j(h, &null, reference, self.terms, self.weights, self.tau) == 0.0
    }

    /// Run the full diagnose-and-repair procedure: derive `R_Γ(h)` from
    /// `Γ_h` and `B(h)`, route to taxonomy branch (2)/(3) if it doesn't
    /// yield a usable reference class, and otherwise evaluate all
    /// candidate interventions against `J` and return the best warranted
    /// one, or a branch-(1) refusal.
    pub fn diagnose_and_repair(
        &self,
        h: &History,
        boundary: &BoundaryData,
        candidates: &[Intervention],
    ) -> RepairOutcome {
        match ext(self.gamma, boundary) {
            ExtOutcome::Undetermined => {
                let unclassified_fact_ids = boundary
                    .facts
                    .iter()
                    .filter(|f| f.classification == crate::domain::Classification::Unclassified)
                    .map(|f| f.id.clone())
                    .collect();
                RepairOutcome::Refused(StopReason::Escalated(
                    RefusalBranch::CoordinateDeficiency { unclassified_fact_ids },
                ))
            }
            ExtOutcome::EmptyDetermined => RepairOutcome::Refused(StopReason::Escalated(
                RefusalBranch::EmptyReferenceClass,
            )),
            ExtOutcome::NonEmpty(reference) => {
                let mut best: Option<(Intervention, f64)> = None;
                for a in candidates {
                    if a.is_null() {
                        continue;
                    }
                    let value = j(h, a, &reference, self.terms, self.weights, self.tau);
                    if best.as_ref().map_or(true, |(_, best_v)| value < *best_v) {
                        best = Some((a.clone(), value));
                    }
                }
                match best {
                    Some((intervention, value)) if value < 0.0 => {
                        RepairOutcome::Warranted { intervention, j_value: value }
                    }
                    Some((intervention, value)) => {
                        RepairOutcome::Refused(StopReason::Escalated(
                            RefusalBranch::NoInterventionBeatsRefusal {
                                best_candidate: Some(intervention),
                                best_j: value,
                            },
                        ))
                    }
                    None => RepairOutcome::Refused(StopReason::Escalated(
                        RefusalBranch::NoInterventionBeatsRefusal {
                            best_candidate: None,
                            best_j: 0.0,
                        },
                    )),
                }
            }
        }
    }
}
