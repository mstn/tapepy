use std::collections::BTreeMap;

use open_hypergraphs::lax::{NodeId, OpenHypergraph};

struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl DisjointSet {
    fn new(n: usize) -> Self {
        let mut parent = Vec::with_capacity(n);
        for i in 0..n {
            parent.push(i);
        }
        Self {
            parent,
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            let root = self.find(self.parent[x]);
            self.parent[x] = root;
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb {
            return;
        }
        if self.rank[ra] < self.rank[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        if self.rank[ra] == self.rank[rb] {
            self.rank[ra] += 1;
        }
    }
}

pub fn connected_components<O, A>(graph: &OpenHypergraph<O, A>) -> Vec<Vec<NodeId>> {
    connected_components_with_edges(graph)
        .into_iter()
        .map(|component| component.nodes)
        .collect()
}

#[derive(Debug, Clone)]
pub struct HypergraphComponent {
    pub nodes: Vec<NodeId>,
    pub edges: Vec<usize>,
}

pub fn connected_components_with_edges<O, A>(
    graph: &OpenHypergraph<O, A>,
) -> Vec<HypergraphComponent> {
    let (mut node_components, node_to_component) = component_nodes(graph);
    let mut edge_components: Vec<Vec<usize>> = vec![Vec::new(); node_components.len()];
    let mut edge_only_components: Vec<usize> = Vec::new();

    for (edge_idx, edge) in graph.hypergraph.adjacency.iter().enumerate() {
        let anchor = edge
            .sources
            .first()
            .or_else(|| edge.targets.first())
            .map(|node| node.0);
        if let Some(node_idx) = anchor {
            let component_idx = node_to_component[node_idx];
            edge_components[component_idx].push(edge_idx);
        } else {
            edge_only_components.push(edge_idx);
        }
    }

    let mut out = Vec::with_capacity(node_components.len() + edge_only_components.len());
    for (idx, mut nodes) in node_components.drain(..).enumerate() {
        nodes.sort_by_key(|node| node.0);
        let mut edges = edge_components[idx].clone();
        edges.sort();
        out.push(HypergraphComponent { nodes, edges });
    }

    for edge_idx in edge_only_components {
        out.push(HypergraphComponent {
            nodes: Vec::new(),
            edges: vec![edge_idx],
        });
    }

    out
}

fn component_nodes<O, A>(graph: &OpenHypergraph<O, A>) -> (Vec<Vec<NodeId>>, Vec<usize>) {
    let n = graph.hypergraph.nodes.len();
    let mut dsu = DisjointSet::new(n);

    for edge in &graph.hypergraph.adjacency {
        let mut nodes = Vec::with_capacity(edge.sources.len() + edge.targets.len());
        nodes.extend(edge.sources.iter().map(|node| node.0));
        nodes.extend(edge.targets.iter().map(|node| node.0));
        if let Some((&first, rest)) = nodes.split_first() {
            for &node in rest {
                dsu.union(first, node);
            }
        }
    }

    for (from, to) in graph
        .hypergraph
        .quotient
        .0
        .iter()
        .zip(graph.hypergraph.quotient.1.iter())
    {
        dsu.union(from.0, to.0);
    }

    let mut components: Vec<Vec<NodeId>> = Vec::new();
    let mut node_to_component = vec![0; n];
    let mut root_to_component: BTreeMap<usize, usize> = BTreeMap::new();

    for node in 0..n {
        let root = dsu.find(node);
        let component_idx = match root_to_component.get(&root) {
            Some(&idx) => idx,
            None => {
                let idx = components.len();
                root_to_component.insert(root, idx);
                components.push(Vec::new());
                idx
            }
        };
        node_to_component[node] = component_idx;
        components[component_idx].push(NodeId(node));
    }

    (components, node_to_component)
}

#[cfg(test)]
mod tests {
    use super::{connected_components, connected_components_with_edges};
    use open_hypergraphs::lax::OpenHypergraph;

    fn normalize(mut comps: Vec<Vec<open_hypergraphs::lax::NodeId>>) -> Vec<Vec<usize>> {
        let mut out: Vec<Vec<usize>> = comps
            .drain(..)
            .map(|mut comp| {
                comp.sort_by_key(|node| node.0);
                comp.into_iter().map(|node| node.0).collect()
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn connected_components_basic() {
        let mut graph: OpenHypergraph<(), ()> = OpenHypergraph::empty();
        let n0 = graph.new_node(());
        let n1 = graph.new_node(());
        let n2 = graph.new_node(());
        let n3 = graph.new_node(());

        graph.new_edge((), (vec![n0], vec![n1]));
        graph.hypergraph.unify(n2, n3);

        let comps = connected_components(&graph);
        let normalized = normalize(comps);

        assert_eq!(normalized, vec![vec![0, 1], vec![2, 3]]);
    }

    #[test]
    fn connected_components_with_edges_basic() {
        let mut graph: OpenHypergraph<(), ()> = OpenHypergraph::empty();
        let n0 = graph.new_node(());
        let n1 = graph.new_node(());
        let n2 = graph.new_node(());
        let n3 = graph.new_node(());

        graph.new_edge((), (vec![n0], vec![n1]));
        graph.new_edge((), (vec![n2], vec![n3]));

        let mut components = connected_components_with_edges(&graph);
        components.sort_by_key(|component| component.nodes.first().map(|node| node.0).unwrap_or(0));

        assert_eq!(components.len(), 2);
        assert_eq!(components[0].nodes.iter().map(|n| n.0).collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(components[0].edges, vec![0]);
        assert_eq!(components[1].nodes.iter().map(|n| n.0).collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(components[1].edges, vec![1]);
    }
}
