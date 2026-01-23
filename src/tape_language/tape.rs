use open_hypergraphs::lax::{Arrow as _, Monoidal, OpenHypergraph};
use std::fmt;

use super::{compose_lax_unchecked, Circuit, GeneratorShape, GeneratorTypes, Monomial};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tape<S, G> {
    Id(Monomial<S>),
    IdZero,
    EmbedCircuit(Box<Circuit<S, G>>),
    Swap {
        left: Monomial<S>,
        right: Monomial<S>,
    },
    Seq(Box<Tape<S, G>>, Box<Tape<S, G>>),
    Product(Box<Tape<S, G>>, Box<Tape<S, G>>),
    Sum(Box<Tape<S, G>>, Box<Tape<S, G>>),
    Discard(Monomial<S>),
    Split(Monomial<S>),
    Create(Monomial<S>),
    Merge(Monomial<S>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TapeEdge<S, G> {
    Embedded(OpenHypergraph<S, G>),
    Product(
        Box<OpenHypergraph<Monomial<S>, TapeEdge<S, G>>>,
        Box<OpenHypergraph<Monomial<S>, TapeEdge<S, G>>>,
    ),
}

impl<S: fmt::Display, G: fmt::Display> fmt::Display for TapeEdge<S, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TapeEdge::Embedded(child) => {
                write!(f, "Embed({}x{})", child.sources.len(), child.targets.len())
            }
            TapeEdge::Product(left, right) => write!(
                f,
                "Product({}x{}, {}x{})",
                left.sources.len(),
                left.targets.len(),
                right.sources.len(),
                right.targets.len()
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapeArity {
    pub inputs: usize,
    pub outputs: usize,
}

impl TapeArity {
    pub fn new(inputs: usize, outputs: usize) -> Self {
        Self { inputs, outputs }
    }
}

impl<S, G: GeneratorShape> Tape<S, G> {
    pub fn arity(&self) -> TapeArity {
        match self {
            Tape::Id(mono) => {
                let len = mono.len();
                TapeArity::new(len, len)
            }
            Tape::IdZero => TapeArity::new(0, 0),
            Tape::EmbedCircuit(circuit) => {
                let ty = circuit.arity();
                TapeArity::new(ty.inputs, ty.outputs)
            }
            Tape::Swap { left, right } => {
                let inputs = left.len() + right.len();
                TapeArity::new(inputs, inputs)
            }
            Tape::Seq(left, right) => {
                let left_ty = left.arity();
                let right_ty = right.arity();
                if left_ty.outputs != right_ty.inputs {
                    panic!(
                        "sequence arity mismatch: {} vs {}",
                        left_ty.outputs, right_ty.inputs
                    );
                }
                TapeArity::new(left_ty.inputs, right_ty.outputs)
            }
            Tape::Product(left, right) => {
                let left_ty = left.arity();
                let right_ty = right.arity();
                TapeArity::new(
                    left_ty.inputs + right_ty.inputs,
                    left_ty.outputs + right_ty.outputs,
                )
            }
            Tape::Sum(left, right) => {
                let left_ty = left.arity();
                let right_ty = right.arity();
                if left_ty.inputs != right_ty.inputs || left_ty.outputs != right_ty.outputs {
                    panic!(
                        "sum arity mismatch: {}x{} vs {}x{}",
                        left_ty.inputs, left_ty.outputs, right_ty.inputs, right_ty.outputs
                    );
                }
                TapeArity::new(left_ty.inputs, left_ty.outputs)
            }
            Tape::Discard(mono) => TapeArity::new(mono.len(), 0),
            Tape::Split(mono) => {
                let len = mono.len();
                TapeArity::new(len, len)
            }
            Tape::Create(mono) => TapeArity::new(0, mono.len()),
            Tape::Merge(mono) => {
                let len = mono.len();
                TapeArity::new(len, len)
            }
        }
    }
}

impl<S: Clone + PartialEq, G: GeneratorShape + GeneratorTypes<S> + Clone> Tape<S, G> {
    pub fn to_hypergraph(
        &self,
        fresh_sort: &mut impl FnMut() -> S,
    ) -> OpenHypergraph<Monomial<S>, TapeEdge<S, G>> {
        match self {
            Tape::Id(mono) => OpenHypergraph::identity(monomial_atoms(mono)),
            Tape::IdZero => OpenHypergraph::empty(),
            Tape::EmbedCircuit(circuit) => match circuit.as_ref() {
                Circuit::Id(sort) => OpenHypergraph::identity(vec![Monomial::atom(sort.clone())]),
                Circuit::IdOne => OpenHypergraph::empty(),
                _ => {
                    let child_graph = circuit.to_hypergraph(fresh_sort);
                    let nodes = &child_graph.hypergraph.nodes;
                    let mut sources = Vec::with_capacity(child_graph.sources.len());
                    for node_id in &child_graph.sources {
                        sources.push(Monomial::atom(nodes[node_id.0].clone()));
                    }
                    let mut targets = Vec::with_capacity(child_graph.targets.len());
                    for node_id in &child_graph.targets {
                        targets.push(Monomial::atom(nodes[node_id.0].clone()));
                    }
                    OpenHypergraph::singleton(TapeEdge::Embedded(child_graph), sources, targets)
                }
            },
            Tape::Swap { left, right } => {
                let mut graph = OpenHypergraph::empty();
                let left_nodes = add_nodes(&mut graph, &monomial_atoms(left));
                let right_nodes = add_nodes(&mut graph, &monomial_atoms(right));
                graph.sources = left_nodes
                    .iter()
                    .chain(right_nodes.iter())
                    .copied()
                    .collect();
                graph.targets = right_nodes
                    .into_iter()
                    .chain(left_nodes.into_iter())
                    .collect();
                graph
            }
            Tape::Seq(left, right) => {
                let left_graph = left.to_hypergraph(fresh_sort);
                let right_graph = right.to_hypergraph(fresh_sort);
                compose_lax_unchecked(&left_graph, &right_graph)
            }
            Tape::Product(left, right) => {
                let left_graph = left.to_hypergraph(fresh_sort);
                let right_graph = right.to_hypergraph(fresh_sort);
                let mut sources = interface_labels(&left_graph, &left_graph.sources);
                sources.extend(interface_labels(&right_graph, &right_graph.sources));
                let mut targets = interface_labels(&left_graph, &left_graph.targets);
                targets.extend(interface_labels(&right_graph, &right_graph.targets));
                OpenHypergraph::singleton(
                    TapeEdge::Product(Box::new(left_graph), Box::new(right_graph)),
                    sources,
                    targets,
                )
            }
            Tape::Sum(left, right) => {
                let left_graph = left.to_hypergraph(fresh_sort);
                let right_graph = right.to_hypergraph(fresh_sort);
                left_graph.tensor(&right_graph)
            }
            Tape::Discard(mono) => {
                let mut graph = OpenHypergraph::empty();
                graph.sources = add_nodes(&mut graph, &monomial_atoms(mono));
                graph.targets = Vec::new();
                graph
            }
            Tape::Split(mono) => {
                let mut graph = OpenHypergraph::empty();
                let nodes = add_nodes(&mut graph, &monomial_atoms(mono));
                graph.sources = nodes.clone();
                graph.targets = nodes.iter().copied().chain(nodes.iter().copied()).collect();
                graph
            }
            Tape::Create(mono) => {
                let mut graph = OpenHypergraph::empty();
                graph.sources = Vec::new();
                graph.targets = add_nodes(&mut graph, &monomial_atoms(mono));
                graph
            }
            Tape::Merge(mono) => {
                let mut graph = OpenHypergraph::empty();
                let nodes = add_nodes(&mut graph, &monomial_atoms(mono));
                graph.sources = nodes.iter().copied().chain(nodes.iter().copied()).collect();
                graph.targets = nodes;
                graph
            }
        }
    }
}

fn interface_labels<S: Clone, G>(
    graph: &OpenHypergraph<Monomial<S>, G>,
    nodes: &[open_hypergraphs::lax::NodeId],
) -> Vec<Monomial<S>> {
    nodes
        .iter()
        .map(|node_id| graph.hypergraph.nodes[node_id.0].clone())
        .collect()
}

fn fresh_monomials<S>(fresh_sort: &mut impl FnMut() -> S, count: usize) -> Vec<Monomial<S>> {
    (0..count).map(|_| Monomial::atom(fresh_sort())).collect()
}

fn embed_circuit<S: Clone + PartialEq, G: GeneratorShape + GeneratorTypes<S> + Clone>(
    circuit: &Circuit<S, G>,
    fresh_sort: &mut impl FnMut() -> S,
) -> OpenHypergraph<Monomial<S>, Circuit<Monomial<S>, G>> {
    let lifted = lift_circuit(circuit);
    if let Some((inputs, outputs)) = circuit.io_types() {
        OpenHypergraph::singleton(
            lifted,
            inputs.into_iter().map(Monomial::atom).collect(),
            outputs.into_iter().map(Monomial::atom).collect(),
        )
    } else {
        let arity = circuit.arity();
        OpenHypergraph::singleton(
            lifted,
            fresh_monomials(fresh_sort, arity.inputs),
            fresh_monomials(fresh_sort, arity.outputs),
        )
    }
}

fn monomial_atoms<S: Clone>(monomial: &Monomial<S>) -> Vec<Monomial<S>> {
    match monomial {
        Monomial::One => Vec::new(),
        Monomial::Atom(sort) => vec![Monomial::atom(sort.clone())],
        Monomial::Product(left, right) => {
            let mut atoms = monomial_atoms(left);
            atoms.extend(monomial_atoms(right));
            atoms
        }
    }
}

fn add_nodes<S: Clone, G>(
    graph: &mut OpenHypergraph<S, G>,
    labels: &[S],
) -> Vec<open_hypergraphs::lax::NodeId> {
    labels
        .iter()
        .map(|label| graph.new_node(label.clone()))
        .collect()
}

fn lift_circuit<S: Clone, G: Clone>(circuit: &Circuit<S, G>) -> Circuit<Monomial<S>, G> {
    match circuit {
        Circuit::Id(sort) => Circuit::Id(Monomial::atom(sort.clone())),
        Circuit::IdOne => Circuit::IdOne,
        Circuit::Generator(gen) => Circuit::Generator(gen.clone()),
        Circuit::Swap { left, right } => Circuit::Swap {
            left: Monomial::atom(left.clone()),
            right: Monomial::atom(right.clone()),
        },
        Circuit::Seq(left, right) => {
            Circuit::Seq(Box::new(lift_circuit(left)), Box::new(lift_circuit(right)))
        }
        Circuit::Product(left, right) => {
            Circuit::Product(Box::new(lift_circuit(left)), Box::new(lift_circuit(right)))
        }
        Circuit::Copy(sort) => Circuit::Copy(Monomial::atom(sort.clone())),
        Circuit::Discard(sort) => Circuit::Discard(Monomial::atom(sort.clone())),
        Circuit::Join(sort) => Circuit::Join(Monomial::atom(sort.clone())),
    }
}
