use std::collections::BTreeMap;

use crate::types::{TypeExpr, TypeVar};

#[derive(Debug, Clone, Default)]
pub struct Context {
    vars: BTreeMap<String, TypeExpr>,
    next_var: usize,
}

impl Context {
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

    pub fn set_var(&mut self, name: &str, ty: TypeExpr) {
        self.vars.insert(name.to_string(), ty);
    }

    pub fn entries(&self) -> Vec<(String, TypeExpr)> {
        self.vars
            .iter()
            .map(|(name, ty)| (name.clone(), ty.clone()))
            .collect()
    }
}
