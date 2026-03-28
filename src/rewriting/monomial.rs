use open_hypergraphs::array::vec::{VecArray, VecKind};
use open_hypergraphs::category::{Arrow, Monoidal};
use open_hypergraphs::semifinite::SemifiniteFunction;
use open_hypergraphs::strict::open_hypergraph::SmcRewriteRule;
use open_hypergraphs::strict::vec::{FiniteFunction, OpenHypergraph};

use crate::tape_language::monomial_tape::{MonomialHyperNode, MonomialTapeEdge, TensorKind};
use crate::tape_language::Monomial;

pub type MonomialRewriteRule<S, G> =
    SmcRewriteRule<VecKind, MonomialHyperNode<S>, MonomialTapeEdge<S, G>>;

pub fn from_add_to_mul_then_from_mul_to_add_rule<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> MonomialRewriteRule<S, G> {
    let lhs = compose_strict(
        &from_add_to_mul_graph(context),
        &from_mul_to_add_graph(context),
    );
    let rhs = additive_identity_graph(context);
    SmcRewriteRule::new(lhs, rhs).expect("monomial conversion rule should be valid")
}

pub fn from_mul_to_add_then_from_add_to_mul_rule<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> MonomialRewriteRule<S, G> {
    let lhs = compose_strict(
        &from_mul_to_add_graph(context),
        &from_add_to_mul_graph(context),
    );
    let rhs = multiplicative_identity_graph(context);
    SmcRewriteRule::new(lhs, rhs).expect("monomial conversion rule should be valid")
}

pub fn additive_split_then_merge_rule<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> MonomialRewriteRule<S, G> {
    let lhs = compose_strict(&additive_split_graph(context), &additive_merge_graph(context));
    let rhs = additive_identity_graph(context);
    SmcRewriteRule::new(lhs, rhs).expect("monomial split/merge rule should be valid")
}

fn additive_identity_graph<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    OpenHypergraph::identity(object_labels(context, TensorKind::Additive))
}

fn multiplicative_identity_graph<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    OpenHypergraph::identity(object_labels(context, TensorKind::Multiplicative))
}

fn from_add_to_mul_graph<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    tensor_many(
        atomic_contexts(context)
            .into_iter()
            .map(|atom| {
                OpenHypergraph::singleton(
                    MonomialTapeEdge::FromAddToMul(atom.clone()),
                    SemifiniteFunction(VecArray(vec![MonomialHyperNode::new(
                        TensorKind::Additive,
                        atom.clone(),
                    )])),
                    SemifiniteFunction(VecArray(vec![MonomialHyperNode::new(
                        TensorKind::Multiplicative,
                        atom,
                    )])),
                )
            })
            .collect(),
    )
}

fn from_mul_to_add_graph<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    tensor_many(
        atomic_contexts(context)
            .into_iter()
            .map(|atom| {
                OpenHypergraph::singleton(
                    MonomialTapeEdge::FromMulToAdd(atom.clone()),
                    SemifiniteFunction(VecArray(vec![MonomialHyperNode::new(
                        TensorKind::Multiplicative,
                        atom.clone(),
                    )])),
                    SemifiniteFunction(VecArray(vec![MonomialHyperNode::new(
                        TensorKind::Additive,
                        atom,
                    )])),
                )
            })
            .collect(),
    )
}

fn additive_split_graph<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    let width = atomic_contexts(context).len();
    let object = object_labels(context, TensorKind::Additive);
    let s = FiniteFunction::new(VecArray((0..width).collect()), width).unwrap();
    let t = FiniteFunction::new(
        VecArray((0..width).flat_map(|idx| [idx, idx]).collect()),
        width,
    )
    .unwrap();
    OpenHypergraph::spider(s, t, object).expect("additive split should be a valid spider")
}

fn additive_merge_graph<S: Clone + PartialEq, G: Clone>(
    context: &Monomial<S>,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    let width = atomic_contexts(context).len();
    let object = object_labels(context, TensorKind::Additive);
    let s = FiniteFunction::new(
        VecArray((0..width).flat_map(|idx| [idx, idx]).collect()),
        width,
    )
    .unwrap();
    let t = FiniteFunction::new(VecArray((0..width).collect()), width).unwrap();
    OpenHypergraph::spider(s, t, object).expect("additive merge should be a valid spider")
}

fn object_labels<S: Clone>(
    context: &Monomial<S>,
    tensor_kind: TensorKind,
) -> SemifiniteFunction<VecKind, MonomialHyperNode<S>> {
    SemifiniteFunction(VecArray(
        atomic_contexts(context)
            .into_iter()
            .map(|atom| MonomialHyperNode::new(tensor_kind, atom))
            .collect(),
    ))
}

fn atomic_contexts<S: Clone>(context: &Monomial<S>) -> Vec<Monomial<S>> {
    match context {
        Monomial::One => Vec::new(),
        Monomial::Atom(sort) => vec![Monomial::atom(sort.clone())],
        Monomial::Product(left, right) => {
            let mut atoms = atomic_contexts(left);
            atoms.extend(atomic_contexts(right));
            atoms
        }
    }
}

fn tensor_many<S: Clone + PartialEq, G: Clone>(
    graphs: Vec<OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>>,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    graphs
        .into_iter()
        .reduce(|acc, graph| acc.tensor(&graph))
        .unwrap_or_else(|| OpenHypergraph::identity(SemifiniteFunction(VecArray(vec![]))))
}

fn compose_strict<S: Clone + PartialEq, G: Clone>(
    lhs: &OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>,
    rhs: &OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>,
) -> OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>> {
    Arrow::compose(lhs, rhs).expect("rewrite rule boundary types should compose")
}

#[cfg(test)]
mod tests {
    use super::{
        additive_split_then_merge_rule, from_add_to_mul_then_from_mul_to_add_rule,
        from_mul_to_add_then_from_add_to_mul_rule,
    };
    use crate::tape_language::Monomial;

    #[test]
    fn constructs_conversion_rules() {
        let context = Monomial::from_sorts(vec!["x", "y"]);
        let _rule0 = from_add_to_mul_then_from_mul_to_add_rule::<_, ()>(&context);
        let _rule1 = from_mul_to_add_then_from_add_to_mul_rule::<_, ()>(&context);
    }

    #[test]
    fn constructs_additive_split_merge_rule() {
        let context = Monomial::from_sorts(vec!["x", "y"]);
        let _rule = additive_split_then_merge_rule::<_, ()>(&context);
    }
}
