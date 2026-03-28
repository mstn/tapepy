use open_hypergraphs::lax::OpenHypergraph;
use std::fmt::{self, Debug, Display};

use super::{compose_lax_unchecked, Circuit, GeneratorShape, GeneratorTypes, Monomial, Polynomial};

#[derive(Clone, PartialEq, Eq)]
pub enum Tape<S: Clone, G> {
    Id(Monomial<S>),
    IdZero,
    EmbedCircuit(Box<Circuit<S, G>>),
    Swap {
        left: Monomial<S>,
        right: Monomial<S>,
    },
    Trace {
        around: Monomial<S>,
        tape: Box<Tape<S, G>>,
    },
    Seq(Box<Tape<S, G>>, Box<Tape<S, G>>),
    Product(Box<Tape<S, G>>, Box<Tape<S, G>>),
    Sum(Box<Tape<S, G>>, Box<Tape<S, G>>),
    Discard(Monomial<S>),
    Split(Monomial<S>),
    Create(Monomial<S>),
    Merge(Monomial<S>),
}

impl<S: Clone + PartialEq + Display, G: GeneratorTypes<S> + Display> fmt::Debug for Tape<S, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_tape_tree(self, 0))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TapeEdge<S: Clone, G> {
    Embedded(OpenHypergraph<S, G>),
    Product(
        Box<OpenHypergraph<Monomial<S>, TapeEdge<S, G>>>,
        Box<OpenHypergraph<Monomial<S>, TapeEdge<S, G>>>,
    ),
}

impl<S: fmt::Display + Clone, G: fmt::Display> fmt::Display for TapeEdge<S, G> {
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

impl<S: Clone, G: GeneratorShape> Tape<S, G> {
    pub fn copy_wires(monomial: Monomial<S>) -> Tape<S, G>
    where
        S: Clone,
    {
        let atoms = monomial_atom_sorts(&monomial);
        Tape::EmbedCircuit(Box::new(Circuit::copy_wires(atoms)))
    }

    pub fn join_wires(monomial: Monomial<S>) -> Tape<S, G>
    where
        S: Clone,
    {
        let atoms = monomial_atom_sorts(&monomial);
        Tape::EmbedCircuit(Box::new(Circuit::join_wires(atoms)))
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
            Tape::Trace { around, tape } => {
                let inner = tape.arity();
                let around_len = around.len();
                if inner.inputs < around_len || inner.outputs < around_len {
                    panic!(
                        "trace arity mismatch: inner {}x{}, around {}",
                        inner.inputs, inner.outputs, around_len
                    );
                }
                TapeArity::new(inner.inputs - around_len, inner.outputs - around_len)
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
                TapeArity::new(
                    left_ty.inputs + right_ty.inputs,
                    left_ty.outputs + right_ty.outputs,
                )
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

impl<S: Clone + PartialEq, G: GeneratorTypes<S>> Tape<S, G> {
    pub fn io_types(&self) -> Option<(Vec<Monomial<S>>, Vec<Monomial<S>>)> {
        match self {
            Tape::Id(mono) => Some((vec![mono.clone()], vec![mono.clone()])),
            Tape::IdZero => Some((Vec::new(), Vec::new())),
            Tape::EmbedCircuit(circuit) => {
                let (inputs, outputs) = circuit.io_types()?;
                let inputs = vec![Monomial::from_sorts(inputs)];
                let outputs = vec![Monomial::from_sorts(outputs)];
                Some((inputs, outputs))
            }
            Tape::Swap { left, right } => {
                let inputs = vec![left.clone(), right.clone()];
                let outputs = vec![right.clone(), left.clone()];
                Some((inputs, outputs))
            }
            Tape::Trace { around, tape } => {
                let (inputs, outputs) = tape.io_types()?;
                let Some(input_head) = inputs.first() else {
                    return None;
                };
                let Some(output_head) = outputs.first() else {
                    return None;
                };
                if input_head != around || output_head != around {
                    return None;
                }
                Some((inputs[1..].to_vec(), outputs[1..].to_vec()))
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
                let inputs = Polynomial::into_monomials(Polynomial::product(
                    Polynomial::from_monomials(left_in),
                    Polynomial::from_monomials(right_in),
                ));
                let outputs = Polynomial::into_monomials(Polynomial::product(
                    Polynomial::from_monomials(left_out),
                    Polynomial::from_monomials(right_out),
                ));
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
            Tape::Discard(mono) => Some((vec![mono.clone()], Vec::new())),
            Tape::Split(mono) => {
                let inputs = vec![mono.clone()];
                let outputs = vec![mono.clone(), mono.clone()];
                Some((inputs, outputs))
            }
            Tape::Create(mono) => Some((Vec::new(), vec![mono.clone()])),
            Tape::Merge(mono) => {
                let outputs = vec![mono.clone()];
                let inputs = vec![mono.clone(), mono.clone()];
                Some((inputs, outputs))
            }
        }
    }
}

impl<S: Clone + PartialEq + Debug + Display, G: Debug + GeneratorTypes<S> + Clone + Display>
    Tape<S, G>
{
    pub fn validate(&self) -> Result<(), TapeValidationError<S>> {
        validate_tape(self, &mut Vec::new())
    }

    pub fn product(t1: &Tape<S, G>, t2: &Tape<S, G>) -> Tape<S, G> {
        let (p1_in, _q1_out) = t1.io_types().expect("product requires io types");
        let (_p2_in, q2_out) = t2.io_types().expect("product requires io types");
        let p1 = Polynomial::from_monomials(p1_in);
        let q2 = Polynomial::from_monomials(q2_out);

        let left = t2.left_whisk(&p1);
        let right = t1.right_whisk(&q2);
        // Product construction guarantees interfaces align, so we can safely
        // collapse Id;X or X;Id here without forcing full type resolution elsewhere.
        let tape = Tape::seq_with_id(left.clone(), right.clone());

        if let Err(err) = tape.validate() {
            // TODO maybe return Option
            println!("{:?}", t1);
            println!("{:?}", t2);
            println!("============");
            println!("{:?}", left);
            println!("{:?}", right);
            panic!("product invalid:\n{}", err);
        }

        return tape;
    }
}

pub trait Whisker<Rhs> {
    type Output;

    fn left_whisk(&self, rhs: &Rhs) -> Self::Output;
    fn right_whisk(&self, rhs: &Rhs) -> Self::Output;
}

impl<S: Clone, G> Tape<S, G> {
    pub fn sum(left: Tape<S, G>, right: Tape<S, G>) -> Tape<S, G> {
        match (left, right) {
            (Tape::IdZero, right) => right,
            (left, Tape::IdZero) => left,
            (left, right) => Tape::Sum(Box::new(left), Box::new(right)),
        }
    }

    pub fn trace_poly(poly: &Polynomial<S>, tape: Tape<S, G>) -> Tape<S, G> {
        let (head, rest) = split_polynomial(poly);
        let Some(head) = head else {
            return tape;
        };
        let traced = Tape::Trace {
            around: head,
            tape: Box::new(tape),
        };
        Tape::trace_poly(&rest, traced)
    }

    pub fn discard_poly(poly: &Polynomial<S>) -> Tape<S, G> {
        let mut terms = polynomial_monomials(poly);
        if terms.is_empty() {
            return Tape::IdZero;
        }
        let mut acc = Tape::Discard(terms.remove(0));
        for term in terms {
            acc = Tape::sum(acc, Tape::Discard(term));
        }
        acc
    }

    pub fn create_poly(poly: &Polynomial<S>) -> Tape<S, G> {
        let mut terms = polynomial_monomials(poly);
        if terms.is_empty() {
            return Tape::IdZero;
        }
        let mut acc = Tape::Create(terms.remove(0));
        for term in terms {
            acc = Tape::sum(acc, Tape::Create(term));
        }
        acc
    }

    pub fn seq(left: Tape<S, G>, right: Tape<S, G>) -> Tape<S, G> {
        match (left, right) {
            (Tape::IdZero, Tape::IdZero) => Tape::IdZero,
            (Tape::EmbedCircuit(left), Tape::EmbedCircuit(right)) => {
                let composed = Circuit::seq(*left, *right);
                Tape::EmbedCircuit(Box::new(composed))
            }
            (left, right) => Tape::Seq(Box::new(left), Box::new(right)),
        }
    }

    fn seq_with_id(left: Tape<S, G>, right: Tape<S, G>) -> Tape<S, G>
    where
        S: Clone + PartialEq,
        G: GeneratorTypes<S>,
    {
        match (&left, &right) {
            (Tape::Id(_), _) => {
                if let (Some((_left_in, left_out)), Some((right_in, _right_out))) =
                    (left.io_types(), right.io_types())
                {
                    if left_out == right_in {
                        return right;
                    }
                }
            }
            (_, Tape::Id(_)) => {
                if let (Some((_left_in, left_out)), Some((right_in, _right_out))) =
                    (left.io_types(), right.io_types())
                {
                    if left_out == right_in {
                        return left;
                    }
                }
            }
            _ => {}
        }
        Tape::seq(left, right)
    }

    pub fn left_distributor(
        poly: &Polynomial<S>,
        left: &Polynomial<S>,
        right: &Polynomial<S>,
    ) -> Tape<S, G>
    where
        S: Clone,
        G: Clone,
    {
        left_distributor(poly, left, right)
    }

    pub fn right_whisk_poly(&self, poly: &Polynomial<S>) -> Tape<S, G>
    where
        S: Clone + PartialEq + Debug + Display,
        G: Clone + GeneratorTypes<S>,
    {
        let (head, rest) = split_polynomial(poly);
        let Some(head) = head else {
            return Tape::IdZero;
        };

        if rest == Polynomial::zero() {
            return self.right_whisk_mono(&head);
        }

        let (inputs, outputs) = self
            .io_types()
            .expect("right whisking requires tape io types");
        let left_poly = Polynomial::from_monomials(inputs);
        let right_poly = Polynomial::from_monomials(outputs);

        let left_dist = left_distributor(&left_poly, &Polynomial::monomial(head.clone()), &rest);
        let mid = Tape::sum(self.right_whisk_mono(&head), self.right_whisk_poly(&rest));
        let right_dist = inverse_left_distributor(&right_poly, &Polynomial::monomial(head), &rest);

        Tape::seq(left_dist, Tape::seq(mid, right_dist))
    }

    fn left_whisk_poly(&self, poly: &Polynomial<S>) -> Tape<S, G>
    where
        G: Clone,
    {
        match poly {
            Polynomial::Zero => Tape::IdZero,
            Polynomial::Monomial(term) => self.left_whisk_mono(term),
            Polynomial::Sum(left, right) => {
                Tape::sum(self.left_whisk_poly(left), self.left_whisk_poly(right))
            }
        }
    }

    fn left_whisk_mono(&self, left: &Monomial<S>) -> Tape<S, G>
    where
        G: Clone,
    {
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
            Tape::Trace { around, tape } => Tape::Trace {
                around: Monomial::product(left.clone(), around.clone()),
                tape: Box::new(tape.left_whisk_mono(left)),
            },
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

    fn right_whisk_mono(&self, right: &Monomial<S>) -> Tape<S, G>
    where
        G: Clone,
    {
        match self {
            Tape::IdZero => Tape::IdZero,
            Tape::EmbedCircuit(circuit) => {
                let id_right = Circuit::id(monomial_atom_sorts(right));
                let whiskered = Circuit::product(circuit.as_ref().clone(), id_right);
                Tape::EmbedCircuit(Box::new(whiskered))
            }
            Tape::Seq(head, tail) => Tape::Seq(
                Box::new(head.right_whisk_mono(right)),
                Box::new(tail.right_whisk_mono(right)),
            ),
            Tape::Product(left_tape, right_tape) => Tape::Product(
                Box::new(left_tape.right_whisk_mono(right)),
                Box::new(right_tape.right_whisk_mono(right)),
            ),
            Tape::Sum(left_tape, right_tape) => Tape::Sum(
                Box::new(left_tape.right_whisk_mono(right)),
                Box::new(right_tape.right_whisk_mono(right)),
            ),
            Tape::Trace { around, tape } => Tape::Trace {
                around: Monomial::product(around.clone(), right.clone()),
                tape: Box::new(tape.right_whisk_mono(right)),
            },
            Tape::Id(left) => Tape::Id(Monomial::product(left.clone(), right.clone())),
            Tape::Swap {
                left: swap_left,
                right: swap_right,
            } => Tape::Swap {
                left: Monomial::product(swap_left.clone(), right.clone()),
                right: Monomial::product(swap_right.clone(), right.clone()),
            },
            Tape::Discard(left) => Tape::Discard(Monomial::product(left.clone(), right.clone())),
            Tape::Split(left) => Tape::Split(Monomial::product(left.clone(), right.clone())),
            Tape::Create(left) => Tape::Create(Monomial::product(left.clone(), right.clone())),
            Tape::Merge(left) => Tape::Merge(Monomial::product(left.clone(), right.clone())),
        }
    }
}

impl<S: Clone + PartialEq + Debug + Display, G: GeneratorTypes<S> + Clone + Display> Tape<S, G> {
    pub fn convolution(t1: Tape<S, G>, t2: Tape<S, G>) -> Tape<S, G> {
        let (p1_in, q1_out) = t1.io_types().expect("convolution requires io types");
        let (p2_in, q2_out) = t2.io_types().expect("convolution requires io types");
        if p1_in != p2_in || q1_out != q2_out {
            panic!("convolution requires matching input/output interfaces");
        }
        let p = Polynomial::from_monomials(p1_in);
        let q = Polynomial::from_monomials(q1_out);
        let split = split_poly(&p);
        let join = merge_poly(&q);
        Tape::seq(split, Tape::seq(Tape::sum(t1, t2), join))
    }

    pub fn zero(inputs: &Polynomial<S>, outputs: &Polynomial<S>) -> Tape<S, G> {
        Tape::seq(Tape::discard_poly(inputs), Tape::create_poly(outputs))
    }

    pub fn kleene(tape: Tape<S, G>) -> Tape<S, G> {
        let (inputs, outputs) = tape.io_types().expect("kleene requires io types");
        if inputs.len() != 1 || outputs.len() != 1 {
            println!("{:?}", tape);
            panic!("kleene requires tape of type U -> U");
        }
        let u = inputs[0].clone();
        let sum = Tape::sum(tape, Tape::Id(u.clone()));
        let body = Tape::seq(
            sum,
            Tape::seq(Tape::Merge(u.clone()), Tape::Split(u.clone())),
        );
        Tape::Trace {
            around: u,
            tape: Box::new(body),
        }
    }
}

pub fn left_distributor<S: Clone, G: Clone>(
    poly: &Polynomial<S>,
    left: &Polynomial<S>,
    right: &Polynomial<S>,
) -> Tape<S, G> {
    let (head, rest) = split_polynomial(poly);
    let Some(head) = head else {
        return Tape::IdZero;
    };

    let sum_left_right = Polynomial::sum(left.clone(), right.clone());
    let head_sum = monomial_times_poly(&head, &sum_left_right);
    let left_part = Tape::sum(id_poly(&head_sum), left_distributor(&rest, left, right));

    let head_left = monomial_times_poly(&head, left);
    let head_right = monomial_times_poly(&head, right);
    let rest_left = Polynomial::product(rest.clone(), left.clone());
    let rest_right = Polynomial::product(rest, right.clone());

    let swap = swap_sum_blocks(&head_right, &rest_left);
    let right_part = Tape::sum(Tape::sum(id_poly(&head_left), swap), id_poly(&rest_right));

    Tape::seq(left_part, right_part)
}

pub fn inverse_left_distributor<S: Clone, G: Clone>(
    poly: &Polynomial<S>,
    left: &Polynomial<S>,
    right: &Polynomial<S>,
) -> Tape<S, G> {
    let (head, rest) = split_polynomial(poly);
    let Some(head) = head else {
        return Tape::IdZero;
    };

    let head_left = monomial_times_poly(&head, left);
    let head_right = monomial_times_poly(&head, right);
    let rest_left = Polynomial::product(rest.clone(), left.clone());
    let rest_right = Polynomial::product(rest.clone(), right.clone());

    let right_part = Tape::sum(
        Tape::sum(
            id_poly(&head_left),
            swap_sum_blocks(&head_right, &rest_left),
        ),
        id_poly(&rest_right),
    );

    let sum_left_right = Polynomial::sum(left.clone(), right.clone());
    let head_sum = monomial_times_poly(&head, &sum_left_right);
    let left_part_inv = Tape::sum(
        id_poly(&head_sum),
        inverse_left_distributor(&rest, left, right),
    );

    Tape::seq(right_part, left_part_inv)
}

pub fn swap_poly<S: Clone, G: Clone>(left: &Polynomial<S>, right: &Polynomial<S>) -> Tape<S, G> {
    let (head, rest) = split_polynomial(right);
    let Some(head) = head else {
        return Tape::IdZero;
    };

    let head_poly = Polynomial::monomial(head.clone());
    let rest_poly = rest;
    let left_dist = left_distributor(left, &head_poly, &rest_poly);

    let sum_swaps = sum_swaps(left, &head);
    let right_part = Tape::sum(sum_swaps, swap_poly(left, &rest_poly));

    Tape::seq(left_dist, right_part)
}

impl<S: Clone, G: Clone> Whisker<Monomial<S>> for Tape<S, G> {
    type Output = Tape<S, G>;

    fn left_whisk(&self, rhs: &Monomial<S>) -> Self::Output {
        self.left_whisk_mono(rhs)
    }

    fn right_whisk(&self, rhs: &Monomial<S>) -> Self::Output {
        self.right_whisk_mono(rhs)
    }
}

impl<S: Clone + PartialEq + Debug + Display, G: GeneratorTypes<S> + Clone + Display>
    Whisker<Polynomial<S>> for Tape<S, G>
{
    type Output = Tape<S, G>;

    fn left_whisk(&self, rhs: &Polynomial<S>) -> Self::Output {
        self.left_whisk_poly(rhs)
    }

    fn right_whisk(&self, rhs: &Polynomial<S>) -> Self::Output {
        self.right_whisk_poly(rhs)
    }
}

fn polynomial_monomials<S: Clone>(poly: &Polynomial<S>) -> Vec<Monomial<S>> {
    match poly {
        Polynomial::Zero => Vec::new(),
        Polynomial::Monomial(term) => vec![term.clone()],
        Polynomial::Sum(left, right) => {
            let mut terms = polynomial_monomials(left);
            terms.extend(polynomial_monomials(right));
            terms
        }
    }
}

fn split_polynomial<S: Clone>(poly: &Polynomial<S>) -> (Option<Monomial<S>>, Polynomial<S>) {
    let mut terms = polynomial_monomials(poly);
    if terms.is_empty() {
        return (None, Polynomial::zero());
    }
    let head = terms.remove(0);
    let rest = Polynomial::from_monomials(terms);
    (Some(head), rest)
}

fn id_poly<S: Clone, G: Clone>(poly: &Polynomial<S>) -> Tape<S, G> {
    let mut terms = polynomial_monomials(poly);
    if terms.is_empty() {
        return Tape::IdZero;
    }
    let mut acc = Tape::Id(terms.remove(0));
    for term in terms {
        acc = Tape::sum(acc, Tape::Id(term));
    }
    acc
}

fn split_poly<S: Clone, G: Clone>(poly: &Polynomial<S>) -> Tape<S, G> {
    let terms = polynomial_monomials(poly);
    let Some(first) = terms.first().cloned() else {
        return Tape::IdZero;
    };
    let mut acc_terms = vec![first.clone()];
    let mut acc = Tape::Split(first);
    for term in terms.into_iter().skip(1) {
        let rest_poly = Polynomial::from_monomials(acc_terms.clone());
        let term_poly = Polynomial::monomial(term.clone());
        let combined = Tape::sum(acc, Tape::Split(term.clone()));

        // Reorder outputs: P + P + T + T -> P + T + P + T.
        let swap_mid = swap_sum_blocks(&rest_poly, &term_poly);
        let reorder = Tape::sum(
            id_poly(&rest_poly),
            Tape::sum(swap_mid, Tape::Id(term.clone())),
        );
        acc = Tape::seq(combined, reorder);
        acc_terms.push(term);
    }
    acc
}

fn merge_poly<S: Clone, G: Clone>(poly: &Polynomial<S>) -> Tape<S, G> {
    let terms = polynomial_monomials(poly);
    let Some(first) = terms.first().cloned() else {
        return Tape::IdZero;
    };
    let mut acc_terms = vec![first.clone()];
    let mut acc = Tape::Merge(first);
    for term in terms.into_iter().skip(1) {
        let rest_poly = Polynomial::from_monomials(acc_terms.clone());
        let term_poly = Polynomial::monomial(term.clone());

        // Reorder inputs: P + T + P + T -> P + P + T + T.
        let swap_mid = swap_sum_blocks(&term_poly, &rest_poly);
        let reorder = Tape::sum(
            id_poly(&rest_poly),
            Tape::sum(swap_mid, Tape::Id(term.clone())),
        );
        let combined = Tape::sum(acc, Tape::Merge(term.clone()));
        acc = Tape::seq(reorder, combined);
        acc_terms.push(term);
    }
    acc
}

fn monomial_times_poly<S: Clone>(mono: &Monomial<S>, poly: &Polynomial<S>) -> Polynomial<S> {
    Polynomial::from_monomials(
        polynomial_monomials(poly)
            .into_iter()
            .map(|term| Monomial::product(mono.clone(), term)),
    )
}

fn sum_swaps<S: Clone, G: Clone>(poly: &Polynomial<S>, right: &Monomial<S>) -> Tape<S, G> {
    let mut terms = polynomial_monomials(poly);
    if terms.is_empty() {
        return Tape::IdZero;
    }
    let right_types = monomial_atom_sorts(right);
    let mut acc = {
        let left_types = monomial_atom_sorts(&terms.remove(0));
        Tape::EmbedCircuit(Box::new(Circuit::swap_blocks(&left_types, &right_types)))
    };
    for term in terms {
        let left_types = monomial_atom_sorts(&term);
        let swap = Tape::EmbedCircuit(Box::new(Circuit::swap_blocks(&left_types, &right_types)));
        acc = Tape::sum(acc, swap);
    }
    acc
}

fn swap_sum_blocks<S: Clone, G>(left: &Polynomial<S>, right: &Polynomial<S>) -> Tape<S, G> {
    let left_types = polynomial_atom_sorts(left);
    let right_types = polynomial_atom_sorts(right);
    Tape::EmbedCircuit(Box::new(Circuit::swap_blocks(&left_types, &right_types)))
}

fn polynomial_atom_sorts<S: Clone>(poly: &Polynomial<S>) -> Vec<S> {
    let mut atoms = Vec::new();
    for term in polynomial_monomials(poly) {
        atoms.extend(monomial_atom_sorts(&term));
    }
    atoms
}

#[derive(Debug, Clone)]
pub struct TapeValidationError<S> {
    pub path: Vec<String>,
    pub left_out: Option<Vec<Monomial<S>>>,
    pub right_in: Option<Vec<Monomial<S>>>,
    pub left_tape: Option<String>,
    pub right_tape: Option<String>,
    pub full_tape: String,
}

impl<S: Debug + Display> fmt::Display for TapeValidationError<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = if self.path.is_empty() {
            "<root>".to_string()
        } else {
            self.path.join(" -> ")
        };
        writeln!(f, "path: {}", path)?;
        if let (Some(left), Some(right)) = (&self.left_out, &self.right_in) {
            writeln!(f, "left outputs: {}", format_monomials(left))?;
            writeln!(f, "right inputs: {}", format_monomials(right))?;
        }
        if let Some(left) = &self.left_tape {
            writeln!(f, "left tape:\n{}", left)?;
        }
        if let Some(right) = &self.right_tape {
            writeln!(f, "right tape:\n{}", right)?;
        }
        writeln!(f, "full tape:\n{}", self.full_tape)
    }
}

fn validate_tape<S: Clone + PartialEq + Debug + Display, G: GeneratorTypes<S> + Display>(
    tape: &Tape<S, G>,
    path: &mut Vec<String>,
) -> Result<(), TapeValidationError<S>> {
    match tape {
        Tape::Seq(left, right) => {
            path.push("Seq.left".to_string());
            validate_tape(left, path)?;
            path.pop();
            path.push("Seq.right".to_string());
            validate_tape(right, path)?;
            path.pop();

            let (left_in, left_out) = left.io_types().ok_or_else(|| TapeValidationError {
                path: path.clone(),
                left_out: None,
                right_in: None,
                left_tape: Some(format_tape_tree(left, 0)),
                right_tape: None,
                full_tape: format_tape_tree(tape, 0),
            })?;
            let (right_in, right_out) = right.io_types().ok_or_else(|| TapeValidationError {
                path: path.clone(),
                left_out: None,
                right_in: None,
                left_tape: None,
                right_tape: Some(format_tape_tree(right, 0)),
                full_tape: format_tape_tree(tape, 0),
            })?;

            if left_out != right_in {
                return Err(TapeValidationError {
                    path: path.clone(),
                    left_out: Some(left_out),
                    right_in: Some(right_in),
                    left_tape: Some(format_tape_tree(left, 0)),
                    right_tape: Some(format_tape_tree(right, 0)),
                    full_tape: format_tape_tree(tape, 0),
                });
            }
            let _ = (left_in, right_out);
            Ok(())
        }
        Tape::Sum(left, right) => {
            path.push("Sum.left".to_string());
            validate_tape(left, path)?;
            path.pop();
            path.push("Sum.right".to_string());
            validate_tape(right, path)?;
            path.pop();
            Ok(())
        }
        Tape::Product(left, right) => {
            path.push("Product.left".to_string());
            validate_tape(left, path)?;
            path.pop();
            path.push("Product.right".to_string());
            validate_tape(right, path)?;
            path.pop();
            Ok(())
        }
        Tape::Trace {
            around,
            tape: inner,
        } => {
            path.push("Trace".to_string());
            validate_tape(inner, path)?;
            path.pop();

            let Some((inputs, outputs)) = inner.io_types() else {
                return Err(TapeValidationError {
                    path: path.clone(),
                    left_out: None,
                    right_in: None,
                    left_tape: Some(format_tape_tree(inner, 0)),
                    right_tape: None,
                    full_tape: format_tape_tree(tape, 0),
                });
            };
            if inputs.first() != Some(around) || outputs.first() != Some(around) {
                return Err(TapeValidationError {
                    path: path.clone(),
                    left_out: Some(inputs),
                    right_in: Some(outputs),
                    left_tape: Some(format_tape_tree(inner, 0)),
                    right_tape: None,
                    full_tape: format_tape_tree(tape, 0),
                });
            }
            Ok(())
        }
        Tape::EmbedCircuit(circuit) => {
            if circuit.io_types().is_none() {
                return Err(TapeValidationError {
                    path: path.clone(),
                    left_out: None,
                    right_in: None,
                    left_tape: None,
                    right_tape: None,
                    full_tape: format_tape_tree(tape, 0),
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn format_monomials<S: Display>(monos: &[Monomial<S>]) -> String {
    if monos.is_empty() {
        return "0".to_string();
    }
    let mut parts = Vec::with_capacity(monos.len());
    for mono in monos {
        parts.push(format!("{}", mono));
    }
    parts.join(" + ")
}

fn format_sorts<S: Display>(sorts: &[S]) -> String {
    if sorts.is_empty() {
        return "1".to_string();
    }
    let mut parts = Vec::with_capacity(sorts.len());
    for sort in sorts {
        parts.push(format!("{}", sort));
    }
    parts.join(" * ")
}

fn format_circuit_tree<S: Clone + PartialEq + Display, G: GeneratorTypes<S> + Display>(
    circuit: &Circuit<S, G>,
    indent: usize,
) -> String {
    let pad = "  ".repeat(indent);
    let io = circuit
        .io_types()
        .map(|(i, o)| format!(" [in: {}, out: {}]", format_sorts(&i), format_sorts(&o)));
    let io = io.unwrap_or_else(|| " [in: ?, out: ?]".to_string());

    match circuit {
        Circuit::Id(sort) => format!("{}Id({}){}", pad, sort, io),
        Circuit::IdOne => format!("{}IdOne{}", pad, io),
        Circuit::Generator(gen) => format!("{}Gen({}){}", pad, gen, io),
        Circuit::Swap { left, right } => format!("{}Swap({}, {}){}", pad, left, right, io),
        Circuit::Copy(sort) => format!("{}Copy({}){}", pad, sort, io),
        Circuit::Discard(sort) => format!("{}Discard({}){}", pad, sort, io),
        Circuit::Join(sort) => format!("{}Join({}){}", pad, sort, io),
        Circuit::Seq(left, right) => format!(
            "{}Seq{}\n{}\n{}",
            pad,
            io,
            format_circuit_tree(left, indent + 1),
            format_circuit_tree(right, indent + 1)
        ),
        Circuit::Product(left, right) => format!(
            "{}Product{}\n{}\n{}",
            pad,
            io,
            format_circuit_tree(left, indent + 1),
            format_circuit_tree(right, indent + 1)
        ),
    }
}

fn format_tape_tree<S: Clone + PartialEq + Display, G: GeneratorTypes<S> + Display>(
    tape: &Tape<S, G>,
    indent: usize,
) -> String {
    let pad = "  ".repeat(indent);
    let io = tape.io_types().map(|(i, o)| {
        format!(
            " [in: {}, out: {}]",
            format_monomials(&i),
            format_monomials(&o)
        )
    });
    let io = io.unwrap_or_else(|| " [in: ?, out: ?]".to_string());

    match tape {
        Tape::Id(mono) => format!("{}Id({}){}", pad, mono, io),
        Tape::IdZero => format!("{}IdZero{}", pad, io),
        Tape::EmbedCircuit(circuit) => format!(
            "{}EmbedCircuit{}\n{}",
            pad,
            io,
            format_circuit_tree(circuit, indent + 1)
        ),
        Tape::Swap { left, right } => {
            format!("{}Swap({}, {}){}", pad, left, right, io)
        }
        Tape::Trace { around, tape } => format!(
            "{}Trace({}){}\n{}",
            pad,
            around,
            io,
            format_tape_tree(tape, indent + 1)
        ),
        Tape::Discard(mono) => format!("{}Discard({}){}", pad, mono, io),
        Tape::Split(mono) => format!("{}Split({}){}", pad, mono, io),
        Tape::Create(mono) => format!("{}Create({}){}", pad, mono, io),
        Tape::Merge(mono) => format!("{}Merge({}){}", pad, mono, io),
        Tape::Seq(left, right) => format!(
            "{}Seq{}\n{}\n{}",
            pad,
            io,
            format_tape_tree(left, indent + 1),
            format_tape_tree(right, indent + 1)
        ),
        Tape::Product(left, right) => format!(
            "{}Product{}\n{}\n{}",
            pad,
            io,
            format_tape_tree(left, indent + 1),
            format_tape_tree(right, indent + 1)
        ),
        Tape::Sum(left, right) => format!(
            "{}Sum{}\n{}\n{}",
            pad,
            io,
            format_tape_tree(left, indent + 1),
            format_tape_tree(right, indent + 1)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression_circuit::ExprGenerator;
    use crate::types::TypeExpr;

    fn atom(name: &str) -> Monomial<TypeExpr> {
        Monomial::atom(TypeExpr::Named(name.to_string()))
    }

    #[test]
    fn swap_sum_blocks_reorders_atoms() {
        let u_sort = TypeExpr::Named("U".to_string());
        let u2_sort = TypeExpr::Named("U2".to_string());
        let v_sort = TypeExpr::Named("V".to_string());
        let w_sort = TypeExpr::Named("W".to_string());

        let u = Monomial::atom(u_sort.clone());
        let u2 = Monomial::atom(u2_sort.clone());
        let v = Monomial::atom(v_sort.clone());
        let w = Monomial::atom(w_sort.clone());

        let left = Polynomial::sum(
            Polynomial::monomial(Monomial::product(u.clone(), v.clone())),
            Polynomial::monomial(Monomial::product(u2.clone(), v.clone())),
        );
        let right = Polynomial::monomial(Monomial::product(u.clone(), w.clone()));

        let tape: Tape<TypeExpr, ExprGenerator> = swap_sum_blocks(&left, &right);
        let (inputs, outputs) = tape.io_types().expect("expected io types");

        let expected_inputs = vec![Monomial::from_sorts(vec![
            u_sort.clone(),
            v_sort.clone(),
            u2_sort.clone(),
            v_sort.clone(),
            u_sort.clone(),
            w_sort.clone(),
        ])];
        let expected_outputs = vec![Monomial::from_sorts(vec![
            u_sort.clone(),
            w_sort,
            u_sort.clone(),
            v_sort.clone(),
            u2_sort,
            v_sort,
        ])];

        assert_eq!(inputs, expected_inputs);
        assert_eq!(outputs, expected_outputs);
    }
}

impl<
        S: Clone + PartialEq + Debug + Display,
        G: GeneratorShape + GeneratorTypes<S> + Clone + Display,
    > Tape<S, G>
{
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
            Tape::Trace { around, tape } => {
                let mut graph = tape.to_hypergraph(fresh_sort);
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
