#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeVar(pub &'static str);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PyType {
    Any,
    NoneType,
    Bool,
    Int,
    Float,
    Complex,
    Str,
    Bytes,
    ByteArray,
    Range,
    Slice,
    MemoryView,
    Object,
    Type,
    Var(TypeVar),
    Iterable(Box<PyType>),
    Sequence(Box<PyType>),
    List(Box<PyType>),
    Tuple(Vec<PyType>),
    TupleOf(Box<PyType>),
    Dict(Box<PyType>, Box<PyType>),
    Set(Box<PyType>),
    FrozenSet(Box<PyType>),
    Mapping(Box<PyType>, Box<PyType>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constraint {
    Numeric(TypeVar),
    Iterable(TypeVar),
    Mapping(TypeVar, TypeVar),
    Sequence(TypeVar),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScheme {
    pub params: Vec<TypeVar>,
    pub inputs: Vec<PyType>,
    pub output: PyType,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinSignature {
    pub name: &'static str,
    pub schemes: Vec<TypeScheme>,
}

pub fn builtin_type_signatures() -> Vec<BuiltinSignature> {
    let t = TypeVar("T");
    let u = TypeVar("U");
    let k = TypeVar("K");
    let v = TypeVar("V");

    vec![
        BuiltinSignature {
            name: "bool",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::Bool,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any],
                    output: PyType::Bool,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "int",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::Int,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any],
                    output: PyType::Int,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "float",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::Float,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any],
                    output: PyType::Float,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "complex",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::Complex,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any],
                    output: PyType::Complex,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any, PyType::Any],
                    output: PyType::Complex,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "str",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::Str,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any],
                    output: PyType::Str,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "bytes",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::Bytes,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                    output: PyType::Bytes,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Str, PyType::Str],
                    output: PyType::Bytes,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "bytearray",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::ByteArray,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                    output: PyType::ByteArray,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Str, PyType::Str],
                    output: PyType::ByteArray,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "list",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::List(Box::new(PyType::Any)),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                    output: PyType::List(Box::new(PyType::Var(t.clone()))),
                    constraints: vec![Constraint::Iterable(t.clone())],
                },
            ],
        },
        BuiltinSignature {
            name: "tuple",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::TupleOf(Box::new(PyType::Any)),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                    output: PyType::TupleOf(Box::new(PyType::Var(t.clone()))),
                    constraints: vec![Constraint::Iterable(t.clone())],
                },
            ],
        },
        BuiltinSignature {
            name: "dict",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::Dict(Box::new(PyType::Any), Box::new(PyType::Any)),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![k.clone(), v.clone()],
                    inputs: vec![PyType::Mapping(
                        Box::new(PyType::Var(k.clone())),
                        Box::new(PyType::Var(v.clone())),
                    )],
                    output: PyType::Dict(
                        Box::new(PyType::Var(k.clone())),
                        Box::new(PyType::Var(v.clone())),
                    ),
                    constraints: vec![Constraint::Mapping(k.clone(), v.clone())],
                },
                TypeScheme {
                    params: vec![k.clone(), v.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Tuple(vec![
                        PyType::Var(k.clone()),
                        PyType::Var(v.clone()),
                    ])))],
                    output: PyType::Dict(
                        Box::new(PyType::Var(k.clone())),
                        Box::new(PyType::Var(v.clone())),
                    ),
                    constraints: vec![Constraint::Iterable(k.clone())],
                },
            ],
        },
        BuiltinSignature {
            name: "set",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::Set(Box::new(PyType::Any)),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                    output: PyType::Set(Box::new(PyType::Var(t.clone()))),
                    constraints: vec![Constraint::Iterable(t.clone())],
                },
            ],
        },
        BuiltinSignature {
            name: "frozenset",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::FrozenSet(Box::new(PyType::Any)),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                    output: PyType::FrozenSet(Box::new(PyType::Var(t.clone()))),
                    constraints: vec![Constraint::Iterable(t.clone())],
                },
            ],
        },
        BuiltinSignature {
            name: "range",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int],
                    output: PyType::Range,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int],
                    output: PyType::Range,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int, PyType::Int],
                    output: PyType::Range,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "slice",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int],
                    output: PyType::Slice,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int],
                    output: PyType::Slice,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int, PyType::Int],
                    output: PyType::Slice,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "memoryview",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Any],
                output: PyType::MemoryView,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "object",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![],
                output: PyType::Object,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "type",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any],
                    output: PyType::Type,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![
                        PyType::Str,
                        PyType::TupleOf(Box::new(PyType::Type)),
                        PyType::Dict(Box::new(PyType::Str), Box::new(PyType::Any)),
                    ],
                    output: PyType::Type,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "abs",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![PyType::Var(t.clone())],
                output: PyType::Var(t.clone()),
                constraints: vec![Constraint::Numeric(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "len",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![PyType::Sequence(Box::new(PyType::Var(t.clone())))],
                output: PyType::Int,
                constraints: vec![Constraint::Sequence(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "iter",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                output: PyType::Iterable(Box::new(PyType::Var(t.clone()))),
                constraints: vec![Constraint::Iterable(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "next",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                output: PyType::Var(t.clone()),
                constraints: vec![Constraint::Iterable(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "enumerate",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                output: PyType::Iterable(Box::new(PyType::Tuple(vec![
                    PyType::Int,
                    PyType::Var(t.clone()),
                ]))),
                constraints: vec![Constraint::Iterable(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "zip",
            schemes: vec![TypeScheme {
                params: vec![t.clone(), u.clone()],
                inputs: vec![
                    PyType::Iterable(Box::new(PyType::Var(t.clone()))),
                    PyType::Iterable(Box::new(PyType::Var(u.clone()))),
                ],
                output: PyType::Iterable(Box::new(PyType::Tuple(vec![
                    PyType::Var(t.clone()),
                    PyType::Var(u.clone()),
                ]))),
                constraints: vec![
                    Constraint::Iterable(t.clone()),
                    Constraint::Iterable(u.clone()),
                ],
            }],
        },
        BuiltinSignature {
            name: "map",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![
                    PyType::Any,
                    PyType::Iterable(Box::new(PyType::Var(t.clone()))),
                ],
                output: PyType::Iterable(Box::new(PyType::Any)),
                constraints: vec![Constraint::Iterable(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "filter",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![
                    PyType::Any,
                    PyType::Iterable(Box::new(PyType::Var(t.clone()))),
                ],
                output: PyType::Iterable(Box::new(PyType::Var(t.clone()))),
                constraints: vec![Constraint::Iterable(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "sum",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                output: PyType::Var(t.clone()),
                constraints: vec![
                    Constraint::Numeric(t.clone()),
                    Constraint::Iterable(t.clone()),
                ],
            }],
        },
        BuiltinSignature {
            name: "min",
            schemes: vec![
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Var(t.clone()), PyType::Var(t.clone())],
                    output: PyType::Var(t.clone()),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                    output: PyType::Var(t.clone()),
                    constraints: vec![Constraint::Iterable(t.clone())],
                },
            ],
        },
        BuiltinSignature {
            name: "max",
            schemes: vec![
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Var(t.clone()), PyType::Var(t.clone())],
                    output: PyType::Var(t.clone()),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                    output: PyType::Var(t.clone()),
                    constraints: vec![Constraint::Iterable(t.clone())],
                },
            ],
        },
        BuiltinSignature {
            name: "sorted",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![PyType::Iterable(Box::new(PyType::Var(t.clone())))],
                output: PyType::List(Box::new(PyType::Var(t.clone()))),
                constraints: vec![Constraint::Iterable(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "all",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Iterable(Box::new(PyType::Any))],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "any",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Iterable(Box::new(PyType::Any))],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "repr",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Any],
                output: PyType::Str,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "ascii",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Any],
                output: PyType::Str,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "format",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any],
                    output: PyType::Str,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any, PyType::Str],
                    output: PyType::Str,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "round",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Float],
                    output: PyType::Float,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Float, PyType::Int],
                    output: PyType::Float,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "pow",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int],
                    output: PyType::Int,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Float, PyType::Float],
                    output: PyType::Float,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Complex, PyType::Complex],
                    output: PyType::Complex,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "divmod",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int],
                    output: PyType::Tuple(vec![PyType::Int, PyType::Int]),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Float, PyType::Float],
                    output: PyType::Tuple(vec![PyType::Float, PyType::Float]),
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "print",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![],
                    output: PyType::NoneType,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Any],
                    output: PyType::NoneType,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "isinstance",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Any, PyType::Any],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "+",
            schemes: vec![
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Var(t.clone()), PyType::Var(t.clone())],
                    output: PyType::Var(t.clone()),
                    constraints: vec![Constraint::Numeric(t.clone())],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Str, PyType::Str],
                    output: PyType::Str,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![
                        PyType::List(Box::new(PyType::Var(t.clone()))),
                        PyType::List(Box::new(PyType::Var(t.clone()))),
                    ],
                    output: PyType::List(Box::new(PyType::Var(t.clone()))),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![
                        PyType::TupleOf(Box::new(PyType::Var(t.clone()))),
                        PyType::TupleOf(Box::new(PyType::Var(t.clone()))),
                    ],
                    output: PyType::TupleOf(Box::new(PyType::Var(t.clone()))),
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "-",
            schemes: vec![TypeScheme {
                params: vec![t.clone()],
                inputs: vec![PyType::Var(t.clone()), PyType::Var(t.clone())],
                output: PyType::Var(t.clone()),
                constraints: vec![Constraint::Numeric(t.clone())],
            }],
        },
        BuiltinSignature {
            name: "*",
            schemes: vec![
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Var(t.clone()), PyType::Var(t.clone())],
                    output: PyType::Var(t.clone()),
                    constraints: vec![Constraint::Numeric(t.clone())],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Str, PyType::Int],
                    output: PyType::Str,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Int, PyType::Str],
                    output: PyType::Str,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::List(Box::new(PyType::Var(t.clone()))), PyType::Int],
                    output: PyType::List(Box::new(PyType::Var(t.clone()))),
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![t.clone()],
                    inputs: vec![PyType::Int, PyType::List(Box::new(PyType::Var(t.clone())))],
                    output: PyType::List(Box::new(PyType::Var(t.clone()))),
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "/",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int],
                    output: PyType::Float,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Float, PyType::Float],
                    output: PyType::Float,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Complex, PyType::Complex],
                    output: PyType::Complex,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "//",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int],
                    output: PyType::Int,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Float, PyType::Float],
                    output: PyType::Float,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "%",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int],
                    output: PyType::Int,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Float, PyType::Float],
                    output: PyType::Float,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "**",
            schemes: vec![
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Int, PyType::Int],
                    output: PyType::Int,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Float, PyType::Float],
                    output: PyType::Float,
                    constraints: vec![],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Complex, PyType::Complex],
                    output: PyType::Complex,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "&",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Int, PyType::Int],
                output: PyType::Int,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "|",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Int, PyType::Int],
                output: PyType::Int,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "^",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Int, PyType::Int],
                output: PyType::Int,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "<<",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Int, PyType::Int],
                output: PyType::Int,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: ">>",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Int, PyType::Int],
                output: PyType::Int,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "@",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Any, PyType::Any],
                output: PyType::Any,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "==",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Any, PyType::Any],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "!=",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Any, PyType::Any],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "<",
            schemes: vec![
                TypeScheme {
                    params: vec![t.clone(), u.clone()],
                    inputs: vec![PyType::Var(t.clone()), PyType::Var(u.clone())],
                    output: PyType::Bool,
                    constraints: vec![Constraint::Numeric(t.clone()), Constraint::Numeric(u.clone())],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Str, PyType::Str],
                    output: PyType::Bool,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "<=",
            schemes: vec![
                TypeScheme {
                    params: vec![t.clone(), u.clone()],
                    inputs: vec![PyType::Var(t.clone()), PyType::Var(u.clone())],
                    output: PyType::Bool,
                    constraints: vec![Constraint::Numeric(t.clone()), Constraint::Numeric(u.clone())],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Str, PyType::Str],
                    output: PyType::Bool,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: ">",
            schemes: vec![
                TypeScheme {
                    params: vec![t.clone(), u.clone()],
                    inputs: vec![PyType::Var(t.clone()), PyType::Var(u.clone())],
                    output: PyType::Bool,
                    constraints: vec![Constraint::Numeric(t.clone()), Constraint::Numeric(u.clone())],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Str, PyType::Str],
                    output: PyType::Bool,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: ">=",
            schemes: vec![
                TypeScheme {
                    params: vec![t.clone(), u.clone()],
                    inputs: vec![PyType::Var(t.clone()), PyType::Var(u.clone())],
                    output: PyType::Bool,
                    constraints: vec![Constraint::Numeric(t.clone()), Constraint::Numeric(u.clone())],
                },
                TypeScheme {
                    params: vec![],
                    inputs: vec![PyType::Str, PyType::Str],
                    output: PyType::Bool,
                    constraints: vec![],
                },
            ],
        },
        BuiltinSignature {
            name: "and",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Bool, PyType::Bool],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "or",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Bool, PyType::Bool],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "not",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Bool],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
        BuiltinSignature {
            name: "in",
            schemes: vec![TypeScheme {
                params: vec![],
                inputs: vec![PyType::Any, PyType::Iterable(Box::new(PyType::Any))],
                output: PyType::Bool,
                constraints: vec![],
            }],
        },
    ]
}
