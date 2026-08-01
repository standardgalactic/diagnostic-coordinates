//! Runs four toy scenarios through `RepairLedger::diagnose_and_repair`,
//! one per outcome the taxonomy distinguishes: a warranted repair, branch
//! (1) refusal, branch (2) refusal, and branch (3) refusal.
//!
//! `cargo run --example toy_scenario`

use repair_admissibility::domain::{
    BoundaryData, Classification, Continuation, Fact, History, Intervention,
};
use repair_admissibility::ledger::{RepairLedger, RepairOutcome};
use repair_admissibility::objective::{ObjectiveTerms, Weights};
use repair_admissibility::reference_class::ConstraintStructure;

/// Toy scoring: cost (`E`) and "marginal value" (`M`) both scale with the
/// size of the intervention's effect, so both vanish for the null
/// intervention (required by the Repair-Warrant Criterion). `R` is left
/// at zero — no regularization term in this toy.
struct ToyTerms;

fn effect_norm(a: &Intervention) -> f64 {
    a.effect.iter().map(|x| x * x).sum::<f64>().sqrt()
}

impl ObjectiveTerms for ToyTerms {
    fn e(&self, _h_prime: &History, a: &Intervention) -> f64 {
        0.1 * effect_norm(a)
    }
    fn r(&self, _h_prime: &History, _a: &Intervention) -> f64 {
        0.0
    }
    fn m(&self, _h_prime: &History, a: &Intervention) -> f64 {
        0.5 * effect_norm(a)
    }
}

fn weights() -> Weights {
    Weights { lambda: 1.0, mu: 1.0, nu: 1.0 }
}

fn print_outcome(label: &str, outcome: &RepairOutcome) {
    println!("--- {label} ---");
    match outcome {
        RepairOutcome::Warranted { intervention, j_value } => {
            println!("  warranted: {} (J = {:.4})", intervention.id, j_value);
        }
        RepairOutcome::Refused(reason) => {
            println!("  refused: {reason:?}");
        }
    }
    println!();
}

fn main() {
    let h = History::new("h0", vec![0.0, 0.0]);
    let weights = weights();
    let terms = ToyTerms;

    // Scenario A: warranted repair. Reference class is close by, and the
    // intervention that reaches it costs less than the distance it closes.
    {
        let gamma = ConstraintStructure::new("gamma-A", |_boundary: &BoundaryData| {
            vec![Continuation { id: "target".into(), features: vec![1.0, 1.0] }]
        });
        let boundary = BoundaryData::new(vec![Fact {
            id: "fact-1".into(),
            classification: Classification::Revisable,
        }]);
        let candidates = vec![
            Intervention { id: "nudge-small".into(), effect: vec![0.2, 0.2] },
            Intervention { id: "nudge-full".into(), effect: vec![1.0, 1.0] },
        ];
        let ledger = RepairLedger::new(&gamma, &terms, weights, 0.25);
        let reference = vec![Continuation { id: "target".into(), features: vec![1.0, 1.0] }];
        assert!(ledger.check_null_intervention(&h, &reference));
        let outcome = ledger.diagnose_and_repair(&h, &boundary, &candidates);
        print_outcome("A: warranted repair", &outcome);
    }

    // Scenario B: branch (1). Reference class exists but every candidate
    // costs more than the structural ground it recovers.
    {
        let gamma = ConstraintStructure::new("gamma-B", |_boundary: &BoundaryData| {
            vec![Continuation { id: "far-target".into(), features: vec![10.0, 10.0] }]
        });
        let boundary = BoundaryData::new(vec![Fact {
            id: "fact-1".into(),
            classification: Classification::Revisable,
        }]);
        // Moves away from the far target: Δ_dep is positive (distance
        // grows) so no choice of positive weights lets this candidate's
        // marginal-value term rescue it — cost is paid for no structural
        // benefit, guaranteeing J ≥ 0.
        let candidates =
            vec![Intervention { id: "wrong-direction".into(), effect: vec![-0.1, -0.1] }];
        let ledger = RepairLedger::new(&gamma, &terms, weights, 0.25);
        let outcome = ledger.diagnose_and_repair(&h, &boundary, &candidates);
        print_outcome("B: branch (1) — no intervention beats refusal", &outcome);
    }

    // Scenario C: branch (2). Boundary is fully classified but the
    // constraint structure genuinely yields no admissible continuation.
    {
        let gamma =
            ConstraintStructure::new("gamma-C", |_boundary: &BoundaryData| Vec::new());
        let boundary = BoundaryData::new(vec![Fact {
            id: "fact-1".into(),
            classification: Classification::Irreversible,
        }]);
        let candidates = vec![Intervention { id: "any-nudge".into(), effect: vec![0.5, 0.5] }];
        let ledger = RepairLedger::new(&gamma, &terms, weights, 0.25);
        let outcome = ledger.diagnose_and_repair(&h, &boundary, &candidates);
        print_outcome("C: branch (2) — empty reference class", &outcome);
    }

    // Scenario D: branch (3). One boundary fact is unclassified, so
    // repair can't yet tell whether the obstruction is in B(h) or Γ_h.
    {
        let gamma =
            ConstraintStructure::new("gamma-D", |_boundary: &BoundaryData| Vec::new());
        let boundary = BoundaryData::new(vec![
            Fact { id: "fact-1".into(), classification: Classification::Revisable },
            Fact { id: "fact-2".into(), classification: Classification::Unclassified },
        ]);
        let candidates = vec![Intervention { id: "any-nudge".into(), effect: vec![0.5, 0.5] }];
        let ledger = RepairLedger::new(&gamma, &terms, weights, 0.25);
        let outcome = ledger.diagnose_and_repair(&h, &boundary, &candidates);
        print_outcome("D: branch (3) — coordinate deficiency", &outcome);
    }
}
