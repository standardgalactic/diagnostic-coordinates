//! Core domain types for the repair/admissibility apparatus.
//!
//! These are deliberately minimal placeholders. `History`, `Continuation`,
//! and `Intervention` carry only enough structure (a feature vector) to
//! compute distances and let the objective function run end-to-end. Swap
//! the internals for your actual state representation once you're testing
//! a specific theory rather than the scaffolding.

/// A trajectory / history `h`. Damage, if any, is already baked into `features`.
#[derive(Debug, Clone, PartialEq)]
pub struct History {
    pub id: String,
    pub features: Vec<f64>,
}

impl History {
    pub fn new(id: impl Into<String>, features: Vec<f64>) -> Self {
        History { id: id.into(), features }
    }

    /// Apply an intervention to produce the post-intervention history `h'`.
    /// Placeholder semantics: elementwise addition of the intervention's
    /// effect vector, clamped to the same dimensionality.
    pub fn apply(&self, intervention: &Intervention) -> History {
        let features = self
            .features
            .iter()
            .zip(intervention.effect.iter().chain(std::iter::repeat(&0.0)))
            .map(|(f, e)| f + e)
            .collect();
        History { id: format!("{}+{}", self.id, intervention.id), features }
    }
}

/// A candidate admissible continuation, as produced either by `Ext_Γ(B(h))`
/// (the reference class `R_Γ(h)`) or by direct reachability (`F(h)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Continuation {
    pub id: String,
    pub features: Vec<f64>,
}

/// An intervention `a`. `id == "∅"` by convention denotes the null
/// intervention, which the Repair-Warrant Criterion requires to have
/// J(∅|h) = 0 exactly.
#[derive(Debug, Clone, PartialEq)]
pub struct Intervention {
    pub id: String,
    pub effect: Vec<f64>,
}

impl Intervention {
    pub fn null() -> Self {
        Intervention { id: "∅".to_string(), effect: Vec::new() }
    }

    pub fn is_null(&self) -> bool {
        self.id == "∅"
    }
}

/// How a single retained fact in the repair-time boundary data is classified.
/// `Unclassified` is the case that produces taxonomy branch (3): repair
/// can't yet tell whether an obstruction belongs to `B(h)` or to `Γ_h`
/// because this fact hasn't been pinned down as one or the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    Irreversible,
    Revisable,
    Unclassified,
}

/// A single retained fact within the repair-time boundary data `B(h)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    pub id: String,
    pub classification: Classification,
}

/// Repair-time boundary data `B(h)`: the facts the intervention is not
/// entitled to rewrite (to the extent they're classified `Irreversible`).
#[derive(Debug, Clone, Default)]
pub struct BoundaryData {
    pub facts: Vec<Fact>,
}

impl BoundaryData {
    pub fn new(facts: Vec<Fact>) -> Self {
        BoundaryData { facts }
    }

    pub fn has_unclassified(&self) -> bool {
        self.facts.iter().any(|f| f.classification == Classification::Unclassified)
    }
}
