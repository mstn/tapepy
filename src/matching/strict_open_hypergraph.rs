use open_hypergraphs::array::{Array, ArrayKind, NaturalArray};

type NodeId = usize;
type EdgeId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrictOpenHypergraphMatch {
    pub node_map: Vec<NodeId>,
    pub edge_map: Vec<EdgeId>,
}

#[derive(Debug, Clone)]
struct NodeInfo {
    source_positions: Vec<NodeId>,
    target_positions: Vec<NodeId>,
    source_incidence_count: usize,
    target_incidence_count: usize,
    incident_edges: Vec<IncidentEdge>,
}

#[derive(Debug, Clone, Copy)]
struct IncidentEdge {
    edge: EdgeId,
    direction: Direction,
    port: usize,
}

#[derive(Debug, Clone)]
struct EdgeInfo {
    sources: Vec<NodeId>,
    targets: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Source,
    Target,
}

#[derive(Debug, Clone, Copy)]
enum NextPatternElement {
    Edge(EdgeId),
    Node(NodeId),
}

#[derive(Debug, Clone)]
struct SearchState {
    pattern_to_host_node: Vec<Option<NodeId>>,
    host_node_used: Vec<bool>,
    pattern_to_host_edge: Vec<Option<EdgeId>>,
    host_edge_used: Vec<bool>,
}

impl SearchState {
    fn new(pattern_node_count: usize, host_node_count: usize, pattern_edge_count: usize, host_edge_count: usize) -> Self {
        Self {
            pattern_to_host_node: vec![None; pattern_node_count],
            host_node_used: vec![false; host_node_count],
            pattern_to_host_edge: vec![None; pattern_edge_count],
            host_edge_used: vec![false; host_edge_count],
        }
    }
}

pub fn enumerate_matches<O: Eq, A: Eq>(
    pattern: &open_hypergraphs::strict::vec::OpenHypergraph<O, A>,
    host: &open_hypergraphs::strict::vec::OpenHypergraph<O, A>,
) -> Vec<StrictOpenHypergraphMatch>
where
    O: Clone,
    A: Clone,
{
    enumerate_matches_generic(pattern, host)
}

pub fn enumerate_matches_generic<K, O, A>(
    pattern: &open_hypergraphs::strict::open_hypergraph::OpenHypergraph<K, O, A>,
    host: &open_hypergraphs::strict::open_hypergraph::OpenHypergraph<K, O, A>,
) -> Vec<StrictOpenHypergraphMatch>
where
    K: ArrayKind,
    K::Type<K::I>: NaturalArray<K> + AsRef<K::Index>,
    K::Type<O>: Array<K, O>,
    K::Type<A>: Array<K, A>,
    K::I: TryFrom<usize> + TryInto<usize>,
    <K::I as TryFrom<usize>>::Error: core::fmt::Debug,
    <K::I as TryInto<usize>>::Error: core::fmt::Debug,
    O: Eq + Clone,
    A: Eq + Clone,
{
    // We normalize the strict hypergraph into ordinary Rust vectors once up front.
    // The backtracking search then runs on this cached view instead of repeatedly
    // decoding segmented incidence data from the library representation.
    let pattern_data = PreparedOpenHypergraph::new(pattern);
    let host_data = PreparedOpenHypergraph::new(host);

    if pattern_data.source_len() > host_data.source_len()
        || pattern_data.target_len() > host_data.target_len()
        || pattern_data.node_count() > host_data.node_count()
        || pattern_data.edge_count() > host_data.edge_count()
    {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let mut state = SearchState::new(
        pattern_data.node_count(),
        host_data.node_count(),
        pattern_data.edge_count(),
        host_data.edge_count(),
    );
    search(&pattern_data, &host_data, &mut state, &mut matches);
    matches
}

fn search<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &mut SearchState,
    matches: &mut Vec<StrictOpenHypergraphMatch>,
) {
    if state.pattern_to_host_edge.iter().all(|m| m.is_some())
        && state.pattern_to_host_node.iter().all(|m| m.is_some())
    {
        matches.push(StrictOpenHypergraphMatch {
            node_map: state
                .pattern_to_host_node
                .iter()
                .map(|m| m.expect("complete node map"))
                .collect(),
            edge_map: state
                .pattern_to_host_edge
                .iter()
                .map(|m| m.expect("complete edge map"))
                .collect(),
        });
        return;
    }

    // VF2/Ullmann-style branching heuristic: always expand the currently most
    // constrained unmapped pattern element first.
    let next = choose_next(pattern, host, state);
    match next {
        NextPatternElement::Edge(pattern_edge) => {
            let candidates = edge_candidates(pattern, host, state, pattern_edge);
            for host_edge in candidates {
                let mut assigned_nodes = Vec::new();
                state.pattern_to_host_edge[pattern_edge] = Some(host_edge);
                state.host_edge_used[host_edge] = true;
                if assign_nodes_for_edge(pattern, host, state, pattern_edge, host_edge, &mut assigned_nodes) {
                    search(pattern, host, state, matches);
                }
                for (pattern_node, host_node) in assigned_nodes.into_iter().rev() {
                    state.pattern_to_host_node[pattern_node] = None;
                    state.host_node_used[host_node] = false;
                }
                state.pattern_to_host_edge[pattern_edge] = None;
                state.host_edge_used[host_edge] = false;
            }
        }
        NextPatternElement::Node(pattern_node) => {
            let candidates = node_candidates(pattern, host, state, pattern_node);
            for host_node in candidates {
                state.pattern_to_host_node[pattern_node] = Some(host_node);
                state.host_node_used[host_node] = true;
                search(pattern, host, state, matches);
                state.pattern_to_host_node[pattern_node] = None;
                state.host_node_used[host_node] = false;
            }
        }
    }
}

fn choose_next<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &SearchState,
) -> NextPatternElement {
    let mut best_edge = None;
    let mut best_edge_count = usize::MAX;
    for pattern_edge in 0..pattern.edge_count() {
        if state.pattern_to_host_edge[pattern_edge].is_some() {
            continue;
        }
        let count = edge_candidates(pattern, host, state, pattern_edge).len();
        if count < best_edge_count {
            best_edge_count = count;
            best_edge = Some(pattern_edge);
        }
    }
    if let Some(edge) = best_edge {
        return NextPatternElement::Edge(edge);
    }

    let mut best_node = None;
    let mut best_node_count = usize::MAX;
    for pattern_node in 0..pattern.node_count() {
        if state.pattern_to_host_node[pattern_node].is_some() {
            continue;
        }
        let count = node_candidates(pattern, host, state, pattern_node).len();
        if count < best_node_count {
            best_node_count = count;
            best_node = Some(pattern_node);
        }
    }
    NextPatternElement::Node(best_node.expect("incomplete search state must have unmapped node or edge"))
}

fn edge_candidates<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &SearchState,
    pattern_edge: EdgeId,
) -> Vec<EdgeId> {
    let mut out = Vec::new();
    for host_edge in 0..host.edge_count() {
        if state.host_edge_used[host_edge] {
            continue;
        }
        if edge_candidate_ok(pattern, host, state, pattern_edge, host_edge) {
            out.push(host_edge);
        }
    }
    out
}

fn node_candidates<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &SearchState,
    pattern_node: NodeId,
) -> Vec<NodeId> {
    let mut out = Vec::new();
    for host_node in 0..host.node_count() {
        if state.host_node_used[host_node] {
            continue;
        }
        if node_candidate_ok(pattern, host, state, pattern_node, host_node) {
            out.push(host_node);
        }
    }
    out
}

fn edge_candidate_ok<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &SearchState,
    pattern_edge: EdgeId,
    host_edge: EdgeId,
) -> bool {
    if pattern.edge_label(pattern_edge) != host.edge_label(host_edge) {
        return false;
    }

    let p_edge = pattern.edge(pattern_edge);
    let h_edge = host.edge(host_edge);
    if p_edge.sources.len() != h_edge.sources.len() || p_edge.targets.len() != h_edge.targets.len() {
        return false;
    }

    for (&pattern_node, &host_node) in p_edge.sources.iter().zip(h_edge.sources.iter()) {
        match state.pattern_to_host_node[pattern_node] {
            Some(mapped) if mapped != host_node => return false,
            None => {
                if state.host_node_used[host_node] || !node_candidate_static_ok(pattern, host, pattern_node, host_node) {
                    return false;
                }
            }
            _ => {}
        }
    }

    for (&pattern_node, &host_node) in p_edge.targets.iter().zip(h_edge.targets.iter()) {
        match state.pattern_to_host_node[pattern_node] {
            Some(mapped) if mapped != host_node => return false,
            None => {
                if state.host_node_used[host_node] || !node_candidate_static_ok(pattern, host, pattern_node, host_node) {
                    return false;
                }
            }
            _ => {}
        }
    }

    // If the current pattern edge touches the same node several times, every
    // corresponding host port must point at the same host node. Distinct pattern
    // nodes must still map injectively.
    let mut local_pattern_to_host = std::collections::BTreeMap::new();
    let mut local_host_used = std::collections::BTreeSet::new();
    for (&pattern_node, &host_node) in p_edge.sources.iter().zip(h_edge.sources.iter()) {
        if let Some(existing) = local_pattern_to_host.insert(pattern_node, host_node) {
            if existing != host_node {
                return false;
            }
        } else if state.pattern_to_host_node[pattern_node].is_none() && !local_host_used.insert(host_node) {
            return false;
        }
    }
    for (&pattern_node, &host_node) in p_edge.targets.iter().zip(h_edge.targets.iter()) {
        if let Some(existing) = local_pattern_to_host.insert(pattern_node, host_node) {
            if existing != host_node {
                return false;
            }
        } else if state.pattern_to_host_node[pattern_node].is_none() && !local_host_used.insert(host_node) {
            return false;
        }
    }

    true
}

fn assign_nodes_for_edge<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &mut SearchState,
    pattern_edge: EdgeId,
    host_edge: EdgeId,
    assigned_nodes: &mut Vec<(NodeId, NodeId)>,
) -> bool {
    let p_edge = pattern.edge(pattern_edge);
    let h_edge = host.edge(host_edge);

    for (&pattern_node, &host_node) in p_edge.sources.iter().zip(h_edge.sources.iter()) {
        if !assign_single_node(pattern, host, state, pattern_node, host_node, assigned_nodes) {
            return false;
        }
    }
    for (&pattern_node, &host_node) in p_edge.targets.iter().zip(h_edge.targets.iter()) {
        if !assign_single_node(pattern, host, state, pattern_node, host_node, assigned_nodes) {
            return false;
        }
    }

    true
}

fn assign_single_node<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &mut SearchState,
    pattern_node: NodeId,
    host_node: NodeId,
    assigned_nodes: &mut Vec<(NodeId, NodeId)>,
) -> bool {
    if let Some(mapped) = state.pattern_to_host_node[pattern_node] {
        return mapped == host_node;
    }
    if state.host_node_used[host_node] || !node_candidate_static_ok(pattern, host, pattern_node, host_node) {
        return false;
    }

    state.pattern_to_host_node[pattern_node] = Some(host_node);
    state.host_node_used[host_node] = true;
    assigned_nodes.push((pattern_node, host_node));

    if !node_incidence_consistent(pattern, host, state, pattern_node, host_node) {
        return false;
    }

    true
}

fn node_candidate_ok<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &SearchState,
    pattern_node: NodeId,
    host_node: NodeId,
) -> bool {
    node_candidate_static_ok(pattern, host, pattern_node, host_node)
        && node_incidence_consistent(pattern, host, state, pattern_node, host_node)
}

fn node_candidate_static_ok<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    pattern_node: NodeId,
    host_node: NodeId,
) -> bool {
    if pattern.node_label(pattern_node) != host.node_label(host_node) {
        return false;
    }

    let p_node = pattern.node(pattern_node);
    let h_node = host.node(host_node);
    p_node.source_positions == h_node.source_positions
        && p_node.target_positions == h_node.target_positions
        && p_node.source_incidence_count <= h_node.source_incidence_count
        && p_node.target_incidence_count <= h_node.target_incidence_count
}

fn node_incidence_consistent<O: Eq, A: Eq>(
    pattern: &PreparedOpenHypergraph<O, A>,
    host: &PreparedOpenHypergraph<O, A>,
    state: &SearchState,
    pattern_node: NodeId,
    host_node: NodeId,
) -> bool {
    for incident in &pattern.node(pattern_node).incident_edges {
        let Some(host_edge) = state.pattern_to_host_edge[incident.edge] else {
            continue;
        };
        let host_edge_info = host.edge(host_edge);
        let actual_host_node = match incident.direction {
            Direction::Source => host_edge_info.sources[incident.port],
            Direction::Target => host_edge_info.targets[incident.port],
        };
        if actual_host_node != host_node {
            return false;
        }
    }
    true
}

/// A search-oriented cache extracted from the strict library representation.
///
/// The library stores incidence in segmented arrays; that is the right canonical
/// format for composition and rewriting, but it is not convenient for backtracking.
/// We therefore decode it once into ordinary vectors and keep the search core
/// independent from the concrete array backend.
struct PreparedOpenHypergraph<O, A> {
    node_labels: Vec<O>,
    edge_labels: Vec<A>,
    source_len: usize,
    target_len: usize,
    nodes: Vec<NodeInfo>,
    edges: Vec<EdgeInfo>,
}

impl<O, A> PreparedOpenHypergraph<O, A> {
    fn new<K>(graph: &open_hypergraphs::strict::open_hypergraph::OpenHypergraph<K, O, A>) -> Self
    where
        K: ArrayKind,
        K::Type<K::I>: NaturalArray<K> + AsRef<K::Index>,
        K::Type<O>: Array<K, O>,
        K::Type<A>: Array<K, A>,
        K::I: TryFrom<usize> + TryInto<usize>,
        <K::I as TryFrom<usize>>::Error: core::fmt::Debug,
        <K::I as TryInto<usize>>::Error: core::fmt::Debug,
        O: Clone,
        A: Clone,
    {
        let edges = extract_edges(graph);
        let node_labels = collect_array::<K, O>(&graph.h.w.0);
        let edge_labels = collect_array::<K, A>(&graph.h.x.0);
        let source_len = to_usize(graph.s.table.len());
        let target_len = to_usize(graph.t.table.len());
        let mut nodes = vec![
            NodeInfo {
                source_positions: Vec::new(),
                target_positions: Vec::new(),
                source_incidence_count: 0,
                target_incidence_count: 0,
                incident_edges: Vec::new(),
            };
            node_labels.len()
        ];

        for position in 0..to_usize(graph.s.table.len()) {
            let node = to_usize(graph.s.table.get(to_index::<K>(position)));
            nodes[node].source_positions.push(position);
        }
        for position in 0..to_usize(graph.t.table.len()) {
            let node = to_usize(graph.t.table.get(to_index::<K>(position)));
            nodes[node].target_positions.push(position);
        }

        for (edge_idx, edge) in edges.iter().enumerate() {
            for (port, &node) in edge.sources.iter().enumerate() {
                nodes[node].source_incidence_count += 1;
                nodes[node].incident_edges.push(IncidentEdge {
                    edge: edge_idx,
                    direction: Direction::Source,
                    port,
                });
            }
            for (port, &node) in edge.targets.iter().enumerate() {
                nodes[node].target_incidence_count += 1;
                nodes[node].incident_edges.push(IncidentEdge {
                    edge: edge_idx,
                    direction: Direction::Target,
                    port,
                });
            }
        }

        Self {
            node_labels,
            edge_labels,
            source_len,
            target_len,
            nodes,
            edges,
        }
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn source_len(&self) -> usize {
        self.source_len
    }

    fn target_len(&self) -> usize {
        self.target_len
    }

    fn node_label(&self, idx: NodeId) -> &O {
        &self.node_labels[idx]
    }

    fn edge_label(&self, idx: EdgeId) -> &A {
        &self.edge_labels[idx]
    }

    fn node(&self, idx: NodeId) -> &NodeInfo {
        &self.nodes[idx]
    }

    fn edge(&self, idx: EdgeId) -> &EdgeInfo {
        &self.edges[idx]
    }
}

fn extract_edges<K, O, A>(
    graph: &open_hypergraphs::strict::open_hypergraph::OpenHypergraph<K, O, A>,
) -> Vec<EdgeInfo>
where
    K: ArrayKind,
    K::Type<K::I>: NaturalArray<K> + AsRef<K::Index>,
    K::Type<A>: Array<K, A>,
    K::I: TryFrom<usize> + TryInto<usize>,
    <K::I as TryFrom<usize>>::Error: core::fmt::Debug,
    <K::I as TryInto<usize>>::Error: core::fmt::Debug,
{
    let source_lengths = &graph.h.s.sources.table;
    let target_lengths = &graph.h.t.sources.table;
    let source_values = &graph.h.s.values.table;
    let target_values = &graph.h.t.values.table;

    let mut edges = Vec::with_capacity(to_usize(graph.h.x.len()));
    let mut source_offset = 0usize;
    let mut target_offset = 0usize;

    for edge_idx in 0..to_usize(graph.h.x.len()) {
        let source_len = to_usize(source_lengths.get(to_index::<K>(edge_idx)));
        let target_len = to_usize(target_lengths.get(to_index::<K>(edge_idx)));
        let sources = (0..source_len)
            .map(|port| to_usize(source_values.get(to_index::<K>(source_offset + port))))
            .collect();
        let targets = (0..target_len)
            .map(|port| to_usize(target_values.get(to_index::<K>(target_offset + port))))
            .collect();
        edges.push(EdgeInfo { sources, targets });
        source_offset += source_len;
        target_offset += target_len;
    }

    edges
}

fn collect_array<K, T>(array: &K::Type<T>) -> Vec<T>
where
    K: ArrayKind,
    K::Type<T>: Array<K, T>,
    K::I: TryFrom<usize> + TryInto<usize>,
    <K::I as TryFrom<usize>>::Error: core::fmt::Debug,
    <K::I as TryInto<usize>>::Error: core::fmt::Debug,
    T: Clone,
{
    (0..to_usize(array.len()))
        .map(|idx| array.get(to_index::<K>(idx)))
        .collect()
}

fn to_index<K: ArrayKind>(idx: usize) -> K::I
where
    K::I: TryFrom<usize>,
    <K::I as TryFrom<usize>>::Error: core::fmt::Debug,
{
    K::I::try_from(idx).expect("index conversion failed")
}

fn to_usize<I>(idx: I) -> usize
where
    I: TryInto<usize>,
    <I as TryInto<usize>>::Error: core::fmt::Debug,
{
    idx.try_into().expect("usize conversion failed")
}

#[cfg(test)]
mod tests {
    use super::enumerate_matches;
    use open_hypergraphs::array::vec::VecArray;
    use open_hypergraphs::finite_function::FiniteFunction;
    use open_hypergraphs::indexed_coproduct::IndexedCoproduct;
    use open_hypergraphs::semifinite::SemifiniteFunction;
    use open_hypergraphs::strict::vec::Hypergraph as StrictHypergraph;
    use open_hypergraphs::strict::vec::OpenHypergraph as StrictOpenHypergraph;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum NodeLabel {
        A,
        B,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum EdgeLabel {
        F,
        G,
    }

    fn strict_open_hypergraph(
        node_labels: Vec<NodeLabel>,
        edge_labels: Vec<EdgeLabel>,
        edge_sources: Vec<Vec<usize>>,
        edge_targets: Vec<Vec<usize>>,
        sources: Vec<usize>,
        targets: Vec<usize>,
    ) -> StrictOpenHypergraph<NodeLabel, EdgeLabel> {
        let source_lengths = SemifiniteFunction(VecArray(
            edge_sources.iter().map(|ports| ports.len()).collect(),
        ));
        let target_lengths = SemifiniteFunction(VecArray(
            edge_targets.iter().map(|ports| ports.len()).collect(),
        ));
        let source_values = FiniteFunction::new(
            VecArray(edge_sources.into_iter().flatten().collect()),
            node_labels.len(),
        )
        .unwrap();
        let target_values = FiniteFunction::new(
            VecArray(edge_targets.into_iter().flatten().collect()),
            node_labels.len(),
        )
        .unwrap();

        let h = StrictHypergraph::new(
            IndexedCoproduct::from_semifinite(source_lengths, source_values).unwrap(),
            IndexedCoproduct::from_semifinite(target_lengths, target_values).unwrap(),
            SemifiniteFunction(VecArray(node_labels)),
            SemifiniteFunction(VecArray(edge_labels)),
        )
        .unwrap();

        StrictOpenHypergraph::new(
            FiniteFunction::new(VecArray(sources), h.w.len()).unwrap(),
            FiniteFunction::new(VecArray(targets), h.w.len()).unwrap(),
            h,
        )
        .unwrap()
    }

    #[test]
    fn finds_single_match_with_boundary_and_labels() {
        let pattern = strict_open_hypergraph(
            vec![NodeLabel::A, NodeLabel::B],
            vec![EdgeLabel::F],
            vec![vec![0]],
            vec![vec![1]],
            vec![0],
            vec![1],
        );
        let host = strict_open_hypergraph(
            vec![NodeLabel::A, NodeLabel::B, NodeLabel::A],
            vec![EdgeLabel::F, EdgeLabel::G],
            vec![vec![0], vec![2]],
            vec![vec![1], vec![1]],
            vec![0],
            vec![1],
        );

        let matches = enumerate_matches(&pattern, &host);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].node_map, vec![0, 1]);
        assert_eq!(matches[0].edge_map, vec![0]);
    }

    #[test]
    fn rejects_host_with_wrong_boundary_positions() {
        let pattern = strict_open_hypergraph(
            vec![NodeLabel::A, NodeLabel::B],
            vec![EdgeLabel::F],
            vec![vec![0]],
            vec![vec![1]],
            vec![0],
            vec![1],
        );
        let host = strict_open_hypergraph(
            vec![NodeLabel::A, NodeLabel::B],
            vec![EdgeLabel::F],
            vec![vec![0]],
            vec![vec![1]],
            vec![1],
            vec![0],
        );

        let matches = enumerate_matches(&pattern, &host);
        assert!(matches.is_empty());
    }

    #[test]
    fn enumerates_multiple_non_induced_matches() {
        let pattern = strict_open_hypergraph(
            vec![NodeLabel::A, NodeLabel::B],
            vec![EdgeLabel::F],
            vec![vec![0]],
            vec![vec![1]],
            vec![],
            vec![],
        );
        let host = strict_open_hypergraph(
            vec![NodeLabel::A, NodeLabel::B, NodeLabel::A, NodeLabel::B],
            vec![EdgeLabel::F, EdgeLabel::F, EdgeLabel::G],
            vec![vec![0], vec![2], vec![0]],
            vec![vec![1], vec![3], vec![3]],
            vec![],
            vec![],
        );

        let matches = enumerate_matches(&pattern, &host);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].edge_map.len(), 1);
    }
}
