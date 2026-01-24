use open_hypergraphs::lax::{Arrow as _, Monoidal, OpenHypergraph};
use std::fmt::{self, Display};

use super::{compose_lax_unchecked, Circuit, GeneratorShape, GeneratorTypes, Monomial, Polynomial};

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
    pub fn copy_wires(monomial: Monomial<S>) -> Tape<S, G>
    where
        S: Clone,
    {
        let atoms = monomial_atom_sorts(&monomial);
        Tape::EmbedCircuit(Box::new(Circuit::copy_wires(atoms)))
    }

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

impl<S: Clone + PartialEq + Display, G: GeneratorTypes<S>> Tape<S, G> {
    pub fn io_types(&self) -> Option<(Vec<Monomial<S>>, Vec<Monomial<S>>)> {
        match self {
            Tape::Id(mono) => {
                let atoms = monomial_atoms(mono);
                Some((atoms.clone(), atoms))
            }
            Tape::IdZero => Some((Vec::new(), Vec::new())),
            Tape::EmbedCircuit(circuit) => {
                let (inputs, outputs) = circuit.io_types()?;
                Some((
                    inputs.into_iter().map(Monomial::atom).collect(),
                    outputs.into_iter().map(Monomial::atom).collect(),
                ))
            }
            Tape::Swap { left, right } => {
                let mut inputs = monomial_atoms(left);
                inputs.extend(monomial_atoms(right));
                let mut outputs = monomial_atoms(right);
                outputs.extend(monomial_atoms(left));
                Some((inputs, outputs))
            }
            Tape::Seq(left, right) => {
                let (left_in, left_out) = left.io_types()?;
                let (right_in, right_out) = right.io_types()?;
                if left_out != right_in {
                    return None;
                }
                Some((left_in, right_out))
            }
            Tape::Product(left, right) => {
                let (left_in, left_out) = left.io_types()?;
                let (right_in, right_out) = right.io_types()?;
                let mut inputs = left_in;
                inputs.extend(right_in);
                let mut outputs = left_out;
                outputs.extend(right_out);
                Some((inputs, outputs))
            }
            Tape::Sum(left, right) => {
                let (left_in, left_out) = left.io_types()?;
                let (right_in, right_out) = right.io_types()?;
                let mut inputs = left_in;
                inputs.extend(right_in);
                let mut outputs = left_out;
                outputs.extend(right_out);
                Some((inputs, outputs))
            }
            Tape::Discard(mono) => Some((monomial_atoms(mono), Vec::new())),
            Tape::Split(mono) => {
                let atoms = monomial_atoms(mono);
                let mut outputs = atoms.clone();
                outputs.extend(atoms.clone());
                Some((atoms, outputs))
            }
            Tape::Create(mono) => Some((Vec::new(), monomial_atoms(mono))),
            Tape::Merge(mono) => {
                let atoms = monomial_atoms(mono);
                let mut inputs = atoms.clone();
                inputs.extend(atoms.clone());
                Some((inputs, atoms))
            }
        }
    }
}

pub trait Whisker<Rhs> {
    type Output;

    fn left_whisk(&self, rhs: &Rhs) -> Self::Output;
    fn right_whisk(&self, rhs: &Rhs) -> Self::Output;
}

impl<S: Clone, G: Clone> Tape<S, G> {
    fn left_whisk_poly(&self, poly: &Polynomial<S>) -> Tape<S, G> {
        match poly {
            Polynomial::Zero => Tape::IdZero,
            Polynomial::Monomial(term) => self.left_whisk_mono(term),
            Polynomial::Sum(left, right) => Tape::Sum(
                Box::new(self.left_whisk_poly(left)),
                Box::new(self.left_whisk_poly(right)),
            ),
        }
    }

    fn left_whisk_mono(&self, left: &Monomial<S>) -> Tape<S, G> {
        match self {
            Tape::IdZero => Tape::IdZero,
            Tape::EmbedCircuit(circuit) => {
                let id_left = Circuit::id(monomial_atom_sorts(left));
                let whiskered = Circuit::product(id_left, circuit.as_ref().clone());
                Tape::EmbedCircuit(Box::new(whiskered))
            }
            Tape::Seq(head, tail) => Tape::Seq(
                Box::new(head.left_whisk_mono(left)),
                Box::new(tail.left_whisk_mono(left)),
            ),
            Tape::Product(left_tape, right_tape) => Tape::Product(
                Box::new(left_tape.left_whisk_mono(left)),
                Box::new(right_tape.left_whisk_mono(left)),
            ),
            Tape::Sum(left_tape, right_tape) => Tape::Sum(
                Box::new(left_tape.left_whisk_mono(left)),
                Box::new(right_tape.left_whisk_mono(left)),
            ),
            Tape::Id(right) => Tape::Id(Monomial::product(left.clone(), right.clone())),
            Tape::Swap {
                left: swap_left,
                right: swap_right,
            } => Tape::Swap {
                left: Monomial::product(left.clone(), swap_left.clone()),
                right: Monomial::product(left.clone(), swap_right.clone()),
            },
            Tape::Discard(right) => Tape::Discard(Monomial::product(left.clone(), right.clone())),
            Tape::Split(right) => Tape::Split(Monomial::product(left.clone(), right.clone())),
            Tape::Create(right) => Tape::Create(Monomial::product(left.clone(), right.clone())),
            Tape::Merge(right) => Tape::Merge(Monomial::product(left.clone(), right.clone())),
        }
    }

    pub fn right_whisk(&self, right: &Monomial<S>) -> Tape<S, G> {
        match self {
            Tape::IdZero => Tape::IdZero,
            Tape::EmbedCircuit(circuit) => {
                let id_right = Circuit::id(monomial_atom_sorts(right));
                let whiskered = Circuit::product(circuit.as_ref().clone(), id_right);
                Tape::EmbedCircuit(Box::new(whiskered))
            }
            Tape::Seq(head, tail) => Tape::Seq(
                Box::new(head.right_whisk(right)),
                Box::new(tail.right_whisk(right)),
            ),
            Tape::Product(left_tape, right_tape) => Tape::Product(
                Box::new(left_tape.right_whisk(right)),
                Box::new(right_tape.right_whisk(right)),
            ),
            Tape::Sum(left_tape, right_tape) => Tape::Sum(
                Box::new(left_tape.right_whisk(right)),
                Box::new(right_tape.right_whisk(right)),
            ),
            Tape::Id(left) => Tape::Id(Monomial::product(left.clone(), right.clone())),
            Tape::Swap {
                left: swap_left,
                right: swap_right,
            } => Tape::Swap {
                left: Monomial::product(swap_left.clone(), right.clone()),
                right: Monomial::product(swap_right.clone(), right.clone()),
            },
            Tape::Discard(left) => {
                Tape::Discard(Monomial::product(left.clone(), right.clone()))
            }
            Tape::Split(left) => Tape::Split(Monomial::product(left.clone(), right.clone())),
            Tape::Create(left) => Tape::Create(Monomial::product(left.clone(), right.clone())),
            Tape::Merge(left) => Tape::Merge(Monomial::product(left.clone(), right.clone())),
        }
    }
}

impl<S: Clone, G: Clone> Whisker<Monomial<S>> for Tape<S, G> {
    type Output = Tape<S, G>;

    fn left_whisk(&self, rhs: &Monomial<S>) -> Self::Output {
        self.left_whisk_mono(rhs)
    }

    fn right_whisk(&self, rhs: &Monomial<S>) -> Self::Output {
        self.right_whisk(rhs)
    }
}

impl<S: Clone, G: Clone> Whisker<Polynomial<S>> for Tape<S, G> {
    type Output = Tape<S, G>;

    fn left_whisk(&self, rhs: &Polynomial<S>) -> Self::Output {
        self.left_whisk_poly(rhs)
    }

    fn right_whisk(&self, _: &Polynomial<S>) -> Self::Output {
        panic!("right whisking by a polynomial is not implemented");
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

pub fn monomial_atoms<S: Clone>(monomial: &Monomial<S>) -> Vec<Monomial<S>> {
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

fn monomial_atom_sorts<S: Clone>(monomial: &Monomial<S>) -> Vec<S> {
    monomial_atoms(monomial)
        .into_iter()
        .map(|atom| match atom {
            Monomial::Atom(sort) => sort,
            Monomial::One | Monomial::Product(_, _) => {
                panic!("expected monomial atoms from monomial_atoms")
            }
        })
        .collect()
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
