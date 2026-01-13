use std::collections::BTreeMap;

use crate::types::{TypeExpr, TypeVar};

#[derive(Debug, Clone, Default)]
pub struct Gamma {
    vars: BTreeMap<String, TypeExpr>,
    next_var: usize,
}

impl Gamma {
    pub fn get_or_insert_var(&mut self, name: &str) -> TypeExpr {
        if let Some(ty) = self.vars.get(name) {
            return ty.clone();
        }

        let var = TypeExpr::Var(TypeVar(self.next_var));
        self.next_var += 1;
        self.vars.insert(name.to_string(), var.clone());
        var
    }

    pub fn get(&self, name: &str) -> Option<&TypeExpr> {
        self.vars.get(name)
    }

    pub fn snapshot(&self) -> BTreeMap<String, TypeExpr> {
        self.vars.clone()
    }
}
