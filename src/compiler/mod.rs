use std::error::Error;
use std::fmt;

use graphviz_rust::printer::{DotPrinter, PrinterContext};
use open_hypergraphs::lax::OpenHypergraph;
use open_hypergraphs_dot::Options;

use crate::command_dot::{generate_dot_with_monomial_tape_clusters, CommandEdge};
use crate::command_tape::tape_from_command;
use crate::command_typing::{collect_constraints, CommandDerivationTree};
use crate::program_tape::{apply_substitution_to_monomial, solve_program_tape_with_subst};
use crate::rewriting::monomial::rewrite_graph_with_trace;
use crate::solver::TypeSubstitution;
use crate::tape_language::{self, MonomialTape};
use crate::types;

pub trait CompilerFrontend {
    type Error;

    fn parse(&self, source_name: &str, source: &str) -> Result<CommandDerivationTree, Self::Error>;
}

#[derive(Debug, Clone, Copy)]
pub struct CompileOptions {
    pub raw_tape: bool,
}

#[derive(Debug, Clone)]
pub struct CompileArtifacts {
    pub ir_before_rewrite_dot: String,
    pub ir_dot: String,
    pub rewrite_step_dots: Vec<(String, String)>,
    pub rewrite_debug: String,
}

#[derive(Debug)]
pub enum CompileError<E> {
    Frontend(E),
    Backend(String),
}

impl<E: fmt::Display> fmt::Display for CompileError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Frontend(err) => write!(f, "frontend parse failed: {}", err),
            CompileError::Backend(err) => write!(f, "compile failed: {}", err),
        }
    }
}

impl<E: Error + 'static> Error for CompileError<E> {}

pub fn compile<F: CompilerFrontend>(
    frontend: &F,
    source_name: &str,
    source: &str,
    options: &CompileOptions,
) -> Result<CompileArtifacts, CompileError<F::Error>>
where
    F::Error: Error + 'static,
{
    let tree = frontend
        .parse(source_name, source)
        .map_err(CompileError::Frontend)?;
    compile_program(&tree, options).map_err(CompileError::Backend)
}

fn compile_program(
    tree: &CommandDerivationTree,
    options: &CompileOptions,
) -> Result<CompileArtifacts, String> {
    let tape = tape_from_command(tree);
    let monomial_tape = MonomialTape::try_from_tape(tape.clone())
        .map_err(|err| format!("monomial tape conversion failed: {:?}", err))?;

    let mut next_id = 0usize;
    let term = tape.to_hypergraph(&mut || {
        let id = next_id;
        next_id += 1;
        types::TypeExpr::Var(types::TypeVar(id))
    });

    let subst = if options.raw_tape {
        None
    } else {
        let constraints = collect_constraints(tree);
        let (_solved, subst) = solve_program_tape_with_subst(&term, constraints.constraints());
        Some(subst)
    };

    let mut mono_next_id = 0usize;
    let monomial_graph = monomial_tape.to_hypergraph(&mut || {
        let id = mono_next_id;
        mono_next_id += 1;
        types::TypeExpr::Var(types::TypeVar(id))
    });
    let monomial_graph = if let Some(subst) = &subst {
        let monomial_graph = monomial_graph
            .map_nodes(|node| apply_substitution_to_monomial_node(&node, subst))
            .map_edges(|edge| apply_substitution_to_monomial_edge(&edge, subst));
        OpenHypergraph::from_strict(monomial_graph.to_strict())
    } else {
        monomial_graph
    };
    let ir_before_rewrite_dot = render_monomial_dot(&monomial_graph, &opts_for_monomial());
    let rewrite_trace = rewrite_monomial_graph(monomial_graph)
        .map_err(|err| format!("monomial rewrite failed: {:?}", err))?;
    let opts = opts_for_monomial();
    let rewrite_step_dots = rewrite_trace
        .steps
        .iter()
        .enumerate()
        .map(|(idx, step)| {
            (
                format!("step_{idx:03}_{}.dot", sanitize_trace_label(&step.phase)),
                render_monomial_dot(&OpenHypergraph::from_strict(step.graph.clone()), &opts),
            )
        })
        .collect();

    Ok(CompileArtifacts {
        ir_before_rewrite_dot,
        ir_dot: render_monomial_dot(&OpenHypergraph::from_strict(rewrite_trace.normal_form.graph), &opts),
        rewrite_step_dots,
        rewrite_debug: rewrite_trace.debug_log,
    })
}

fn opts_for_monomial(
) -> Options<tape_language::MonomialHyperNode<types::TypeExpr>, CommandEdge> {
    Options {
        node_label: Box::new(|node: &tape_language::MonomialHyperNode<types::TypeExpr>| {
            node.context.to_string()
        }),
        edge_label: Box::new(|edge: &CommandEdge| edge.to_string()),
        ..Options::default()
    }
}

fn rewrite_monomial_graph<E>(
    graph: OpenHypergraph<
        tape_language::MonomialHyperNode<types::TypeExpr>,
        tape_language::MonomialTapeEdge<types::TypeExpr, E>,
    >,
) -> Result<
    crate::rewriting::monomial::MonomialRewriteTrace<
        types::TypeExpr,
        E,
    >,
    crate::rewriting::engine::RewriteError,
>
where
    E: Clone + PartialEq,
{
    let strict_graph = graph.to_strict();
    rewrite_graph_with_trace(strict_graph, 1_000)
}

fn sanitize_trace_label(label: &str) -> String {
    label
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => ch,
            _ => '_',
        })
        .collect()
}

fn render_monomial_dot<E: Clone + fmt::Display>(
    graph: &OpenHypergraph<
        tape_language::MonomialHyperNode<types::TypeExpr>,
        tape_language::MonomialTapeEdge<types::TypeExpr, E>,
    >,
    opts: &Options<tape_language::MonomialHyperNode<types::TypeExpr>, CommandEdge>,
) -> String {
    let dot_graph = generate_dot_with_monomial_tape_clusters(graph, opts);
    let mut ctx = PrinterContext::default();
    dot_graph.print(&mut ctx)
}

fn apply_substitution_to_monomial_node(
    node: &tape_language::MonomialHyperNode<types::TypeExpr>,
    subst: &TypeSubstitution,
) -> tape_language::MonomialHyperNode<types::TypeExpr> {
    tape_language::MonomialHyperNode::new(
        node.tensor_kind,
        apply_substitution_to_monomial(&node.context, subst),
    )
}

fn apply_substitution_to_monomial_edge<E: Clone>(
    edge: &tape_language::MonomialTapeEdge<types::TypeExpr, E>,
    subst: &TypeSubstitution,
) -> tape_language::MonomialTapeEdge<types::TypeExpr, E> {
    match edge {
        tape_language::MonomialTapeEdge::Generator(generator) => {
            tape_language::MonomialTapeEdge::Generator(generator.clone())
        }
        tape_language::MonomialTapeEdge::FromAddToMul(mono) => {
            tape_language::MonomialTapeEdge::FromAddToMul(apply_substitution_to_monomial(
                mono, subst,
            ))
        }
        tape_language::MonomialTapeEdge::FromMulToAdd(mono) => {
            tape_language::MonomialTapeEdge::FromMulToAdd(apply_substitution_to_monomial(
                mono, subst,
            ))
        }
    }
}
