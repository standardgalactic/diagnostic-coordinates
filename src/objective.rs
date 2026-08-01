//! The repair objective `J(a|h) = E(a|h) + λR(a|h) + μΔ_dep(a|h,R(h)) − νM(a|h)`
//! and the Repair-Warrant Criterion built on top of it.
//!
//! `E`, `R`, and `M` are left as pluggable scoring functions — this crate
//! doesn't take a position on what "evidence," "regularization cost," or
//! "marginal value" mean for your theory. `Δ_dep` is the one piece given a
//! concrete default implementation, since its shape (soft-min aggregated
//! structural distance to a reference class) is specified in Diagnostic
//! Coordinates itself.

use crate::domain::{Continuation, History, Intervention};

/// The three scalar terms of `J` you supply per use case. `e`, `r`, and `m`
/// each take the *post-intervention* history so they can score the
/// intervention's direct effect; `Δ_dep` is computed separately by
/// [`delta_dep`] because its definition is fixed by the theory.
pub trait ObjectiveTerms {
    fn e(&self, h_prime: &History, a: &Intervention) -> f64;
    fn r(&self, h_prime: &History, a: &Intervention) -> f64;
    fn m(&self, h_prime: &History, a: &Intervention) -> f64;
}

/// Weights `(λ, μ, ν)` on the regularization, structural-effect, and
/// marginal-value terms respectively.
#[derive(Debug, Clone, Copy)]
pub struct Weights {
    pub lambda: f64,
    pub mu: f64,
    pub nu: f64,
}

/// Euclidean distance between two feature vectors, padding the shorter
/// with zeros. Swap for whatever structure-sensitive metric your theory
/// actually wants; this is the placeholder the soft-min is built on.
fn feature_distance(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| {
            let x = a.get(i).copied().unwrap_or(0.0);
            let y = b.get(i).copied().unwrap_or(0.0);
            (x - y).powi(2)
        })
        .sum::<f64>()
        .sqrt()
}

/// `δ_dep(h, R(h))`: soft-min aggregated structural distance from `h` to
/// the reference class. Uses a log-sum-exp soft-min with sharpness
/// `tau` (smaller `tau` → closer to a hard min). Returns `f64::INFINITY`
/// if the reference class is empty — callers should have already routed
/// an empty reference class to taxonomy branch (2) or (3) before reaching
/// here; this is a defined-but-degenerate fallback, not silent handling.
pub fn delta_dep_distance(h: &History, reference: &[Continuation], tau: f64) -> f64 {
    if reference.is_empty() {
        return f64::INFINITY;
    }
    let distances: Vec<f64> =
        reference.iter().map(|c| feature_distance(&h.features, &c.features)).collect();
    let min_d = distances.iter().cloned().fold(f64::INFINITY, f64::min);
    let sum_exp: f64 = distances.iter().map(|d| (-(d - min_d) / tau).exp()).sum();
    min_d - tau * (sum_exp).ln()
}

/// `Δ_dep(a|h, R(h))`: the marginal structural effect of applying `a`,
/// i.e. the change in `δ_dep` between the post- and pre-intervention
/// history against the same reference class.
pub fn delta_dep(
    h: &History,
    a: &Intervention,
    reference: &[Continuation],
    tau: f64,
) -> f64 {
    let h_prime = h.apply(a);
    delta_dep_distance(&h_prime, reference, tau) - delta_dep_distance(h, reference, tau)
}

/// Compute `J(a|h)` in full.
///
/// Per the Repair-Warrant Criterion, callers should verify separately that
/// `j(h, &Intervention::null(), ..)` evaluates to exactly `0.0` for their
/// chosen `ObjectiveTerms` impl — this function doesn't special-case the
/// null intervention, it trusts the terms to be well-formed. See
/// [`crate::ledger::RepairLedger::check_null_intervention`] for a runtime
/// assertion of that property.
pub fn j(
    h: &History,
    a: &Intervention,
    reference: &[Continuation],
    terms: &impl ObjectiveTerms,
    weights: Weights,
    tau: f64,
) -> f64 {
    let h_prime = h.apply(a);
    let e = terms.e(&h_prime, a);
    let r = terms.r(&h_prime, a);
    let m = terms.m(&h_prime, a);
    let dep = delta_dep(h, a, reference, tau);
    e + weights.lambda * r + weights.mu * dep - weights.nu * m
}
