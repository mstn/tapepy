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

impl<S, G> Circuit<S, G> {
    pub fn seq(left: Circuit<S, G>, right: Circuit<S, G>) -> Self {
        match (left, right) {
            (Circuit::IdOne, right) => right,
            (left, Circuit::IdOne) => left,
            (left, right) => Circuit::Seq(Box::new(left), Box::new(right)),
        }
    }

    pub fn product(left: Circuit<S, G>, right: Circuit<S, G>) -> Self {
        match (left, right) {
            (Circuit::IdOne, right) => right,
            (left, Circuit::IdOne) => left,
            (left, right) => Circuit::Product(Box::new(left), Box::new(right)),
        }
    }

    pub fn id(terms: Vec<S>) -> Self {
        let circuits: Vec<Self> = terms.into_iter().map(Circuit::Id).collect();
        Circuit::product_many(circuits)
    }

    pub fn copy_wires(terms: Vec<S>) -> Self
    where
        S: Clone,
    {
        if terms.is_empty() {
            return Circuit::IdOne;
        }

        let mut copies: Vec<Self> = terms.iter().cloned().map(Circuit::Copy).collect();
        let mut acc = copies.remove(0);
        for circuit in copies {
            acc = Circuit::product(acc, circuit);
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

        let permute = Circuit::permute_circuit(&grouped_types, &Permutation(permutation));
        Circuit::seq(acc, permute)
    }

    pub fn copy_wire_n_times(ty: S, count: usize) -> Self
    where
        S: Clone,
    {
        match count {
            0 => Circuit::Discard(ty),
            1 => Circuit::Id(ty),
            2 => Circuit::Copy(ty),
            _ => {
                // Expand fanout by one wire at a time.
                let left = Circuit::Id(ty.clone());
                let right = Circuit::copy_wire_n_times(ty.clone(), count - 1);
                let prod = Circuit::product(left, right);
                Circuit::seq(Circuit::Copy(ty), prod)
            }
        }
    }

    pub fn join_wires(terms: Vec<S>) -> Self
    where
        S: Clone,
    {
        if terms.is_empty() {
            return Circuit::IdOne;
        }

        let mut joins: Vec<Self> = terms.iter().cloned().map(Circuit::Join).collect();
        let mut acc = joins.remove(0);
        for circuit in joins {
            acc = Circuit::product(acc, circuit);
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

        let permute = Circuit::permute_circuit(&interleaved_types, &Permutation(inverse));
        Circuit::seq(permute, acc)
    }

    fn permute_circuit(types: &[S], permutation: &Permutation) -> Self
    where
        S: Clone,
    {
        if permutation.is_identity() {
            return Circuit::id(types.to_vec());
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
            .fold(Circuit::id(types.to_vec()), |acc, swap| {
                Circuit::seq(acc, swap)
            })
    }

    pub fn product_many(mut circuits: Vec<Circuit<S, G>>) -> Circuit<S, G> {
        if circuits.is_empty() {
            return Circuit::IdOne;
        }
        let mut acc = circuits.remove(0);
        for circuit in circuits {
            acc = Circuit::product(acc, circuit);
        }
        acc
    }

    pub fn swap_blocks(types_left: &[S], types_right: &[S]) -> Circuit<S, G>
    where
        S: Clone,
    {
        let mut types = Vec::with_capacity(types_left.len() + types_right.len());
        types.extend_from_slice(types_left);
        types.extend_from_slice(types_right);

        let mut permutation = Vec::with_capacity(types.len());
        for i in 0..types_right.len() {
            permutation.push(types_left.len() + i);
        }
        for i in 0..types_left.len() {
            permutation.push(i);
        }

        Circuit::permute_circuit(&types, &Permutation(permutation))
    }

    pub fn wiring_circuit_for_context(
        context_entries: &[(String, S)],
        input_vars: &[String],
    ) -> Circuit<S, G>
    where
        S: Clone,
    {
        let mut counts = Vec::with_capacity(context_entries.len());
        for (name, _) in context_entries {
            let count = input_vars.iter().filter(|var| *var == name).count();
            counts.push(count);
        }

        let mut var_circuits = Vec::with_capacity(context_entries.len());
        for ((_, ty), count) in context_entries.iter().zip(counts.iter().copied()) {
            var_circuits.push(Circuit::copy_wire_n_times(ty.clone(), count));
        }
        let grouped = Circuit::product_many(var_circuits);

        let grouped_types = grouped_types(context_entries, &counts);
        let permutation = permutation_for_inputs(context_entries, input_vars, &counts);
        if permutation.is_identity() {
            grouped
        } else {
            let perm = Circuit::permute_circuit(&grouped_types, &permutation);
            Circuit::seq(grouped, perm)
        }
    }
}

impl<S, G: GeneratorShape> Circuit<S, G> {
    pub fn arity(&self) -> CircuitArity {
        match self {
            Circuit::Id(_) => CircuitArity {
                inputs: 1,
                outputs: 1,
            },
            Circuit::IdOne => CircuitArity {
                inputs: 0,
                outputs: 0,
            },
            Circuit::Generator(gen) => CircuitArity {
                inputs: gen.arity(),
                outputs: gen.coarity(),
            },
            Circuit::Swap { .. } => CircuitArity {
                inputs: 2,
                outputs: 2,
            },
            Circuit::Seq(left, right) => {
                let left_ty = left.arity();
                let right_ty = right.arity();
                if left_ty.outputs != right_ty.inputs {
                    panic!(
                        "sequence arity mismatch: {} vs {}",
                        left_ty.outputs, right_ty.inputs
                    );
                }
                CircuitArity {
                    inputs: left_ty.inputs,
                    outputs: right_ty.outputs,
                }
            }
            Circuit::Product(left, right) => {
                let left_ty = left.arity();
                let right_ty = right.arity();
                CircuitArity {
                    inputs: left_ty.inputs + right_ty.inputs,
                    outputs: left_ty.outputs + right_ty.outputs,
                }
            }
            Circuit::Copy(_) => CircuitArity {
                inputs: 1,
                outputs: 2,
            },
            Circuit::Discard(_) => CircuitArity {
                inputs: 1,
                outputs: 0,
            },
            Circuit::Join(_) => CircuitArity {
                inputs: 2,
                outputs: 1,
            },
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Permutation(Vec<usize>);

impl Permutation {
    fn is_identity(&self) -> bool {
        self.0.iter().enumerate().all(|(idx, val)| idx == *val)
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
    let left = Circuit::id(types[..index].to_vec());
    let mid = Circuit::Swap {
        left: types[index].clone(),
        right: types[index + 1].clone(),
    };
    let right = Circuit::id(types[index + 2..].to_vec());

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
