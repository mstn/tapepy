use open_hypergraphs::lax::{Arrow as _, Monoidal, OpenHypergraph};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generator(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Monomial<S> {
    One,
    Atom(S),
    Product(Box<Monomial<S>>, Box<Monomial<S>>),
}

impl<S> Monomial<S> {
    pub fn one() -> Self {
        Monomial::One
    }

    pub fn atom(sort: S) -> Self {
        Monomial::Atom(sort)
    }

    pub fn product(left: Monomial<S>, right: Monomial<S>) -> Self {
        match (left, right) {
            (Monomial::One, right) => right,
            (left, Monomial::One) => left,
            (left, right) => Monomial::Product(Box::new(left), Box::new(right)),
        }
    }

    pub fn validate(&self, signature: &MonoidalSignature<S>)
    where
        S: PartialEq + std::fmt::Debug,
    {
        match self {
            Monomial::One => {}
            Monomial::Atom(name) => {
                if !signature.has_sort(name) {
                    panic!("unknown sort `{:?}`", name);
                }
            }
            Monomial::Product(left, right) => {
                left.validate(signature);
                right.validate(signature);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Polynomial<S> {
    Zero,
    Monomial(Monomial<S>),
    Sum(Box<Polynomial<S>>, Box<Polynomial<S>>),
}

impl<S> Polynomial<S> {
    pub fn zero() -> Self {
        Polynomial::Zero
    }

    pub fn monomial(term: Monomial<S>) -> Self {
        Polynomial::Monomial(term)
    }

    pub fn sum(left: Polynomial<S>, right: Polynomial<S>) -> Self {
        match (left, right) {
            (Polynomial::Zero, right) => right,
            (left, Polynomial::Zero) => left,
            (left, right) => Polynomial::Sum(Box::new(left), Box::new(right)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GeneratorSignature<S> {
    pub name: Generator,
    pub arity: Monomial<S>,
    pub coarity: Monomial<S>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonoidalSignature<S> {
    pub sorts: Vec<S>,
    pub generators: Vec<GeneratorSignature<S>>,
}

impl<S: PartialEq> MonoidalSignature<S> {
    pub fn generator(&self, name: &Generator) -> Option<&GeneratorSignature<S>> {
        self.generators.iter().find(|gen| &gen.name == name)
    }

    pub fn has_sort(&self, name: &S) -> bool {
        self.sorts.iter().any(|sort| sort == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Circuit<S> {
    Id(S),
    IdOne,
    Generator(Generator),
    Swap { left: S, right: S },
    Seq(Box<Circuit<S>>, Box<Circuit<S>>),
    Product(Box<Circuit<S>>, Box<Circuit<S>>),
    Copy(S),
    Discard(S),
    Join(S),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tape<S> {
    Id(Monomial<S>),
    IdZero,
    EmbedCircuit(Box<Circuit<S>>),
    Swap { left: Monomial<S>, right: Monomial<S> },
    Seq(Box<Tape<S>>, Box<Tape<S>>),
    Sum(Box<Tape<S>>, Box<Tape<S>>),
    Discard(Monomial<S>),
    Split(Monomial<S>),
    Create(Monomial<S>),
    Merge(Monomial<S>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitType<S> {
    pub domain: Monomial<S>,
    pub codomain: Monomial<S>,
}

impl<S> CircuitType<S> {
    pub fn new(domain: Monomial<S>, codomain: Monomial<S>) -> Self {
        Self { domain, codomain }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapeType<S> {
    pub domain: Polynomial<S>,
    pub codomain: Polynomial<S>,
}

impl<S> TapeType<S> {
    pub fn new(domain: Polynomial<S>, codomain: Polynomial<S>) -> Self {
        Self { domain, codomain }
    }
}

impl<S: Clone + PartialEq + std::fmt::Debug> Circuit<S> {
    pub fn typing(&self, signature: &MonoidalSignature<S>) -> CircuitType<S> {
        match self {
            Circuit::Id(sort) => {
                if !signature.has_sort(sort) {
                    panic!("unknown sort `{:?}`", sort);
                }
                let mono = Monomial::atom(sort.clone());
                CircuitType::new(mono.clone(), mono)
            }
            Circuit::IdOne => CircuitType::new(Monomial::one(), Monomial::one()),
            Circuit::Generator(name) => {
                let gen = signature
                    .generator(name)
                    .unwrap_or_else(|| panic!("unknown generator `{}`", name.0));
                gen.arity.validate(signature);
                gen.coarity.validate(signature);
                CircuitType::new(gen.arity.clone(), gen.coarity.clone())
            }
            Circuit::Swap { left, right } => {
                if !signature.has_sort(left) {
                    panic!("unknown sort `{:?}`", left);
                }
                if !signature.has_sort(right) {
                    panic!("unknown sort `{:?}`", right);
                }
                CircuitType::new(
                    Monomial::product(Monomial::atom(left.clone()), Monomial::atom(right.clone())),
                    Monomial::product(Monomial::atom(right.clone()), Monomial::atom(left.clone())),
                )
            }
            Circuit::Seq(left, right) => {
                let left_ty = left.typing(signature);
                let right_ty = right.typing(signature);
                CircuitType::new(left_ty.domain, right_ty.codomain)
            }
            Circuit::Product(left, right) => {
                let left_ty = left.typing(signature);
                let right_ty = right.typing(signature);
                CircuitType::new(
                    Monomial::product(left_ty.domain, right_ty.domain),
                    Monomial::product(left_ty.codomain, right_ty.codomain),
                )
            }
            Circuit::Copy(sorts) => {
                if !signature.has_sort(sorts) {
                    panic!("unknown sort `{:?}`", sorts);
                }
                let mono = Monomial::atom(sorts.clone());
                CircuitType::new(mono.clone(), Monomial::product(mono.clone(), mono))
            }
            Circuit::Discard(sorts) => {
                if !signature.has_sort(sorts) {
                    panic!("unknown sort `{:?}`", sorts);
                }
                CircuitType::new(Monomial::atom(sorts.clone()), Monomial::one())
            }
            Circuit::Join(sorts) => {
                if !signature.has_sort(sorts) {
                    panic!("unknown sort `{:?}`", sorts);
                }
                let mono = Monomial::atom(sorts.clone());
                CircuitType::new(Monomial::product(mono.clone(), mono.clone()), mono)
            }
        }
    }
}

impl<S: Clone + PartialEq + std::fmt::Debug> Tape<S> {
    pub fn typing(&self, signature: &MonoidalSignature<S>) -> TapeType<S> {
        match self {
            Tape::Id(mono) => {
                mono.validate(signature);
                let poly = Polynomial::monomial(mono.clone());
                TapeType::new(poly.clone(), poly)
            }
            Tape::IdZero => TapeType::new(Polynomial::zero(), Polynomial::zero()),
            Tape::EmbedCircuit(circuit) => {
                let ty = circuit.typing(signature);
                TapeType::new(
                    Polynomial::monomial(ty.domain),
                    Polynomial::monomial(ty.codomain),
                )
            }
            Tape::Swap { left, right } => {
                left.validate(signature);
                right.validate(signature);
                TapeType::new(
                    Polynomial::monomial(Monomial::product(left.clone(), right.clone())),
                    Polynomial::monomial(Monomial::product(right.clone(), left.clone())),
                )
            }
            Tape::Seq(left, right) => {
                let left_ty = left.typing(signature);
                let right_ty = right.typing(signature);
                TapeType::new(left_ty.domain, right_ty.codomain)
            }
            Tape::Sum(left, right) => {
                let left_ty = left.typing(signature);
                let right_ty = right.typing(signature);
                TapeType::new(
                    Polynomial::sum(left_ty.domain, right_ty.domain),
                    Polynomial::sum(left_ty.codomain, right_ty.codomain),
                )
            }
            Tape::Discard(mono) => {
                mono.validate(signature);
                TapeType::new(Polynomial::monomial(mono.clone()), Polynomial::zero())
            }
            Tape::Split(mono) => {
                mono.validate(signature);
                let mono_poly = Polynomial::monomial(mono.clone());
                TapeType::new(
                    mono_poly.clone(),
                    Polynomial::sum(mono_poly.clone(), mono_poly),
                )
            }
            Tape::Create(mono) => {
                mono.validate(signature);
                TapeType::new(Polynomial::zero(), Polynomial::monomial(mono.clone()))
            }
            Tape::Merge(mono) => {
                mono.validate(signature);
                let mono_poly = Polynomial::monomial(mono.clone());
                TapeType::new(
                    Polynomial::sum(mono_poly.clone(), mono_poly),
                    Polynomial::monomial(mono.clone()),
                )
            }
        }
    }
}

impl<S: Clone + PartialEq + std::fmt::Debug> Circuit<S> {
    pub fn to_hypergraph(
        &self,
        signature: &MonoidalSignature<S>,
    ) -> OpenHypergraph<S, Generator> {
        match self {
            Circuit::Id(sort) => {
                if !signature.has_sort(sort) {
                    panic!("unknown sort `{:?}`", sort);
                }
                OpenHypergraph::identity(vec![sort.clone()])
            }
            Circuit::IdOne => OpenHypergraph::empty(),
            Circuit::Generator(name) => {
                let gen = signature
                    .generator(name)
                    .unwrap_or_else(|| panic!("unknown generator `{}`", name.0));
                gen.arity.validate(signature);
                gen.coarity.validate(signature);
                OpenHypergraph::singleton(
                    gen.name.clone(),
                    monomial_to_sorts(&gen.arity),
                    monomial_to_sorts(&gen.coarity),
                )
            }
            Circuit::Swap { left, right } => {
                if !signature.has_sort(left) {
                    panic!("unknown sort `{:?}`", left);
                }
                if !signature.has_sort(right) {
                    panic!("unknown sort `{:?}`", right);
                }
                let mut graph = OpenHypergraph::empty();
                let left_id = graph.new_node(left.clone());
                let right_id = graph.new_node(right.clone());
                graph.sources = vec![left_id, right_id];
                graph.targets = vec![right_id, left_id];
                graph
            }
            Circuit::Seq(left, right) => {
                let left_graph = left.to_hypergraph(signature);
                let right_graph = right.to_hypergraph(signature);
                compose_lax_unchecked(&left_graph, &right_graph)
            }
            Circuit::Product(left, right) => {
                let left_graph = left.to_hypergraph(signature);
                let right_graph = right.to_hypergraph(signature);
                left_graph.tensor(&right_graph)
            }
            Circuit::Copy(sort) => {
                if !signature.has_sort(sort) {
                    panic!("unknown sort `{:?}`", sort);
                }
                let mut graph = OpenHypergraph::empty();
                let node = graph.new_node(sort.clone());
                graph.sources = vec![node];
                graph.targets = vec![node, node];
                graph
            }
            Circuit::Discard(sort) => {
                if !signature.has_sort(sort) {
                    panic!("unknown sort `{:?}`", sort);
                }
                let mut graph = OpenHypergraph::empty();
                let node = graph.new_node(sort.clone());
                graph.sources = vec![node];
                graph.targets = Vec::new();
                graph
            }
            Circuit::Join(sort) => {
                if !signature.has_sort(sort) {
                    panic!("unknown sort `{:?}`", sort);
                }
                let mut graph = OpenHypergraph::empty();
                let node = graph.new_node(sort.clone());
                graph.sources = vec![node, node];
                graph.targets = vec![node];
                graph
            }
        }
    }
}

fn monomial_to_sorts<S: Clone>(monomial: &Monomial<S>) -> Vec<S> {
    match monomial {
        Monomial::One => Vec::new(),
        Monomial::Atom(name) => vec![name.clone()],
        Monomial::Product(left, right) => {
            let mut left_terms = monomial_to_sorts(left);
            let mut right_terms = monomial_to_sorts(right);
            left_terms.append(&mut right_terms);
            left_terms
        }
    }
}

fn compose_lax_unchecked<S: Clone>(
    lhs: &OpenHypergraph<S, Generator>,
    rhs: &OpenHypergraph<S, Generator>,
) -> OpenHypergraph<S, Generator> {
    if lhs.targets.len() != rhs.sources.len() {
        panic!(
            "unchecked composition requires same arity, got {} vs {}",
            lhs.targets.len(),
            rhs.sources.len()
        );
    }

    let n = lhs.hypergraph.nodes.len();
    let mut composed = lhs.tensor(rhs);

    for (u, v) in lhs.targets.iter().zip(rhs.sources.iter()) {
        composed.unify(*u, open_hypergraphs::lax::NodeId(v.0 + n));
    }

    composed.sources = composed.sources[..lhs.sources.len()].to_vec();
    composed.targets = composed.targets[lhs.targets.len()..].to_vec();
    composed
}
