use open_hypergraphs::lax::{Arrow as _, Monoidal, OpenHypergraph};
use std::fmt;

use super::{compose_lax_unchecked, GeneratorShape, GeneratorTypes};

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
pub struct CircuitArity {
    pub inputs: usize,
    pub outputs: usize,
}

impl CircuitArity {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permutation(pub Vec<usize>);

impl Permutation {
    pub fn is_identity(&self) -> bool {
        self.0.iter().enumerate().all(|(idx, val)| idx == *val)
    }
}

pub fn identity_for_types<S: Clone, G>(types: &[S]) -> Circuit<S, G> {
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

pub fn wiring_circuit_for_context<S: Clone, G>(
    context_entries: &[(String, S)],
    input_vars: &[String],
) -> Circuit<S, G> {
    let mut counts = Vec::with_capacity(context_entries.len());
    for (name, _) in context_entries {
        let count = input_vars.iter().filter(|var| *var == name).count();
        counts.push(count);
    }

    let mut var_circuits = Vec::with_capacity(context_entries.len());
    for ((_, ty), count) in context_entries.iter().zip(counts.iter().copied()) {
        var_circuits.push(copy_many(ty.clone(), count));
    }
    let grouped = product_many(var_circuits);

    let grouped_types = grouped_types(context_entries, &counts);
    let permutation = permutation_for_inputs(context_entries, input_vars, &counts);
    if permutation.is_identity() {
        grouped
    } else {
        let perm = permute_circuit(&grouped_types, &permutation);
        Circuit::Seq(Box::new(grouped), Box::new(perm))
    }
}

fn grouped_types<S: Clone>(context_entries: &[(String, S)], counts: &[usize]) -> Vec<S> {
    let mut types = Vec::new();
    for ((_, ty), count) in context_entries.iter().zip(counts.iter().copied()) {
        for _ in 0..count {
            types.push(ty.clone());
        }
    }
    types
}

fn permutation_for_inputs<S>(
    context_entries: &[(String, S)],
    input_vars: &[String],
    counts: &[usize],
) -> Permutation {
    let mut offsets = Vec::with_capacity(counts.len());
    let mut running = 0;
    for count in counts {
        offsets.push(running);
        running += *count;
    }

    let mut seen = vec![0usize; counts.len()];
    let mut permutation = Vec::with_capacity(input_vars.len());
    for name in input_vars {
        let idx = context_entries
            .iter()
            .position(|(var, _)| var == name)
            .unwrap_or_else(|| panic!("variable `{}` not in context", name));
        let offset = offsets[idx];
        let use_idx = offset + seen[idx];
        seen[idx] += 1;
        permutation.push(use_idx);
    }
    Permutation(permutation)
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
