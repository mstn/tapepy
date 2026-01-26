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

    let mut components: BTreeMap<usize, Vec<NodeId>> = BTreeMap::new();
    for node in 0..n {
        let root = dsu.find(node);
        components.entry(root).or_default().push(NodeId(node));
    }

    let mut out: Vec<Vec<NodeId>> = components.into_values().collect();
    for component in &mut out {
        component.sort_by_key(|node| node.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::connected_components;
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
}
