use open_hypergraphs::lax::{Arrow as _, Monoidal, OpenHypergraph};
use std::fmt;


pub trait GeneratorShape {
    fn arity(&self) -> usize;
    fn coarity(&self) -> usize;
}

pub trait GeneratorTypes<S> {
    fn input_types(&self) -> Option<Vec<S>>;
    fn output_types(&self) -> Option<Vec<S>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl<S> From<S> for Monomial<S> {
    fn from(sort: S) -> Self {
        Monomial::atom(sort)
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

impl<S: fmt::Display, G: fmt::Display> fmt::Display for Circuit<S, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Circuit::Id(sort) => write!(f, "id({})", sort),
            Circuit::IdOne => write!(f, "id(1)"),
            Circuit::Generator(gen) => write!(f, "{}", gen),
            Circuit::Swap { left, right } => write!(f, "swap({}, {})", left, right),
            Circuit::Seq(left, right) => write!(f, "{}; {}", left, right),
            Circuit::Product(left, right) => write!(f, "{} ⊗ {}", left, right),
            Circuit::Copy(sort) => write!(f, "copy({})", sort),
            Circuit::Discard(sort) => write!(f, "discard({})", sort),
            Circuit::Join(sort) => write!(f, "join({})", sort),
        }
    }
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
    pub fn id(terms: Vec<S>) -> Self {
        let mut circuits: Vec<Self> = terms.into_iter().map(Circuit::Id).collect();
        product_many(circuits)
    }

    pub fn copy_n(terms: Vec<S>) -> Self
    where
        S: Clone,
    {
        if terms.is_empty() {
            return Circuit::IdOne;
        }

        let mut copies: Vec<Self> = terms.iter().cloned().map(Circuit::Copy).collect();
        let mut acc = copies.remove(0);
        for circuit in copies {
            acc = Circuit::Product(Box::new(acc), Box::new(circuit));
        }

        if terms.len() == 1 {
            return acc;
        }

        let mut grouped_types = Vec::with_capacity(terms.len() * 2);
        for term in &terms {
            grouped_types.push(term.clone());
            grouped_types.push(term.clone());
        }
        let mut permutation = Vec::with_capacity(terms.len() * 2);
        for i in 0..terms.len() {
            permutation.push(2 * i);
        }
        for i in 0..terms.len() {
            permutation.push(2 * i + 1);
        }

        let permute = permute_circuit(&grouped_types, &Permutation(permutation));
        Circuit::Seq(Box::new(acc), Box::new(permute))
    }

    pub fn join_n(terms: Vec<S>) -> Self
    where
        S: Clone,
    {
        if terms.is_empty() {
            return Circuit::IdOne;
        }

        let mut joins: Vec<Self> = terms.iter().cloned().map(Circuit::Join).collect();
        let mut acc = joins.remove(0);
        for circuit in joins {
            acc = Circuit::Product(Box::new(acc), Box::new(circuit));
        }

        if terms.len() == 1 {
            return acc;
        }

        let mut interleaved_types = Vec::with_capacity(terms.len() * 2);
        for term in &terms {
            interleaved_types.push(term.clone());
        }
        for term in &terms {
            interleaved_types.push(term.clone());
        }

        let mut permutation = Vec::with_capacity(terms.len() * 2);
        for i in 0..terms.len() {
            permutation.push(2 * i);
        }
        for i in 0..terms.len() {
            permutation.push(2 * i + 1);
        }
        let mut inverse = vec![0usize; permutation.len()];
        for (idx, val) in permutation.iter().copied().enumerate() {
            inverse[val] = idx;
        }

        let permute = permute_circuit(&interleaved_types, &Permutation(inverse));
        Circuit::Seq(Box::new(permute), Box::new(acc))
    }

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
                CircuitArity::new(
                    left_ty.inputs + right_ty.inputs,
                    left_ty.outputs + right_ty.outputs,
                )
            }
            Circuit::Copy(_) => CircuitArity::new(1, 2),
            Circuit::Discard(_) => CircuitArity::new(1, 0),
            Circuit::Join(_) => CircuitArity::new(2, 1),
        }
    }
}

impl<S: Clone + PartialEq, G: GeneratorTypes<S>> Circuit<S, G> {
    pub fn io_types(&self) -> Option<(Vec<S>, Vec<S>)> {
        match self {
            Circuit::Id(sort) => Some((vec![sort.clone()], vec![sort.clone()])),
            Circuit::IdOne => Some((Vec::new(), Vec::new())),
            Circuit::Generator(gen) => match (gen.input_types(), gen.output_types()) {
                (Some(inputs), Some(outputs)) => Some((inputs, outputs)),
                _ => None,
            },
            Circuit::Swap { left, right } => Some((
                vec![left.clone(), right.clone()],
                vec![right.clone(), left.clone()],
            )),
            Circuit::Seq(left, right) => {
                let (left_in, left_out) = left.io_types()?;
                let (right_in, right_out) = right.io_types()?;
                if left_out != right_in {
                    return None;
                }
                Some((left_in, right_out))
            }
            Circuit::Product(left, right) => {
                let (left_in, left_out) = left.io_types()?;
                let (right_in, right_out) = right.io_types()?;
                let mut inputs = left_in;
                inputs.extend(right_in);
                let mut outputs = left_out;
                outputs.extend(right_out);
                Some((inputs, outputs))
            }
            Circuit::Copy(sort) => Some((vec![sort.clone()], vec![sort.clone(), sort.clone()])),
            Circuit::Discard(sort) => Some((vec![sort.clone()], Vec::new())),
            Circuit::Join(sort) => Some((vec![sort.clone(), sort.clone()], vec![sort.clone()])),
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
            Tape::Product(left, right) => {
                let left_ty = left.typing();
                let right_ty = right.typing();
                TapeArity::new(
                    left_ty.inputs + right_ty.inputs,
                    left_ty.outputs + right_ty.outputs,
                )
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

impl<S: Clone + PartialEq, G: GeneratorShape + GeneratorTypes<S> + Clone> Circuit<S, G> {
    pub fn to_hypergraph<F>(&self, fresh_sort: &mut F) -> OpenHypergraph<S, G>
    where
        F: FnMut() -> S,
    {
        match self {
            Circuit::Id(sort) => OpenHypergraph::identity(vec![sort.clone()]),
            Circuit::IdOne => OpenHypergraph::empty(),
            Circuit::Generator(gen) => OpenHypergraph::singleton(
                gen.clone(),
                gen.input_types()
                    .unwrap_or_else(|| fresh_sorts(fresh_sort, gen.arity())),
                gen.output_types()
                    .unwrap_or_else(|| fresh_sorts(fresh_sort, gen.coarity())),
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
        let arity = circuit.typing();
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

fn fresh_sorts<S, F: FnMut() -> S>(fresh_sort: &mut F, count: usize) -> Vec<S> {
    (0..count).map(|_| fresh_sort()).collect()
}

pub fn permute_circuit<S: Clone, G>(types: &[S], permutation: &Permutation) -> Circuit<S, G> {
    if permutation.is_identity() {
        return identity_for_types(types);
    }

    let mut current: Vec<usize> = (0..types.len()).collect();
    let mut current_types: Vec<S> = types.to_vec();
    let mut swaps = Vec::new();

    for target_idx in 0..permutation.0.len() {
        let desired = permutation.0[target_idx];
        let mut pos = current
            .iter()
            .position(|idx| *idx == desired)
            .unwrap_or_else(|| panic!("permutation missing index {}", desired));
        while pos > target_idx {
            swaps.push(swap_adjacent(&current_types, pos - 1));
            current.swap(pos - 1, pos);
            current_types.swap(pos - 1, pos);
            pos -= 1;
        }
    }

    swaps
        .into_iter()
        .fold(identity_for_types(types), |acc, swap| {
            Circuit::Seq(Box::new(acc), Box::new(swap))
        })
}

pub struct Permutation(pub Vec<usize>);

impl Permutation {
    pub fn is_identity(&self) -> bool {
        self.0.iter().enumerate().all(|(idx, val)| idx == *val)
    }
}

fn identity_for_types<S: Clone, G>(types: &[S]) -> Circuit<S, G> {
    if types.is_empty() {
        return Circuit::IdOne;
    }
    let mut circuits = Vec::with_capacity(types.len());
    for ty in types {
        circuits.push(Circuit::Id(ty.clone()));
    }
    product_many(circuits)
}

pub fn product_many<S, G>(mut circuits: Vec<Circuit<S, G>>) -> Circuit<S, G> {
    if circuits.is_empty() {
        return Circuit::IdOne;
    }
    let mut acc = circuits.remove(0);
    for circuit in circuits {
        acc = Circuit::Product(Box::new(acc), Box::new(circuit));
    }
    acc
}

pub fn copy_many<S: Clone, G>(ty: S, count: usize) -> Circuit<S, G> {
    match count {
        0 => Circuit::Discard(ty),
        1 => Circuit::Id(ty),
        2 => Circuit::Copy(ty),
        _ => {
            // Expand fanout by one wire at a time.
            let left = Circuit::Id(ty.clone());
            let right = copy_many(ty.clone(), count - 1);
            let prod = Circuit::Product(Box::new(left), Box::new(right));
            Circuit::Seq(Box::new(Circuit::Copy(ty)), Box::new(prod))
        }
    }
}

fn swap_adjacent<S: Clone, G>(types: &[S], index: usize) -> Circuit<S, G> {
    let left = identity_for_types(&types[..index]);
    let mid = Circuit::Swap {
        left: types[index].clone(),
        right: types[index + 1].clone(),
    };
    let right = identity_for_types(&types[index + 2..]);

    match (left, right) {
        (Circuit::IdOne, Circuit::IdOne) => mid,
        (Circuit::IdOne, right) => Circuit::Product(Box::new(mid), Box::new(right)),
        (left, Circuit::IdOne) => Circuit::Product(Box::new(left), Box::new(mid)),
        (left, right) => {
            let mid_right = Circuit::Product(Box::new(mid), Box::new(right));
            Circuit::Product(Box::new(left), Box::new(mid_right))
        }
    }
}

fn compose_lax_unchecked<S: Clone + PartialEq, G: Clone>(
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
