#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sort(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generator(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Monomial {
    One,
    Atom(Sort),
    Product(Box<Monomial>, Box<Monomial>),
}

impl Monomial {
    pub fn one() -> Self {
        Monomial::One
    }

    pub fn atom(sort: Sort) -> Self {
        Monomial::Atom(sort)
    }

    pub fn product(left: Monomial, right: Monomial) -> Self {
        match (left, right) {
            (Monomial::One, right) => right,
            (left, Monomial::One) => left,
            (left, right) => Monomial::Product(Box::new(left), Box::new(right)),
        }
    }

    pub fn validate(&self, signature: &MonoidalSignature) {
        match self {
            Monomial::One => {}
            Monomial::Atom(name) => {
                if !signature.has_sort(name) {
                    panic!("unknown sort `{}`", name.0);
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
pub enum Polynomial {
    Zero,
    Monomial(Monomial),
    Sum(Box<Polynomial>, Box<Polynomial>),
}

impl Polynomial {
    pub fn zero() -> Self {
        Polynomial::Zero
    }

    pub fn monomial(term: Monomial) -> Self {
        Polynomial::Monomial(term)
    }

    pub fn sum(left: Polynomial, right: Polynomial) -> Self {
        match (left, right) {
            (Polynomial::Zero, right) => right,
            (left, Polynomial::Zero) => left,
            (left, right) => Polynomial::Sum(Box::new(left), Box::new(right)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GeneratorSignature {
    pub name: Generator,
    pub arity: Monomial,
    pub coarity: Monomial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonoidalSignature {
    pub sorts: Vec<Sort>,
    pub generators: Vec<GeneratorSignature>,
}

impl MonoidalSignature {
    pub fn generator(&self, name: &Generator) -> Option<&GeneratorSignature> {
        self.generators.iter().find(|gen| &gen.name == name)
    }

    pub fn has_sort(&self, name: &Sort) -> bool {
        self.sorts.iter().any(|sort| sort == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Circuit {
    Id(Sort),
    IdOne,
    Generator(Generator),
    Swap { left: Sort, right: Sort },
    Seq(Box<Circuit>, Box<Circuit>),
    Product(Box<Circuit>, Box<Circuit>),
    Copy(Sort),
    Discard(Sort),
    Join(Sort),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tape {
    Id(Monomial),
    IdZero,
    EmbedCircuit(Box<Circuit>),
    Swap { left: Monomial, right: Monomial },
    Seq(Box<Tape>, Box<Tape>),
    Sum(Box<Tape>, Box<Tape>),
    Discard(Monomial),
    Split(Monomial),
    Create(Monomial),
    Merge(Monomial),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitType {
    pub domain: Monomial,
    pub codomain: Monomial,
}

impl CircuitType {
    pub fn new(domain: Monomial, codomain: Monomial) -> Self {
        Self { domain, codomain }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapeType {
    pub domain: Polynomial,
    pub codomain: Polynomial,
}

impl TapeType {
    pub fn new(domain: Polynomial, codomain: Polynomial) -> Self {
        Self { domain, codomain }
    }
}

impl Circuit {
    pub fn typing(&self, signature: &MonoidalSignature) -> CircuitType {
        match self {
            Circuit::Id(sort) => {
                if !signature.has_sort(sort) {
                    panic!("unknown sort `{}`", sort.0);
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
                    panic!("unknown sort `{}`", left.0);
                }
                if !signature.has_sort(right) {
                    panic!("unknown sort `{}`", right.0);
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
                    panic!("unknown sort `{}`", sorts.0);
                }
                let mono = Monomial::atom(sorts.clone());
                CircuitType::new(
                    mono.clone(),
                    Monomial::product(mono.clone(), mono),
                )
            }
            Circuit::Discard(sorts) => {
                if !signature.has_sort(sorts) {
                    panic!("unknown sort `{}`", sorts.0);
                }
                CircuitType::new(Monomial::atom(sorts.clone()), Monomial::one())
            }
            Circuit::Join(sorts) => {
                if !signature.has_sort(sorts) {
                    panic!("unknown sort `{}`", sorts.0);
                }
                let mono = Monomial::atom(sorts.clone());
                CircuitType::new(Monomial::product(mono.clone(), mono.clone()), mono)
            }
        }
    }
}

impl Tape {
    pub fn typing(&self, signature: &MonoidalSignature) -> TapeType {
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
