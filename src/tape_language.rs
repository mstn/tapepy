#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SortName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GeneratorName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Sort {
    One,
    Atom(SortName),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Monomial {
    One,
    Atom(SortName),
    Product(Box<Monomial>, Box<Monomial>),
}

impl Monomial {
    pub fn one() -> Self {
        Monomial::One
    }

    pub fn atom(sort: SortName) -> Self {
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

impl Sort {
    pub fn to_monomial(&self) -> Monomial {
        match self {
            Sort::One => Monomial::One,
            Sort::Atom(name) => Monomial::Atom(name.clone()),
        }
    }

    pub fn validate(&self, signature: &MonoidalSignature) {
        match self {
            Sort::One => {}
            Sort::Atom(name) => {
                if !signature.has_sort(name) {
                    panic!("unknown sort `{}`", name.0);
                }
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
    pub name: GeneratorName,
    pub arity: Monomial,
    pub coarity: Monomial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonoidalSignature {
    pub sorts: Vec<SortName>,
    pub generators: Vec<GeneratorSignature>,
}

impl MonoidalSignature {
    pub fn generator(&self, name: &GeneratorName) -> Option<&GeneratorSignature> {
        self.generators.iter().find(|gen| &gen.name == name)
    }

    pub fn has_sort(&self, name: &SortName) -> bool {
        self.sorts.iter().any(|sort| sort == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Circuit {
    Id(Sort),
    Generator(GeneratorName),
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
            Circuit::Id(sorts) => {
                sorts.validate(signature);
                let mono = sorts.to_monomial();
                CircuitType::new(mono.clone(), mono)
            }
            Circuit::Generator(name) => {
                let gen = signature
                    .generator(name)
                    .unwrap_or_else(|| panic!("unknown generator `{}`", name.0));
                gen.arity.validate(signature);
                gen.coarity.validate(signature);
                CircuitType::new(gen.arity.clone(), gen.coarity.clone())
            }
            Circuit::Swap { left, right } => {
                left.validate(signature);
                right.validate(signature);
                CircuitType::new(
                    Monomial::product(left.to_monomial(), right.to_monomial()),
                    Monomial::product(right.to_monomial(), left.to_monomial()),
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
                sorts.validate(signature);
                let mono = sorts.to_monomial();
                CircuitType::new(
                    mono.clone(),
                    Monomial::product(mono.clone(), mono),
                )
            }
            Circuit::Discard(sorts) => {
                sorts.validate(signature);
                CircuitType::new(sorts.to_monomial(), Monomial::one())
            }
            Circuit::Join(sorts) => {
                sorts.validate(signature);
                let mono = sorts.to_monomial();
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
