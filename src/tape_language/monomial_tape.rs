use open_hypergraphs::lax::OpenHypergraph;
use std::fmt;

use super::circuit::Circuit;
use super::tape::monomial_atoms;
use super::{compose_lax_unchecked, GeneratorShape, GeneratorTypes, Monomial, Tape};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonomialTapeError {
    NonMonomialArity { inputs: usize, outputs: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TensorKind {
    Additive,
    Multiplicative,
}

impl TensorKind {
    pub fn flipped(self) -> Self {
        match self {
            TensorKind::Additive => TensorKind::Multiplicative,
            TensorKind::Multiplicative => TensorKind::Additive,
        }
    }
}

impl fmt::Display for TensorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TensorKind::Additive => write!(f, "add"),
            TensorKind::Multiplicative => write!(f, "mul"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonomialHyperNode<S: Clone> {
    pub tensor_kind: TensorKind,
    pub context: Monomial<S>,
}

impl<S: Clone> MonomialHyperNode<S> {
    pub fn new(tensor_kind: TensorKind, context: Monomial<S>) -> Self {
        Self {
            tensor_kind,
            context,
        }
    }
}

impl<S: Clone + fmt::Display> fmt::Display for MonomialHyperNode<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.tensor_kind, self.context)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MonomialTapeEdge<S: Clone, G> {
    Generator(G),
    CircuitCopy,
    CircuitDiscard,
    CircuitJoin,
    TapeDiscard,
    TapeSplit,
    TapeCreate,
    TapeMerge,
    FromAddToMul(Monomial<S>),
    FromMulToAdd(Monomial<S>),
}

impl<S: Clone, G> MonomialTapeEdge<S, G> {
    pub fn multiplicity(&self) -> usize {
        1
    }
}

impl<S: fmt::Display + Clone, G: fmt::Display> fmt::Display for MonomialTapeEdge<S, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonomialTapeEdge::Generator(generator) => write!(f, "{}", generator),
            MonomialTapeEdge::CircuitCopy => write!(f, "copy"),
            MonomialTapeEdge::CircuitDiscard => write!(f, "discard"),
            MonomialTapeEdge::CircuitJoin => write!(f, "join"),
            MonomialTapeEdge::TapeDiscard => write!(f, "tape-discard"),
            MonomialTapeEdge::TapeSplit => write!(f, "split"),
            MonomialTapeEdge::TapeCreate => write!(f, "create"),
            MonomialTapeEdge::TapeMerge => write!(f, "merge"),
            MonomialTapeEdge::FromAddToMul(mono) => write!(f, "from-add-to-mul({})", mono),
            MonomialTapeEdge::FromMulToAdd(mono) => write!(f, "from-mul-to-add({})", mono),
        }
    }
}

#[derive(Clone)]
pub struct MonomialTape<S: Clone, G> {
    tape: Tape<S, G>,
}

impl<S: Clone, G> MonomialTape<S, G> {
    pub fn into_inner(self) -> Tape<S, G> {
        self.tape
    }

    pub fn as_tape(&self) -> &Tape<S, G> {
        &self.tape
    }
}

impl<S: Clone + PartialEq, G: GeneratorTypes<S>> MonomialTape<S, G> {
    pub fn try_from_tape(tape: Tape<S, G>) -> Result<Self, MonomialTapeError> {
        Ok(Self { tape })
    }

    pub fn io_monomials(&self) -> Result<(Monomial<S>, Monomial<S>), MonomialTapeError> {
        let (inputs, outputs) = self
            .tape
            .io_types()
            .expect("monomial tape requires io types");
        Ok((inputs[0].clone(), outputs[0].clone()))
    }

    pub fn seq(left: MonomialTape<S, G>, right: MonomialTape<S, G>) -> MonomialTape<S, G> {
        let composed = Tape::seq(left.tape, right.tape);
        MonomialTape::try_from_tape(composed)
            .unwrap_or_else(|_| panic!("monomial tape seq preserves interface"))
    }

    pub fn product(left: &MonomialTape<S, G>, right: &MonomialTape<S, G>) -> MonomialTape<S, G>
    where
        S: fmt::Debug + fmt::Display,
        G: Clone + fmt::Display + fmt::Debug,
    {
        let product = Tape::product(&left.tape, &right.tape);
        MonomialTape::try_from_tape(product)
            .unwrap_or_else(|_| panic!("monomial tape product preserves interface"))
    }
}

impl<
        S: Clone + PartialEq + fmt::Debug + fmt::Display,
        G: GeneratorShape + GeneratorTypes<S> + Clone + fmt::Display,
    > MonomialTape<S, G>
{
    pub fn to_hypergraph(
        &self,
        fresh_sort: &mut impl FnMut() -> S,
    ) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
        translate_tape(&self.tape, TensorKind::Multiplicative, fresh_sort)
    }
}

fn translate_tape<
    S: Clone + PartialEq + fmt::Debug + fmt::Display,
    G: GeneratorShape + GeneratorTypes<S> + Clone + fmt::Display,
>(
    tape: &Tape<S, G>,
    boundary_kind: TensorKind,
    fresh_sort: &mut impl FnMut() -> S,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    match tape {
        Tape::Id(mono) => identity_graph(mono, boundary_kind),
        Tape::IdZero => OpenHypergraph::empty(),
        Tape::EmbedCircuit(circuit) => {
            let graph = translate_circuit(circuit.as_ref(), fresh_sort);
            adapt_boundary_kind(graph, boundary_kind, TensorKind::Multiplicative)
        }
        Tape::Swap { left, right } => swap_graph(left, right, boundary_kind),
        Tape::Trace { around, tape } => {
            let mut graph = translate_tape(tape, boundary_kind, fresh_sort);
            let around_len = monomial_atoms(around).len();
            if graph.sources.len() < around_len || graph.targets.len() < around_len {
                panic!(
                    "trace arity mismatch: sources {}, targets {}, around {}",
                    graph.sources.len(),
                    graph.targets.len(),
                    around_len
                );
            }
            let sources = graph.sources.clone();
            let targets = graph.targets.clone();
            for idx in 0..around_len {
                graph.unify(sources[idx], targets[idx]);
            }
            graph.sources = sources[around_len..].to_vec();
            graph.targets = targets[around_len..].to_vec();
            graph
        }
        Tape::Seq(left, right) => {
            let left_graph = translate_tape(left, boundary_kind, fresh_sort);
            let right_graph = translate_tape(right, boundary_kind, fresh_sort);
            compose_lax_unchecked(&left_graph, &right_graph)
        }
        Tape::Product(left, right) | Tape::Sum(left, right) => {
            let left_graph = translate_tape(left, boundary_kind, fresh_sort);
            let right_graph = translate_tape(right, boundary_kind, fresh_sort);
            left_graph.tensor(&right_graph)
        }
        Tape::Discard(mono) => {
            let graph = discard_graph(mono, TensorKind::Additive);
            adapt_boundary_kind(graph, boundary_kind, TensorKind::Additive)
        }
        Tape::Split(mono) => {
            let graph = copy_graph(mono, TensorKind::Additive);
            adapt_boundary_kind(graph, boundary_kind, TensorKind::Additive)
        }
        Tape::Create(mono) => {
            let graph = create_graph(mono, TensorKind::Additive);
            adapt_boundary_kind(graph, boundary_kind, TensorKind::Additive)
        }
        Tape::Merge(mono) => {
            let graph = join_graph(mono, TensorKind::Additive);
            adapt_boundary_kind(graph, boundary_kind, TensorKind::Additive)
        }
    }
}

fn translate_circuit<
    S: Clone + PartialEq + fmt::Debug + fmt::Display,
    G: GeneratorShape + GeneratorTypes<S> + Clone + fmt::Display,
>(
    circuit: &Circuit<S, G>,
    fresh_sort: &mut impl FnMut() -> S,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    match circuit {
        Circuit::Id(sort) => identity_graph(&Monomial::atom(sort.clone()), TensorKind::Multiplicative),
        Circuit::IdOne => OpenHypergraph::empty(),
        Circuit::Generator(generator) => OpenHypergraph::singleton(
            MonomialTapeEdge::Generator(generator.clone()),
            atomic_nodes_from_sorts(
                generator
                    .input_types()
                    .unwrap_or_else(|| fresh_sorts(fresh_sort, generator.arity())),
                TensorKind::Multiplicative,
            ),
            atomic_nodes_from_sorts(
                generator
                    .output_types()
                    .unwrap_or_else(|| fresh_sorts(fresh_sort, generator.coarity())),
                TensorKind::Multiplicative,
            ),
        ),
        Circuit::Swap { left, right } => swap_graph(
            &Monomial::atom(left.clone()),
            &Monomial::atom(right.clone()),
            TensorKind::Multiplicative,
        ),
        Circuit::Seq(left, right) => {
            let left_graph = translate_circuit(left, fresh_sort);
            let right_graph = translate_circuit(right, fresh_sort);
            compose_lax_unchecked(&left_graph, &right_graph)
        }
        Circuit::Product(left, right) => {
            let left_graph = translate_circuit(left, fresh_sort);
            let right_graph = translate_circuit(right, fresh_sort);
            left_graph.tensor(&right_graph)
        }
        Circuit::Copy(sort) => copy_graph(&Monomial::atom(sort.clone()), TensorKind::Multiplicative),
        Circuit::Discard(sort) => {
            discard_graph(&Monomial::atom(sort.clone()), TensorKind::Multiplicative)
        }
        Circuit::Join(sort) => join_graph(&Monomial::atom(sort.clone()), TensorKind::Multiplicative),
    }
}

fn adapt_boundary_kind<
    S: Clone + PartialEq + fmt::Debug + fmt::Display,
    G: Clone + fmt::Display,
>(
    graph: OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>,
    boundary_kind: TensorKind,
    intrinsic_kind: TensorKind,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    if boundary_kind == intrinsic_kind {
        return graph;
    }

    let source_contexts = interface_contexts(&graph, &graph.sources);
    let target_contexts = interface_contexts(&graph, &graph.targets);
    let into_intrinsic = conversion_graph(&source_contexts, boundary_kind, intrinsic_kind);
    let out_of_intrinsic = conversion_graph(&target_contexts, intrinsic_kind, boundary_kind);
    let graph = compose_lax_unchecked(&into_intrinsic, &graph);
    compose_lax_unchecked(&graph, &out_of_intrinsic)
}

fn identity_graph<S: Clone, G>(
    mono: &Monomial<S>,
    tensor_kind: TensorKind,
) -> OpenHypergraph<MonomialHyperNode<S>, G> {
    OpenHypergraph::identity(atomic_nodes(mono, tensor_kind))
}

fn swap_graph<S: Clone, G>(
    left: &Monomial<S>,
    right: &Monomial<S>,
    tensor_kind: TensorKind,
) -> OpenHypergraph<MonomialHyperNode<S>, G> {
    let mut graph = OpenHypergraph::empty();
    let left_nodes = add_nodes(&mut graph, &atomic_nodes(left, tensor_kind));
    let right_nodes = add_nodes(&mut graph, &atomic_nodes(right, tensor_kind));
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

fn discard_graph<S: Clone, G>(
    mono: &Monomial<S>,
    tensor_kind: TensorKind,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    let labels = atomic_nodes(mono, tensor_kind);
    let mut graph = OpenHypergraph::empty();
    let mut sources = Vec::with_capacity(labels.len());
    for label in labels {
        sources.push(graph.new_node(label));
    }
    graph.sources = sources;
    graph.targets = Vec::new();
    graph
}

fn create_graph<S: Clone, G>(
    mono: &Monomial<S>,
    tensor_kind: TensorKind,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    let labels = atomic_nodes(mono, tensor_kind);
    let mut graph = OpenHypergraph::empty();
    let mut targets = Vec::with_capacity(labels.len());
    for label in labels {
        targets.push(graph.new_node(label));
    }
    graph.sources = Vec::new();
    graph.targets = targets;
    graph
}

fn copy_graph<S: Clone, G>(
    mono: &Monomial<S>,
    tensor_kind: TensorKind,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    let labels = atomic_nodes(mono, tensor_kind);
    let mut graph = OpenHypergraph::empty();
    let mut sources = Vec::with_capacity(labels.len());
    let mut left_targets = Vec::with_capacity(labels.len());
    let mut right_targets = Vec::with_capacity(labels.len());
    for label in labels {
        let node = graph.new_node(label);
        sources.push(node);
        left_targets.push(node);
        right_targets.push(node);
    }
    let mut targets = left_targets;
    targets.extend(right_targets);
    graph.sources = sources;
    graph.targets = targets;
    graph
}

fn join_graph<S: Clone, G>(
    mono: &Monomial<S>,
    tensor_kind: TensorKind,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    let labels = atomic_nodes(mono, tensor_kind);
    let mut graph = OpenHypergraph::empty();
    let mut left_sources = Vec::with_capacity(labels.len());
    let mut right_sources = Vec::with_capacity(labels.len());
    let mut targets = Vec::with_capacity(labels.len());
    for label in labels {
        let node = graph.new_node(label);
        left_sources.push(node);
        right_sources.push(node);
        targets.push(node);
    }
    let mut sources = left_sources;
    sources.extend(right_sources);
    graph.sources = sources;
    graph.targets = targets;
    graph
}

fn conversion_graph<S: Clone, G>(
    contexts: &[Monomial<S>],
    from_kind: TensorKind,
    to_kind: TensorKind,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    let mut graph = OpenHypergraph::empty();
    let mut sources = Vec::with_capacity(contexts.len());
    let mut targets = Vec::with_capacity(contexts.len());
    for context in contexts {
        let source = graph.new_node(MonomialHyperNode::new(from_kind, context.clone()));
        let target = graph.new_node(MonomialHyperNode::new(to_kind, context.clone()));
        let edge = match (from_kind, to_kind) {
            (TensorKind::Additive, TensorKind::Multiplicative) => {
                MonomialTapeEdge::FromAddToMul(context.clone())
            }
            (TensorKind::Multiplicative, TensorKind::Additive) => {
                MonomialTapeEdge::FromMulToAdd(context.clone())
            }
            _ => panic!("conversion graph requires distinct tensor kinds"),
        };
        graph.new_edge(edge, (vec![source], vec![target]));
        sources.push(source);
        targets.push(target);
    }
    graph.sources = sources;
    graph.targets = targets;
    graph
}

fn interface_contexts<S: Clone, G>(
    graph: &OpenHypergraph<MonomialHyperNode<S>, G>,
    interface: &[open_hypergraphs::lax::NodeId],
) -> Vec<Monomial<S>> {
    interface
        .iter()
        .map(|node_id| graph.hypergraph.nodes[node_id.0].context.clone())
        .collect()
}

fn atomic_nodes<S: Clone>(mono: &Monomial<S>, tensor_kind: TensorKind) -> Vec<MonomialHyperNode<S>> {
    monomial_atoms(mono)
        .into_iter()
        .map(|context| MonomialHyperNode::new(tensor_kind, context))
        .collect()
}

fn atomic_nodes_from_sorts<S: Clone>(
    sorts: Vec<S>,
    tensor_kind: TensorKind,
) -> Vec<MonomialHyperNode<S>> {
    sorts
        .into_iter()
        .map(Monomial::atom)
        .map(|context| MonomialHyperNode::new(tensor_kind, context))
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

fn fresh_sorts<S>(fresh_sort: &mut impl FnMut() -> S, count: usize) -> Vec<S> {
    (0..count).map(|_| fresh_sort()).collect()
}
