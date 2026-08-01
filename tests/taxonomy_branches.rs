use repair_admissibility::domain::{
    BoundaryData, Classification, Continuation, Fact, History, Intervention,
};
use repair_admissibility::ledger::{RepairLedger, RepairOutcome, StopReason};
use repair_admissibility::objective::{ObjectiveTerms, Weights};
use repair_admissibility::reference_class::ConstraintStructure;
use repair_admissibility::taxonomy::RefusalBranch;

struct ZeroTerms;
impl ObjectiveTerms for ZeroTerms {
    fn e(&self, _h_prime: &History, _a: &Intervention) -> f64 {
        0.0
    }
    fn r(&self, _h_prime: &History, _a: &Intervention) -> f64 {
        0.0
    }
    fn m(&self, _h_prime: &History, _a: &Intervention) -> f64 {
        0.0
    }
}

fn weights() -> Weights {
    Weights { lambda: 1.0, mu: 1.0, nu: 1.0 }
}

#[test]
fn null_intervention_has_zero_j() {
    // Δ_dep(∅|h,R(h)) is δ_dep(h,R(h)) − δ_dep(h,R(h)) = 0 for *any*
    // non-empty reference class — checked against a concrete one here,
    // since an empty reference class makes δ_dep degenerate to ∞ − ∞
    // (NaN), which taxonomy branch (2)/(3) should have intercepted
    // before J is ever evaluated.
    let gamma = ConstraintStructure::new("gamma", |_b: &BoundaryData| Vec::new());
    let terms = ZeroTerms;
    let ledger = RepairLedger::new(&gamma, &terms, weights(), 0.25);
    let h = History::new("h0", vec![0.0, 0.0]);
    let reference = vec![Continuation { id: "target".into(), features: vec![1.0, 1.0] }];
    assert!(ledger.check_null_intervention(&h, &reference));
}

#[test]
fn warranted_repair_when_intervention_closes_ground_cheaply() {
    let gamma = ConstraintStructure::new("gamma", |_b: &BoundaryData| {
        vec![Continuation { id: "target".into(), features: vec![1.0, 1.0] }]
    });
    let terms = ZeroTerms; // isolate the effect of Δ_dep alone
    let boundary = BoundaryData::new(vec![Fact {
        id: "f1".into(),
        classification: Classification::Revisable,
    }]);
    let candidates = vec![Intervention { id: "nudge".into(), effect: vec![1.0, 1.0] }];
    let ledger = RepairLedger::new(&gamma, &terms, weights(), 0.25);
    let h = History::new("h0", vec![0.0, 0.0]);
    let outcome = ledger.diagnose_and_repair(&h, &boundary, &candidates);
    match outcome {
        RepairOutcome::Warranted { j_value, .. } => assert!(j_value < 0.0),
        other => panic!("expected Warranted, got {other:?}"),
    }
}

#[test]
fn branch_one_when_no_candidate_beats_refusal() {
    let gamma = ConstraintStructure::new("gamma", |_b: &BoundaryData| {
        vec![Continuation { id: "target".into(), features: vec![1.0, 1.0] }]
    });
    // Costly terms make every non-null intervention worse than refusal.
    struct ExpensiveTerms;
    impl ObjectiveTerms for ExpensiveTerms {
        fn e(&self, _h_prime: &History, _a: &Intervention) -> f64 {
            1000.0
        }
        fn r(&self, _h_prime: &History, _a: &Intervention) -> f64 {
            0.0
        }
        fn m(&self, _h_prime: &History, _a: &Intervention) -> f64 {
            0.0
        }
    }
    let terms = ExpensiveTerms;
    let boundary = BoundaryData::new(vec![Fact {
        id: "f1".into(),
        classification: Classification::Revisable,
    }]);
    let candidates = vec![Intervention { id: "nudge".into(), effect: vec![1.0, 1.0] }];
    let ledger = RepairLedger::new(&gamma, &terms, weights(), 0.25);
    let h = History::new("h0", vec![0.0, 0.0]);
    let outcome = ledger.diagnose_and_repair(&h, &boundary, &candidates);
    match outcome {
        RepairOutcome::Refused(StopReason::Escalated(RefusalBranch::NoInterventionBeatsRefusal {
            ..
        })) => {}
        other => panic!("expected branch (1) refusal, got {other:?}"),
    }
}

#[test]
fn branch_two_when_reference_class_is_determined_empty() {
    let gamma = ConstraintStructure::new("gamma", |_b: &BoundaryData| Vec::new());
    let terms = ZeroTerms;
    let boundary = BoundaryData::new(vec![Fact {
        id: "f1".into(),
        classification: Classification::Irreversible,
    }]);
    let candidates = vec![Intervention { id: "nudge".into(), effect: vec![1.0, 1.0] }];
    let ledger = RepairLedger::new(&gamma, &terms, weights(), 0.25);
    let h = History::new("h0", vec![0.0, 0.0]);
    let outcome = ledger.diagnose_and_repair(&h, &boundary, &candidates);
    assert_eq!(
        outcome,
        RepairOutcome::Refused(StopReason::Escalated(RefusalBranch::EmptyReferenceClass))
    );
}

#[test]
fn branch_three_when_a_boundary_fact_is_unclassified() {
    let gamma = ConstraintStructure::new("gamma", |_b: &BoundaryData| Vec::new());
    let terms = ZeroTerms;
    let boundary = BoundaryData::new(vec![
        Fact { id: "f1".into(), classification: Classification::Revisable },
        Fact { id: "f2".into(), classification: Classification::Unclassified },
    ]);
    let candidates = vec![Intervention { id: "nudge".into(), effect: vec![1.0, 1.0] }];
    let ledger = RepairLedger::new(&gamma, &terms, weights(), 0.25);
    let h = History::new("h0", vec![0.0, 0.0]);
    let outcome = ledger.diagnose_and_repair(&h, &boundary, &candidates);
    match outcome {
        RepairOutcome::Refused(StopReason::Escalated(RefusalBranch::CoordinateDeficiency {
            unclassified_fact_ids,
        })) => {
            assert_eq!(unclassified_fact_ids, vec!["f2".to_string()]);
        }
        other => panic!("expected branch (3) refusal, got {other:?}"),
    }
}
