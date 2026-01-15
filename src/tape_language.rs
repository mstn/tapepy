use open_hypergraphs::lax::{Arrow as _, Monoidal, OpenHypergraph};

pub trait GeneratorShape {
    fn arity(&self) -> usize;
    fn coarity(&self) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Monomial<S> {
    One,
    Atom(S),
    Product(Box<Monomial<S>>, Box<Monomial<S>>),
}

impl<S> Monomial<S> {
    pub fn one() -> Self {
        Monomial::One
    }

    pub fn atom(sort: S) -> Self {
        Monomial::Atom(sort)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Polynomial<S> {
    Zero,
    Monomial(Monomial<S>),
    Sum(Box<Polynomial<S>>, Box<Polynomial<S>>),
}

impl<S> Polynomial<S> {
    pub fn zero() -> Self {
        Polynomial::Zero
    }

    pub fn monomial(term: Monomial<S>) -> Self {
        Polynomial::Monomial(term)
    }

    pub fn sum(left: Polynomial<S>, right: Polynomial<S>) -> Self {
        match (left, right) {
            (Polynomial::Zero, right) => right,
            (left, Polynomial::Zero) => left,
            (left, right) => Polynomial::Sum(Box::new(left), Box::new(right)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Circuit<S, G> {
    Id(S),
    IdOne,
    Generator(G),
    Swap { left: S, right: S },
    Seq(Box<Circuit<S, G>>, Box<Circuit<S, G>>),
    Product(Box<Circuit<S, G>>, Box<Circuit<S, G>>),
    Copy(S),
    Discard(S),
    Join(S),
}

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
    Sum(Box<Tape<S, G>>, Box<Tape<S, G>>),
    Discard(Monomial<S>),
    Split(Monomial<S>),
    Create(Monomial<S>),
    Merge(Monomial<S>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitArity {
    pub inputs: usize,
    pub outputs: usize,
}

impl CircuitArity {
    pub fn new(inputs: usize, outputs: usize) -> Self {
        Self { inputs, outputs }
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

impl<S, G: GeneratorShape> Circuit<S, G> {
    pub fn typing(&self) -> CircuitArity {
        match self {
            Circuit::Id(_) => CircuitArity::new(1, 1),
            Circuit::IdOne => CircuitArity::new(0, 0),
            Circuit::Generator(gen) => CircuitArity::new(gen.arity(), gen.coarity()),
            Circuit::Swap { .. } => CircuitArity::new(2, 2),
            Circuit::Seq(left, right) => {
                let left_ty = left.typing();
                let right_ty = right.typing();
                if left_ty.outputs != right_ty.inputs {
                    panic!(
                        "sequence arity mismatch: {} vs {}",
                        left_ty.outputs, right_ty.inputs
                    );
                }
                CircuitArity::new(left_ty.inputs, right_ty.outputs)
            }
            Circuit::Product(left, right) => {
                let left_ty = left.typing();
                let right_ty = right.typing();
                CircuitArity::new(left_ty.inputs + right_ty.inputs, left_ty.outputs + right_ty.outputs)
            }
            Circuit::Copy(_) => CircuitArity::new(1, 2),
            Circuit::Discard(_) => CircuitArity::new(1, 0),
            Circuit::Join(_) => CircuitArity::new(2, 1),
        }
    }
}

impl<S, G: GeneratorShape> Tape<S, G> {
    pub fn typing(&self) -> TapeArity {
        match self {
            Tape::Id(mono) => {
                let len = mono.len();
                TapeArity::new(len, len)
            }
            Tape::IdZero => TapeArity::new(0, 0),
            Tape::EmbedCircuit(circuit) => {
                let ty = circuit.typing();
                TapeArity::new(ty.inputs, ty.outputs)
            }
            Tape::Swap { left, right } => {
                let inputs = left.len() + right.len();
                TapeArity::new(inputs, inputs)
            }
            Tape::Seq(left, right) => {
                let left_ty = left.typing();
                let right_ty = right.typing();
                if left_ty.outputs != right_ty.inputs {
                    panic!(
                        "sequence arity mismatch: {} vs {}",
                        left_ty.outputs, right_ty.inputs
                    );
                }
                TapeArity::new(left_ty.inputs, right_ty.outputs)
            }
            Tape::Sum(left, right) => {
                let left_ty = left.typing();
                let right_ty = right.typing();
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

impl<S: Clone, G: GeneratorShape + Clone> Circuit<S, G> {
    pub fn to_hypergraph<F>(&self, fresh_sort: &mut F) -> OpenHypergraph<S, G>
    where
        F: FnMut() -> S,
    {
        match self {
            Circuit::Id(sort) => OpenHypergraph::identity(vec![sort.clone()]),
            Circuit::IdOne => OpenHypergraph::empty(),
            Circuit::Generator(gen) => OpenHypergraph::singleton(
                gen.clone(),
                fresh_sorts(fresh_sort, gen.arity()),
                fresh_sorts(fresh_sort, gen.coarity()),
            ),
            Circuit::Swap { left, right } => {
                let mut graph = OpenHypergraph::empty();
                let left_id = graph.new_node(left.clone());
                let right_id = graph.new_node(right.clone());
                graph.sources = vec![left_id, right_id];
                graph.targets = vec![right_id, left_id];
                graph
            }
            Circuit::Seq(left, right) => {
                let left_graph = left.to_hypergraph(fresh_sort);
                let right_graph = right.to_hypergraph(fresh_sort);
                compose_lax_unchecked(&left_graph, &right_graph)
            }
            Circuit::Product(left, right) => {
                let left_graph = left.to_hypergraph(fresh_sort);
                let right_graph = right.to_hypergraph(fresh_sort);
                left_graph.tensor(&right_graph)
            }
            Circuit::Copy(sort) => {
                let mut graph = OpenHypergraph::empty();
                let node = graph.new_node(sort.clone());
                graph.sources = vec![node];
                graph.targets = vec![node, node];
                graph
            }
            Circuit::Discard(sort) => {
                let mut graph = OpenHypergraph::empty();
                let node = graph.new_node(sort.clone());
                graph.sources = vec![node];
                graph.targets = Vec::new();
                graph
            }
            Circuit::Join(sort) => {
                let mut graph = OpenHypergraph::empty();
                let node = graph.new_node(sort.clone());
                graph.sources = vec![node, node];
                graph.targets = vec![node];
                graph
            }
        }
    }
}

fn fresh_sorts<S, F: FnMut() -> S>(fresh_sort: &mut F, count: usize) -> Vec<S> {
    (0..count).map(|_| fresh_sort()).collect()
}

fn compose_lax_unchecked<S: Clone + PartialEq, G: Clone>(
    lhs: &OpenHypergraph<S, G>,
    rhs: &OpenHypergraph<S, G>,
) -> OpenHypergraph<S, G> {
    // Lax composition: we only check interface lengths here, sort checking is deferred.
    if lhs.targets.len() != rhs.sources.len() {
        panic!(
            "unchecked composition requires same arity, got {} vs {}",
            lhs.targets.len(),
            rhs.sources.len()
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
