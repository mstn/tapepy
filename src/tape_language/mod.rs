use open_hypergraphs::lax::{Monoidal, OpenHypergraph};
use std::fmt;

pub mod circuit;
pub mod tape;

pub use circuit::Circuit;
pub use tape::{inverse_left_distributor, left_distributor, swap_poly, Tape, TapeEdge, Whisker};

pub trait GeneratorShape {
    fn arity(&self) -> usize;
    fn coarity(&self) -> usize;
}

pub trait GeneratorTypes<S> {
    fn input_types(&self) -> Option<Vec<S>>;
    fn output_types(&self) -> Option<Vec<S>>;
}

#[derive(Debug, Clone, Hash)]
pub enum Monomial<S> {
    One,
    Atom(S),
    Product(Box<Monomial<S>>, Box<Monomial<S>>),
}

impl<S: fmt::Display> fmt::Display for Monomial<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Monomial::One => write!(f, "1"),
            Monomial::Atom(sort) => write!(f, "{}", sort),
            Monomial::Product(_, _) => {
                let mut parts = Vec::new();
                collect_monomial_parts(self, &mut parts);
                write!(f, "{}", parts.join(" * "))
            }
        }
    }
}

impl<S: fmt::Display + Clone> fmt::Display for Polynomial<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        collect_polynomial_terms(self, &mut parts);
        if parts.is_empty() {
            return write!(f, "0");
        }
        write!(f, "{}", parts.join(" + "))
    }
}

fn collect_monomial_parts<S: fmt::Display>(mono: &Monomial<S>, parts: &mut Vec<String>) {
    match mono {
        Monomial::One => {}
        Monomial::Atom(sort) => parts.push(sort.to_string()),
        Monomial::Product(left, right) => {
            collect_monomial_parts(left, parts);
            collect_monomial_parts(right, parts);
        }
    }
}

fn monomial_sorts<S: Clone>(mono: &Monomial<S>) -> Vec<S> {
    match mono {
        Monomial::One => Vec::new(),
        Monomial::Atom(sort) => vec![sort.clone()],
        Monomial::Product(left, right) => {
            let mut parts = monomial_sorts(left);
            parts.extend(monomial_sorts(right));
            parts
        }
    }
}

// Note: equality is associative, so (a*b)*c == a*(b*c). We compare flattened sort lists.
impl<S: PartialEq + Clone> PartialEq for Monomial<S> {
    fn eq(&self, other: &Self) -> bool {
        monomial_sorts(self) == monomial_sorts(other)
    }
}

impl<S: Eq + Clone> Eq for Monomial<S> {}

impl<S> Monomial<S> {
    pub fn one() -> Self {
        Monomial::One
    }

    pub fn atom(sort: S) -> Self {
        Monomial::Atom(sort)
    }

    pub fn from_sorts<I>(sorts: I) -> Self
    where
        I: IntoIterator<Item = S>,
    {
        sorts.into_iter().fold(Monomial::one(), |acc, sort| {
            Monomial::product(acc, Monomial::atom(sort))
        })
    }

    /// Build a monomial from context entries; entry names are discarded.
    pub fn from_context(entries: &[(String, S)]) -> Self
    where
        S: Clone,
    {
        Monomial::from_sorts(entries.iter().map(|(_, ty)| ty.clone()))
    }

    pub fn product(left: Monomial<S>, right: Monomial<S>) -> Self {
        match (left, right) {
            (Monomial::One, right) => right,
            (left, Monomial::One) => left,
            (left, right) => Monomial::Product(Box::new(left), Box::new(right)),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Monomial::One => 0,
            Monomial::Atom(_) => 1,
            Monomial::Product(left, right) => left.len() + right.len(),
        }
    }
}

impl<S> From<S> for Monomial<S> {
    fn from(sort: S) -> Self {
        Monomial::atom(sort)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Polynomial<S: Clone> {
    Zero,
    Monomial(Monomial<S>),
    Sum(Box<Polynomial<S>>, Box<Polynomial<S>>),
}

fn collect_polynomial_terms<S: fmt::Display + Clone>(
    poly: &Polynomial<S>,
    parts: &mut Vec<String>,
) {
    match poly {
        Polynomial::Zero => {}
        Polynomial::Monomial(term) => parts.push(term.to_string()),
        Polynomial::Sum(left, right) => {
            collect_polynomial_terms(left, parts);
            collect_polynomial_terms(right, parts);
        }
    }
}

impl<S: Clone> Polynomial<S> {
    pub fn zero() -> Self {
        Polynomial::Zero
    }

    pub fn monomial(term: Monomial<S>) -> Self {
        Polynomial::Monomial(term)
    }

    pub fn from_monomial_sorts<I, J>(monomials: I) -> Self
    where
        I: IntoIterator<Item = J>,
        J: IntoIterator<Item = S>,
    {
        monomials
            .into_iter()
            .map(Monomial::from_sorts)
            .map(Polynomial::monomial)
            .fold(Polynomial::zero(), Polynomial::sum)
    }

    pub fn from_monomials<I>(monomials: I) -> Self
    where
        I: IntoIterator<Item = Monomial<S>>,
    {
        monomials
            .into_iter()
            .map(Polynomial::monomial)
            .fold(Polynomial::zero(), Polynomial::sum)
    }

    pub fn product(left: Polynomial<S>, right: Polynomial<S>) -> Self
    where
        S: Clone,
    {
        let left_terms = Polynomial::into_monomials(left);
        let right_terms = Polynomial::into_monomials(right);
        if left_terms.is_empty() || right_terms.is_empty() {
            return Polynomial::zero();
        }
        let mut acc = Polynomial::zero();
        for lhs in left_terms {
            for rhs in right_terms.iter().cloned() {
                let term = Monomial::product(lhs.clone(), rhs);
                acc = Polynomial::sum(acc, Polynomial::monomial(term));
            }
        }
        acc
    }

    pub fn sum(left: Polynomial<S>, right: Polynomial<S>) -> Self {
        match (left, right) {
            (Polynomial::Zero, right) => right,
            (left, Polynomial::Zero) => left,
            (left, right) => Polynomial::Sum(Box::new(left), Box::new(right)),
        }
    }

    fn into_monomials(poly: Polynomial<S>) -> Vec<Monomial<S>> {
        match poly {
            Polynomial::Zero => Vec::new(),
            Polynomial::Monomial(term) => vec![term],
            Polynomial::Sum(left, right) => {
                let mut terms = Polynomial::into_monomials(*left);
                terms.extend(Polynomial::into_monomials(*right));
                terms
            }
        }
    }
}

pub(crate) fn compose_lax_unchecked<S: Clone + PartialEq, G: Clone>(
    lhs: &OpenHypergraph<S, G>,
    rhs: &OpenHypergraph<S, G>,
) -> OpenHypergraph<S, G> {
    // Lax composition: we only check interface lengths here; sort checking is deferred.
    if lhs.targets.len() != rhs.sources.len() {
        panic!(
            "unchecked composition requires same arity, got {} vs {} (lhs: {}, rhs: {})",
            lhs.targets.len(),
            rhs.sources.len(),
            describe_open_hypergraph(lhs),
            describe_open_hypergraph(rhs)
        );
    }
    let n = lhs.hypergraph.nodes.len();
    let mut composed = lhs.tensor(rhs);

    for (u, v) in lhs.targets.iter().zip(rhs.sources.iter()) {
        composed.unify(*u, open_hypergraphs::lax::NodeId(v.0 + n));
    }

    composed.sources = composed.sources[..lhs.sources.len()].to_vec();
    composed.targets = composed.targets[lhs.targets.len()..].to_vec();
    composed
}

fn describe_open_hypergraph<O, A>(graph: &OpenHypergraph<O, A>) -> String {
    let sources = format_node_list(&graph.sources);
    let targets = format_node_list(&graph.targets);
    let mut edges = Vec::with_capacity(graph.hypergraph.adjacency.len());
    for (idx, edge) in graph.hypergraph.adjacency.iter().enumerate() {
        let edge_sources = format_node_list(&edge.sources);
        let edge_targets = format_node_list(&edge.targets);
        edges.push(format!("e{}: {} -> {}", idx, edge_sources, edge_targets));
    }
    format!(
        "nodes={} sources={} targets={} edges=[{}]",
        graph.hypergraph.nodes.len(),
        sources,
        targets,
        edges.join(", ")
    )
}

fn format_node_list(nodes: &[open_hypergraphs::lax::NodeId]) -> String {
    let mut parts = Vec::with_capacity(nodes.len());
    for node in nodes {
        parts.push(format!("n{}", node.0));
    }
    format!("[{}]", parts.join(", "))
}
