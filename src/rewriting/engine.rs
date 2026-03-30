use open_hypergraphs::array::vec::VecArray;
use open_hypergraphs::category::Arrow;
use open_hypergraphs::strict::open_hypergraph::{
    apply_frobenius_rewrite, apply_smc_rewrite, FrobeniusRewriteMatch, FrobeniusRewriteRule,
    FrobeniusRewriteApplicationDebug, SmcRewriteMatch, SmcRewriteMatchError, SmcRewriteRule,
};
use open_hypergraphs::array::vec::VecKind;
use open_hypergraphs::strict::vec::{FiniteFunction, OpenHypergraph};

use crate::matching::strict_open_hypergraph::{
    enumerate_subgraph_matches, enumerate_subgraph_matches_with_options, MatchOptions,
    StrictOpenHypergraphMatch,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedRuleMatch {
    pub rule_index: usize,
    pub graph_match: StrictOpenHypergraphMatch,
}

#[derive(Debug, Clone)]
pub struct RewriteStep<O, A> {
    pub rewritten_graph: OpenHypergraph<O, A>,
    pub applied_rule_index: usize,
    pub applied_match: StrictOpenHypergraphMatch,
}

#[derive(Debug, Clone)]
pub struct RewriteNormalForm<O, A> {
    pub graph: OpenHypergraph<O, A>,
    pub steps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteError {
    InvalidMatchWitness,
    InvalidRewriteApplication,
    StepLimitExceeded { max_steps: usize },
}

#[derive(Debug, Clone)]
pub struct RewriteCandidateDebug {
    pub rule_index: usize,
    pub candidate_index: usize,
    pub node_map: Vec<usize>,
    pub edge_map: Vec<usize>,
    pub accepted: bool,
    pub reason: String,
}

pub fn find_rule_matches<O, A>(
    host: &OpenHypergraph<O, A>,
    rules: &[SmcRewriteRule<VecKind, O, A>],
) -> Vec<IndexedRuleMatch>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let mut out = Vec::new();
    for (rule_index, rule) in rules.iter().enumerate() {
        for graph_match in enumerate_subgraph_matches(rule.lhs(), host) {
            if smc_rewrite_match(rule, host, &graph_match).is_some() {
                out.push(IndexedRuleMatch {
                    rule_index,
                    graph_match,
                });
            }
        }
    }
    out
}

pub fn rewrite_step<O, A>(
    host: &OpenHypergraph<O, A>,
    rules: &[SmcRewriteRule<VecKind, O, A>],
) -> Result<Option<RewriteStep<O, A>>, RewriteError>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    for (rule_index, rule) in rules.iter().enumerate() {
        for graph_match in enumerate_subgraph_matches(rule.lhs(), host) {
            let Some(rewrite_match) = smc_rewrite_match(rule, host, &graph_match) else {
                continue;
            };
            let rewritten_graph =
                apply_smc_rewrite(&rewrite_match).ok_or(RewriteError::InvalidRewriteApplication)?;
            return Ok(Some(RewriteStep {
                rewritten_graph,
                applied_rule_index: rule_index,
                applied_match: graph_match,
            }));
        }
    }
    Ok(None)
}

pub fn rewrite_to_normal_form<O, A>(
    mut graph: OpenHypergraph<O, A>,
    rules: &[SmcRewriteRule<VecKind, O, A>],
    max_steps: usize,
) -> Result<RewriteNormalForm<O, A>, RewriteError>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let mut steps = 0;
    loop {
        if steps >= max_steps {
            return Err(RewriteError::StepLimitExceeded { max_steps });
        }

        match rewrite_step(&graph, rules)? {
            Some(step) => {
                graph = step.rewritten_graph;
                steps += 1;
            }
            None => return Ok(RewriteNormalForm { graph, steps }),
        }
    }
}

pub fn frobenius_rewrite_to_normal_form<O, A>(
    mut graph: OpenHypergraph<O, A>,
    rules: &[FrobeniusRewriteRule<O, A>],
    max_steps: usize,
) -> Result<RewriteNormalForm<O, A>, RewriteError>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let mut steps = 0;
    loop {
        if steps >= max_steps {
            return Err(RewriteError::StepLimitExceeded { max_steps });
        }

        match frobenius_rewrite_step(&graph, rules)? {
            Some(step) => {
                graph = step.rewritten_graph;
                steps += 1;
            }
            None => return Ok(RewriteNormalForm { graph, steps }),
        }
    }
}

pub fn frobenius_rewrite_step<O, A>(
    host: &OpenHypergraph<O, A>,
    rules: &[FrobeniusRewriteRule<O, A>],
) -> Result<Option<RewriteStep<O, A>>, RewriteError>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let host_complexity = graph_complexity(host);
    for (rule_index, rule) in rules.iter().enumerate() {
        for graph_match in enumerate_subgraph_matches_with_options(
            rule.lhs(),
            host,
            MatchOptions {
                preserve_host_boundary: false,
                monic: false,
            },
        ) {
            let Some(rewrite_match) = frobenius_rewrite_match(rule, host, &graph_match) else {
                continue;
            };
            let Some(rewritten_graph) = apply_frobenius_rewrite(&rewrite_match)
                .into_iter()
                .filter(|graph| graph_complexity(graph) < host_complexity)
                .min_by_key(graph_complexity)
            else {
                continue;
            };
            return Ok(Some(RewriteStep {
                rewritten_graph,
                applied_rule_index: rule_index,
                applied_match: graph_match,
            }));
        }
    }
    Ok(None)
}

pub fn debug_smc_rule_candidates<O, A>(
    host: &OpenHypergraph<O, A>,
    rules: &[SmcRewriteRule<VecKind, O, A>],
) -> Vec<RewriteCandidateDebug>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let mut out = Vec::new();
    for (rule_index, rule) in rules.iter().enumerate() {
        for (candidate_index, graph_match) in enumerate_subgraph_matches(rule.lhs(), host)
            .into_iter()
            .enumerate()
        {
            let accepted = smc_rewrite_match(rule, host, &graph_match).is_some();
            let reason = smc_rewrite_match_reason(rule, host, &graph_match);
            out.push(RewriteCandidateDebug {
                rule_index,
                candidate_index,
                node_map: graph_match.node_map.clone(),
                edge_map: graph_match.edge_map.clone(),
                accepted,
                reason,
            });
        }
    }
    out
}

pub fn debug_frobenius_rule_candidates<O, A>(
    host: &OpenHypergraph<O, A>,
    rules: &[FrobeniusRewriteRule<O, A>],
) -> Vec<RewriteCandidateDebug>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let mut out = Vec::new();
    let host_complexity = graph_complexity(host);
    for (rule_index, rule) in rules.iter().enumerate() {
        for (candidate_index, graph_match) in enumerate_subgraph_matches_with_options(
            rule.lhs(),
            host,
            MatchOptions {
                preserve_host_boundary: false,
                monic: false,
            },
        )
            .into_iter()
            .enumerate()
        {
            let accepted = frobenius_rewrite_match(rule, host, &graph_match).is_some();
            let reason = if let Some(rewrite_match) = frobenius_rewrite_match(rule, host, &graph_match) {
                match open_hypergraphs::strict::open_hypergraph::apply_frobenius_rewrite_debug(&rewrite_match) {
                    FrobeniusRewriteApplicationDebug::IdentificationConditionFailed => {
                        "accepted apply_failed=identification_condition".to_string()
                    }
                    FrobeniusRewriteApplicationDebug::DanglingConditionFailed => {
                        "accepted apply_failed=dangling_condition".to_string()
                    }
                    FrobeniusRewriteApplicationDebug::NoPushoutResults => {
                        "accepted apply_failed=no_pushout_results".to_string()
                    }
                    FrobeniusRewriteApplicationDebug::ProducedResults { total_outputs } => {
                        let outputs = apply_frobenius_rewrite(&rewrite_match);
                        let simplifying = outputs
                            .iter()
                            .filter(|graph| graph_complexity(graph) < host_complexity)
                            .count();
                        format!(
                            "accepted outputs={} simplifying_outputs={}",
                            total_outputs,
                            simplifying
                        )
                    }
                }
            } else {
                "validation_failed".to_string()
            };
            out.push(RewriteCandidateDebug {
                rule_index,
                candidate_index,
                node_map: graph_match.node_map.clone(),
                edge_map: graph_match.edge_map.clone(),
                accepted,
                reason,
            });
        }
    }
    out
}

fn graph_complexity<O: Clone, A: Clone>(
    graph: &OpenHypergraph<O, A>,
) -> (usize, usize, usize, usize) {
    (
        graph.h.x.len(),
        graph.h.w.len(),
        graph.s.source(),
        graph.t.source(),
    )
}

fn smc_rewrite_match<'a, O, A>(
    rule: &'a SmcRewriteRule<VecKind, O, A>,
    host: &'a OpenHypergraph<O, A>,
    graph_match: &StrictOpenHypergraphMatch,
) -> Option<SmcRewriteMatch<'a, VecKind, O, A>>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let w = FiniteFunction::new(VecArray(graph_match.node_map.clone()), host.h.w.len())?;
    let x = FiniteFunction::new(VecArray(graph_match.edge_map.clone()), host.h.x.len())?;
    SmcRewriteMatch::new(rule, host, w, x)
}

fn smc_rewrite_match_reason<O, A>(
    rule: &SmcRewriteRule<VecKind, O, A>,
    host: &OpenHypergraph<O, A>,
    graph_match: &StrictOpenHypergraphMatch,
) -> String
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let Some(w) = FiniteFunction::new(VecArray(graph_match.node_map.clone()), host.h.w.len()) else {
        return "invalid_w_map".to_string();
    };
    let Some(x) = FiniteFunction::new(VecArray(graph_match.edge_map.clone()), host.h.x.len()) else {
        return "invalid_x_map".to_string();
    };
    match SmcRewriteMatch::validate(rule, host, &w, &x) {
        Ok(()) => "accepted".to_string(),
        Err(err) => format_smc_match_error(err),
    }
}

fn format_smc_match_error(err: SmcRewriteMatchError) -> String {
    match err {
        SmcRewriteMatchError::HostNotMonogamous => "host_not_monogamous".to_string(),
        SmcRewriteMatchError::HostNotAcyclic => "host_not_acyclic".to_string(),
        SmcRewriteMatchError::InvalidHypergraphMorphism(reason) => match reason {
            open_hypergraphs::strict::hypergraph::arrow::InvalidHypergraphArrow::TypeMismatchW => {
                "invalid_hypergraph_morphism:type_mismatch_w".to_string()
            }
            open_hypergraphs::strict::hypergraph::arrow::InvalidHypergraphArrow::TypeMismatchX => {
                "invalid_hypergraph_morphism:type_mismatch_x".to_string()
            }
            open_hypergraphs::strict::hypergraph::arrow::InvalidHypergraphArrow::NotNaturalW => {
                "invalid_hypergraph_morphism:not_natural_w".to_string()
            }
            open_hypergraphs::strict::hypergraph::arrow::InvalidHypergraphArrow::NotNaturalX => {
                "invalid_hypergraph_morphism:not_natural_x".to_string()
            }
            open_hypergraphs::strict::hypergraph::arrow::InvalidHypergraphArrow::NotNaturalS => {
                "invalid_hypergraph_morphism:not_natural_s".to_string()
            }
            open_hypergraphs::strict::hypergraph::arrow::InvalidHypergraphArrow::NotNaturalT => {
                "invalid_hypergraph_morphism:not_natural_t".to_string()
            }
        },
        SmcRewriteMatchError::NonInjectiveW => "non_injective_w".to_string(),
        SmcRewriteMatchError::NonInjectiveX => "non_injective_x".to_string(),
        SmcRewriteMatchError::NonConvexSubgraph => "non_convex_subgraph".to_string(),
    }
}

fn frobenius_rewrite_match<'a, O, A>(
    rule: &'a FrobeniusRewriteRule<O, A>,
    host: &'a OpenHypergraph<O, A>,
    graph_match: &StrictOpenHypergraphMatch,
) -> Option<FrobeniusRewriteMatch<'a, O, A>>
where
    O: Clone + PartialEq,
    A: Clone + PartialEq,
{
    let w = FiniteFunction::new(VecArray(graph_match.node_map.clone()), host.h.w.len())?;
    let x = FiniteFunction::new(VecArray(graph_match.edge_map.clone()), host.h.x.len())?;
    FrobeniusRewriteMatch::new(rule, host, w, x)
}

#[cfg(test)]
mod tests {
    use super::{find_rule_matches, rewrite_step, rewrite_to_normal_form};
    use crate::tape_language::monomial_tape::{
        MonomialHyperNode, MonomialTapeEdge, TensorKind,
    };
    use crate::tape_language::Monomial;
    use open_hypergraphs::array::vec::VecArray;
    use open_hypergraphs::category::Arrow;
    use open_hypergraphs::semifinite::SemifiniteFunction;
    use open_hypergraphs::strict::open_hypergraph::SmcRewriteRule;
    use open_hypergraphs::strict::vec::OpenHypergraph;

    fn add_node(sort: &str) -> MonomialHyperNode<&str> {
        MonomialHyperNode::new(TensorKind::Additive, Monomial::atom(sort))
    }

    fn mul_node(sort: &str) -> MonomialHyperNode<&str> {
        MonomialHyperNode::new(TensorKind::Multiplicative, Monomial::atom(sort))
    }

    fn add_to_mul_graph() -> OpenHypergraph<MonomialHyperNode<&'static str>, MonomialTapeEdge<&'static str, ()>> {
        OpenHypergraph::singleton(
            MonomialTapeEdge::FromAddToMul(Monomial::atom("x")),
            SemifiniteFunction(VecArray(vec![add_node("x")])),
            SemifiniteFunction(VecArray(vec![mul_node("x")])),
        )
    }

    fn mul_to_add_graph() -> OpenHypergraph<MonomialHyperNode<&'static str>, MonomialTapeEdge<&'static str, ()>> {
        OpenHypergraph::singleton(
            MonomialTapeEdge::FromMulToAdd(Monomial::atom("x")),
            SemifiniteFunction(VecArray(vec![mul_node("x")])),
            SemifiniteFunction(VecArray(vec![add_node("x")])),
        )
    }

    fn add_to_mul_then_mul_to_add_rule(
    ) -> SmcRewriteRule<
        open_hypergraphs::array::vec::VecKind,
        MonomialHyperNode<&'static str>,
        MonomialTapeEdge<&'static str, ()>,
    > {
        let lhs = Arrow::compose(&add_to_mul_graph(), &mul_to_add_graph()).unwrap();
        let rhs = OpenHypergraph::identity(SemifiniteFunction(VecArray(vec![add_node("x")])));
        SmcRewriteRule::new(lhs, rhs).unwrap()
    }

    #[test]
    fn finds_matches_for_conversion_rule() {
        let host = Arrow::compose(&add_to_mul_graph(), &mul_to_add_graph()).unwrap();
        let rule = add_to_mul_then_mul_to_add_rule();
        let matches = find_rule_matches(&host, &[rule]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].rule_index, 0);
    }

    #[test]
    fn rewrites_one_step() {
        let host = Arrow::compose(&add_to_mul_graph(), &mul_to_add_graph()).unwrap();
        let rule = add_to_mul_then_mul_to_add_rule();
        let step = rewrite_step(&host, &[rule]).unwrap().unwrap();
        assert_eq!(step.applied_rule_index, 0);
        assert_eq!(step.rewritten_graph.h.x.len(), 0);
        assert_eq!(step.rewritten_graph.h.w.len(), 1);
    }

    #[test]
    fn rewrites_to_normal_form() {
        let host = Arrow::compose(
            &Arrow::compose(&add_to_mul_graph(), &mul_to_add_graph()).unwrap(),
            &Arrow::compose(&add_to_mul_graph(), &mul_to_add_graph()).unwrap(),
        )
        .unwrap();

        let rules = vec![add_to_mul_then_mul_to_add_rule()];
        let normal = rewrite_to_normal_form(host, &rules, 8).unwrap();
        assert_eq!(normal.steps, 2);
        assert_eq!(normal.graph.h.x.len(), 0);
    }
}
