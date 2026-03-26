use open_hypergraphs::lax::OpenHypergraph;
use std::fmt;

use super::tape::monomial_atoms;
use super::{GeneratorShape, GeneratorTypes, Monomial, Tape, TapeEdge};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonomialTapeError {
    NonMonomialArity { inputs: usize, outputs: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MonomialTapeEdge<S: Clone, G> {
    Multiset(Vec<OpenHypergraph<Monomial<S>, TapeEdge<S, G>>>),
}

impl<S: Clone, G> MonomialTapeEdge<S, G> {
    pub fn multiplicity(&self) -> usize {
        match self {
            MonomialTapeEdge::Multiset(items) => items.len(),
        }
    }
}

impl<S: fmt::Display + Clone, G: fmt::Display> fmt::Display for MonomialTapeEdge<S, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonomialTapeEdge::Multiset(items) => write!(f, "Multiset({})", items.len()),
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
    ) -> OpenHypergraph<Monomial<S>, MonomialTapeEdge<S, G>> {
        let (inputs, outputs) = self
            .io_monomials()
            .unwrap_or_else(|_| panic!("monomial tape must have monomial interfaces"));
        let sources = monomial_atoms(&inputs);
        let targets = monomial_atoms(&outputs);
        let branches = collect_convolution_branches(&self.tape)
            .into_iter()
            .map(|branch| branch.to_hypergraph(fresh_sort))
            .collect();
        OpenHypergraph::singleton(MonomialTapeEdge::Multiset(branches), sources, targets)
    }
}

fn collect_convolution_branches<S: Clone + PartialEq, G: Clone>(
    tape: &Tape<S, G>,
) -> Vec<Tape<S, G>> {
    if let Some((left, right)) = convolution_branches(tape) {
        let mut branches = collect_convolution_branches(left);
        branches.extend(collect_convolution_branches(right));
        branches
    } else {
        vec![tape.clone()]
    }
}

fn convolution_branches<'a, S: Clone + PartialEq, G>(
    tape: &'a Tape<S, G>,
) -> Option<(&'a Tape<S, G>, &'a Tape<S, G>)> {
    let Tape::Seq(split, tail) = tape else {
        return None;
    };
    let Tape::Split(split_mono) = split.as_ref() else {
        return None;
    };
    let Tape::Seq(sum, join) = tail.as_ref() else {
        return None;
    };
    let Tape::Merge(merge_mono) = join.as_ref() else {
        return None;
    };
    if split_mono != merge_mono {
        return None;
    }
    let Tape::Sum(left, right) = sum.as_ref() else {
        return None;
    };
    Some((left, right))
}
