use open_hypergraphs::array::vec::{VecArray, VecKind};
use open_hypergraphs::category::{Arrow, Monoidal};
use open_hypergraphs::semifinite::SemifiniteFunction;
use open_hypergraphs::strict::open_hypergraph::FrobeniusRewriteRule;
use open_hypergraphs::strict::vec::{FiniteFunction, OpenHypergraph};

use crate::rewriting::engine::{
    debug_frobenius_rule_candidates, frobenius_rewrite_step, RewriteError, RewriteNormalForm,
};
use crate::tape_language::monomial_tape::{MonomialHyperNode, MonomialTapeEdge, TensorKind};
use crate::tape_language::Monomial;

pub type MonomialFrobeniusRewriteRule<S, G> =
    FrobeniusRewriteRule<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>;

#[derive(Debug, Clone)]
pub struct MonomialRewriteTraceStep<S: Clone, G> {
    pub phase: String,
    pub graph: OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>,
}

#[derive(Debug, Clone)]
pub struct MonomialRewriteTrace<S: Clone, G> {
    pub normal_form: RewriteNormalForm<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>,
    pub steps: Vec<MonomialRewriteTraceStep<S, G>>,
    pub debug_log: String,
}

pub fn frobenius_rules_for_graph<S: Clone + PartialEq, G: Clone + PartialEq>(
    graph: &OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>,
) -> Vec<MonomialFrobeniusRewriteRule<S, G>> {
    let mut contexts = Vec::new();
    for node in graph.h.w.0.iter() {
        if !contexts.iter().any(|context| context == &node.context) {
            contexts.push(node.context.clone());
        }
    }

    let mut rules = Vec::new();
    for context in contexts {
        if let Some(rule) = from_add_to_mul_then_from_mul_to_add_rule(&context) {
            rules.push(rule);
        }
        if let Some(rule) = from_mul_to_add_then_from_add_to_mul_rule(&context) {
            rules.push(rule);
        }
        if let Some(rule) = additive_split_then_merge_rule(&context) {
            rules.push(rule);
        }
    }
    rules
}

pub fn rewrite_graph_to_normal_form<S: Clone + PartialEq, G: Clone + PartialEq>(
    graph: OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>,
    max_steps: usize,
) -> Result<RewriteNormalForm<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>, RewriteError> {
    Ok(rewrite_graph_with_trace(graph, max_steps)?.normal_form)
}

pub fn rewrite_graph_with_trace<S: Clone + PartialEq, G: Clone + PartialEq>(
    mut graph: OpenHypergraph<MonomialHyperNode<S>, MonomialTapeEdge<S, G>>,
    max_steps: usize,
) -> Result<MonomialRewriteTrace<S, G>, RewriteError> {
    let mut steps = 0;
    let mut trace = Vec::new();
    let mut debug_log = String::new();
    loop {
        if steps >= max_steps {
            return Err(RewriteError::StepLimitExceeded { max_steps });
        }

        append_debug_section(
            &mut debug_log,
            steps,
            "frobenius",
            graph.h.w.len(),
            graph.h.x.len(),
            debug_frobenius_rule_candidates(&graph, &frobenius_rules_for_graph(&graph)),
        );
        let mut frobenius_progress = 0;
        let frobenius_rules = frobenius_rules_for_graph(&graph);
        loop {
            let Some(step) = frobenius_rewrite_step(&graph, &frobenius_rules)? else {
                break;
            };
            graph = step.rewritten_graph;
            steps += 1;
            frobenius_progress += 1;
            trace.push(MonomialRewriteTraceStep {
                phase: format!("frobenius.rule_{}", step.applied_rule_index),
                graph: graph.clone(),
            });
            if steps >= max_steps {
                return Err(RewriteError::StepLimitExceeded { max_steps });
            }
        }

        if frobenius_progress == 0 {
            return Ok(MonomialRewriteTrace {
                normal_form: RewriteNormalForm { graph, steps },
                steps: trace,
                debug_log,
            });
        }
    }
}

fn append_debug_section(
    out: &mut String,
    steps_so_far: usize,
    phase: &str,
    node_count: usize,
    edge_count: usize,
    candidates: Vec<crate::rewriting::engine::RewriteCandidateDebug>,
) {
    out.push_str(&format!(
        "[phase={phase}] steps_so_far={steps_so_far} nodes={node_count} edges={edge_count}\n"
    ));
    if candidates.is_empty() {
        out.push_str("  candidates: 0\n");
        return;
    }
    for candidate in candidates {
        out.push_str(&format!(
            "  rule={} candidate={} accepted={} reason={} node_map={:?} edge_map={:?}\n",
            candidate.rule_index,
            candidate.candidate_index,
            candidate.accepted,
            candidate.reason,
            candidate.node_map,
            candidate.edge_map
        ));
    }
}

pub fn from_add_to_mul_then_from_mul_to_add_rule<S: Clone + PartialEq, G: Clone + PartialEq>(
    context: &Monomial<S>,
) -> Option<MonomialFrobeniusRewriteRule<S, G>> {
    let lhs = compose_strict(
        &from_add_to_mul_graph(context),
        &from_mul_to_add_graph(context),
    );
    let rhs = additive_identity_graph(context);
    FrobeniusRewriteRule::new(lhs, rhs)
}

pub fn from_mul_to_add_then_from_add_to_mul_rule<S: Clone + PartialEq, G: Clone + PartialEq>(
    context: &Monomial<S>,
) -> Option<MonomialFrobeniusRewriteRule<S, G>> {
    let lhs = compose_strict(
        &from_mul_to_add_graph(context),
        &from_add_to_mul_graph(context),
    );
    let rhs = multiplicative_identity_graph(context);
    FrobeniusRewriteRule::new(lhs, rhs)
}

pub fn additive_split_then_merge_rule<S: Clone + PartialEq, G: Clone + PartialEq>(
    context: &Monomial<S>,
) -> Option<MonomialFrobeniusRewriteRule<S, G>> {
    let lhs = compose_strict(&additive_split_graph(context), &additive_merge_graph(context));
    let rhs = additive_identity_graph(context);
    FrobeniusRewriteRule::new(lhs, rhs)
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
        additive_split_then_merge_rule, frobenius_rules_for_graph,
        from_add_to_mul_then_from_mul_to_add_rule, from_mul_to_add_then_from_add_to_mul_rule,
        rewrite_graph_to_normal_form,
    };
    use crate::tape_language::Monomial;
    use crate::tape_language::monomial_tape::{MonomialHyperNode, MonomialTapeEdge, TensorKind};
    use open_hypergraphs::array::vec::VecArray;
    use open_hypergraphs::category::Arrow;
    use open_hypergraphs::semifinite::SemifiniteFunction;
    use open_hypergraphs::strict::vec::OpenHypergraph;

    fn add_node(sort: &str) -> MonomialHyperNode<&str> {
        MonomialHyperNode::new(TensorKind::Additive, Monomial::atom(sort))
    }

    fn mul_node(sort: &str) -> MonomialHyperNode<&str> {
        MonomialHyperNode::new(TensorKind::Multiplicative, Monomial::atom(sort))
    }

    #[test]
    fn constructs_conversion_rules() {
        let context = Monomial::from_sorts(vec!["x", "y"]);
        let _rule0 = from_add_to_mul_then_from_mul_to_add_rule::<_, ()>(&context).unwrap();
        let _rule1 = from_mul_to_add_then_from_add_to_mul_rule::<_, ()>(&context).unwrap();
    }

    #[test]
    fn constructs_additive_split_merge_rule() {
        let context = Monomial::from_sorts(vec!["x", "y"]);
        let _rule = additive_split_then_merge_rule::<_, ()>(&context);
        assert!(_rule.is_some());
    }

    #[test]
    fn derives_frobenius_rules_from_graph() {
        let graph: OpenHypergraph<MonomialHyperNode<&str>, MonomialTapeEdge<&str, ()>> =
            OpenHypergraph::singleton(
                MonomialTapeEdge::FromAddToMul(Monomial::atom("x")),
                SemifiniteFunction(VecArray(vec![add_node("x")])),
                SemifiniteFunction(VecArray(vec![mul_node("x")])),
            );
        let rules = frobenius_rules_for_graph(&graph);
        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn rewrites_graph_to_normal_form() {
        let lhs: OpenHypergraph<MonomialHyperNode<&str>, MonomialTapeEdge<&str, ()>> =
            Arrow::compose(
                &OpenHypergraph::singleton(
                    MonomialTapeEdge::FromAddToMul(Monomial::atom("x")),
                    SemifiniteFunction(VecArray(vec![add_node("x")])),
                    SemifiniteFunction(VecArray(vec![mul_node("x")])),
                ),
                &OpenHypergraph::singleton(
                    MonomialTapeEdge::FromMulToAdd(Monomial::atom("x")),
                    SemifiniteFunction(VecArray(vec![mul_node("x")])),
                    SemifiniteFunction(VecArray(vec![add_node("x")])),
                ),
            )
            .unwrap();

        let normal = rewrite_graph_to_normal_form(lhs, 4).unwrap();
        assert_eq!(normal.steps, 1);
        assert_eq!(normal.graph.h.x.len(), 0);
    }
}
