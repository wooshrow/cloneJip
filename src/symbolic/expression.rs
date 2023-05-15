//! Symbolic types representing the values on the stack while symbolically executing a program
//!

use core::fmt;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use super::ref_values::{
    EvaluatedRefs, Interval, IntervalMap, Reference, ReferenceValue, SymRefType,
};
use crate::{ast::*, shared::panic_with_diagnostics, symbolic::memory::SymMemory};

#[derive(Clone)]
pub struct PathConstraints {
    constraints: Vec<PathConstraint>,
}

#[derive(Clone)]
pub enum PathConstraint {
    Assert(SymExpression),
    Assume(SymExpression),
}

impl Default for PathConstraints {
    fn default() -> Self {
        PathConstraints {
            constraints: vec![],
        }
    }
}

impl PathConstraints {
    /// combine constraints over true as follows: `assume(a), assert(b) -> a ==> b && true`
    pub fn combine_over_true(&self) -> SymExpression {
        let mut constraints = SymExpression::Literal(Literal::Boolean(true));

        for constraint in self.constraints.iter().rev() {
            match constraint {
                PathConstraint::Assert(expr) => {
                    constraints = SymExpression::And(Box::new(expr.clone()), Box::new(constraints));
                }
                PathConstraint::Assume(expr) => {
                    constraints =
                        SymExpression::Implies(Box::new(expr.clone()), Box::new(constraints));
                }
            }
        }

        return constraints;
    }
    /// combine constraints as a conjunction as follows: `assume(a), assert(b) -> a && b`
    pub fn conjunct(&self) -> SymExpression {
        let mut constraints = SymExpression::Literal(Literal::Boolean(true));

        for constraint in self.constraints.iter().rev() {
            match constraint {
                PathConstraint::Assert(expr) => {
                    constraints = SymExpression::And(Box::new(expr.clone()), Box::new(constraints));
                }
                PathConstraint::Assume(expr) => {
                    constraints = SymExpression::And(Box::new(expr.clone()), Box::new(constraints));
                }
            }
        }

        return constraints;
    }

    /// adds a new constraint
    pub fn push_assertion(&mut self, assertion: SymExpression) {
        self.constraints.push(PathConstraint::Assert(assertion));
    }
    /// adds a new constraint
    pub fn push_assumption(&mut self, assumption: SymExpression) {
        self.constraints.push(PathConstraint::Assume(assumption));
    }
    /// pops top most constraint
    pub fn pop(&mut self) {
        self.constraints.pop();
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SymType {
    Int,
    Bool,
    Ref(SymRefType),
}

#[derive(Clone)]
pub enum SymExpression {
    Implies(Box<SymExpression>, Box<SymExpression>),
    Forall(Forall),
    Exists(Identifier, Identifier, Identifier, Box<Expression>),
    And(Box<SymExpression>, Box<SymExpression>),
    Or(Box<SymExpression>, Box<SymExpression>),
    EQ(Box<SymExpression>, Box<SymExpression>),
    NE(Box<SymExpression>, Box<SymExpression>),
    LT(Box<SymExpression>, Box<SymExpression>),
    GT(Box<SymExpression>, Box<SymExpression>),
    GEQ(Box<SymExpression>, Box<SymExpression>),
    LEQ(Box<SymExpression>, Box<SymExpression>),
    Plus(Box<SymExpression>, Box<SymExpression>),
    Minus(Box<SymExpression>, Box<SymExpression>),
    Multiply(Box<SymExpression>, Box<SymExpression>),
    Divide(Box<SymExpression>, Box<SymExpression>),
    Mod(Box<SymExpression>, Box<SymExpression>),
    Negative(Box<SymExpression>),
    Not(Box<SymExpression>),
    Literal(Literal),
    FreeVariable(SymType, Identifier),
    Reference(Reference),
    Uninitialized,
}

impl SymExpression {
    /// destructs forall and exists quantifiers and then
    /// generates a substituted expression from it
    pub fn new(sym_memory: &SymMemory, expr: Expression) -> Self {
        match expr {
            Expression::Forall(arr_name, index, value, inner_expr) => {
                let r = match sym_memory.stack_get(&arr_name) {
                    Some(SymExpression::Reference(r)) => r,
                    Some(_) => panic_with_diagnostics(
                        &format!(
                            "In '{:?}' identifier {} is not a reference",
                            inner_expr, arr_name
                        ),
                        &sym_memory,
                    ),
                    None => panic_with_diagnostics(
                        &format!(
                            "In '{:?}' identifier {} is not declared",
                            inner_expr, arr_name
                        ),
                        &sym_memory,
                    ),
                };
                SymExpression::Forall(Forall::new(
                    r,
                    index,
                    value,
                    *inner_expr,
                    sym_memory.clone(),
                ))
            }
            Expression::Exists(arr_name, index, value, expr) => todo!(),
            Expression::Identifier(id) => match sym_memory.stack_get(&id) {
                Some(sym_expr) => sym_expr,
                _ => panic_with_diagnostics(&format!("{} was not declared", id), &sym_memory),
            },
            Expression::SizeOf(arr_name) => match sym_memory.stack_get(&arr_name) {
                Some(SymExpression::Reference(r)) => match sym_memory.heap_get_unsafe(&r) {
                    ReferenceValue::Array((_, _, size, _)) => size.clone(),
                    _ => panic_with_diagnostics(
                        &format!("{} is not an array", arr_name),
                        &sym_memory,
                    ),
                },
                _ => {
                    panic_with_diagnostics(&format!("{} is not a reference", arr_name), &sym_memory)
                }
            },
            Expression::Implies(l, r) => SymExpression::Implies(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::And(l, r) => SymExpression::And(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::Or(l, r) => SymExpression::Or(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::EQ(l, r) => SymExpression::EQ(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::NE(l, r) => SymExpression::NE(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::LT(l, r) => SymExpression::LT(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::GT(l, r) => SymExpression::GT(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::GEQ(l, r) => SymExpression::GEQ(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::LEQ(l, r) => SymExpression::LEQ(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::Plus(l, r) => SymExpression::Plus(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::Minus(l, r) => SymExpression::Minus(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::Multiply(l, r) => SymExpression::Multiply(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::Divide(l, r) => SymExpression::Divide(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::Mod(l, r) => SymExpression::Mod(
                Box::new(SymExpression::new(sym_memory, *l)),
                Box::new(SymExpression::new(sym_memory, *r)),
            ),
            Expression::Negative(expr) => {
                SymExpression::Negative(Box::new(SymExpression::new(sym_memory, *expr)))
            }
            Expression::Not(expr) => {
                SymExpression::Not(Box::new(SymExpression::new(sym_memory, *expr)))
            }
            Expression::Literal(lit) => SymExpression::Literal(lit),
        }
    }

    /// front end simplifier, only pass eval_refs if it makes sense to evaluate the lazy refs
    pub fn eval(self, i: &IntervalMap, eval_refs: Option<&EvaluatedRefs>) -> Self {
        match &self {
            SymExpression::And(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (SymExpression::Literal(Literal::Boolean(false)), _) => {
                        SymExpression::Literal(Literal::Boolean(false))
                    }
                    (_, SymExpression::Literal(Literal::Boolean(false))) => {
                        SymExpression::Literal(Literal::Boolean(false))
                    }
                    (
                        SymExpression::Literal(Literal::Boolean(true)),
                        SymExpression::Literal(Literal::Boolean(true)),
                    ) => SymExpression::Literal(Literal::Boolean(true)),
                    (l_simple, r_simple) => {
                        SymExpression::And(Box::new(l_simple), Box::new(r_simple))
                    }
                }
            }
            SymExpression::Or(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (SymExpression::Literal(Literal::Boolean(true)), _) => {
                        SymExpression::Literal(Literal::Boolean(true))
                    }
                    (_, SymExpression::Literal(Literal::Boolean(true))) => {
                        SymExpression::Literal(Literal::Boolean(true))
                    }
                    (l_simple, r_simple) => {
                        SymExpression::Or(Box::new(l_simple), Box::new(r_simple))
                    }
                }
            }
            SymExpression::Implies(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (SymExpression::Literal(Literal::Boolean(false)), _) => {
                        SymExpression::Literal(Literal::Boolean(true))
                    }
                    (_, SymExpression::Literal(Literal::Boolean(true))) => {
                        SymExpression::Literal(Literal::Boolean(true))
                    }
                    (
                        SymExpression::Literal(Literal::Boolean(_)),
                        SymExpression::Literal(Literal::Boolean(_)),
                    ) => SymExpression::Literal(Literal::Boolean(false)),
                    (l_simple, r_simple) => {
                        SymExpression::Implies(Box::new(l_simple), Box::new(r_simple))
                    }
                }
            }
            // simplify not equal to equal
            SymExpression::NE(l, r) => SymExpression::Not(Box::new(SymExpression::EQ(
                Box::new(*l.clone()),
                Box::new(*r.clone()),
            )))
            .eval(i, eval_refs),
            SymExpression::EQ(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    // if lit or fv are equal => true
                    (SymExpression::Literal(l_lit), SymExpression::Literal(r_lit)) => {
                        SymExpression::Literal(Literal::Boolean(l_lit == r_lit))
                    }
                    (
                        SymExpression::FreeVariable(_, l_fv),
                        SymExpression::FreeVariable(__, r_fv),
                    ) if l_fv == r_fv => SymExpression::Literal(Literal::Boolean(true)),

                    (l_simple, r_simple) => {
                        match (Interval::infer(&l_simple, i), Interval::infer(&r_simple, i)) {
                            //check if intervals have no intersection
                            (Interval(a, b), Interval(d, c)) if b < d || c < a => {
                                SymExpression::Literal(Literal::Boolean(true))
                            }
                            _ => SymExpression::EQ(Box::new(l_simple), Box::new(r_simple)),
                        }
                    }
                }
            }

            // Define LEQ, GT and GEQ in terms of LT
            SymExpression::LEQ(l, r) => SymExpression::Not(Box::new(SymExpression::LT(
                Box::new(*r.clone()),
                Box::new(*l.clone()),
            )))
            .eval(i, eval_refs),
            SymExpression::GT(l, r) => Box::new(SymExpression::LT(
                Box::new(*r.clone()),
                Box::new(*l.clone()),
            ))
            .eval(i, eval_refs),
            SymExpression::GEQ(l, r) => SymExpression::Not(Box::new(SymExpression::LT(
                Box::new(*l.clone()),
                Box::new(*r.clone()),
            )))
            .eval(i, eval_refs),
            SymExpression::LT(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (
                        SymExpression::Literal(Literal::Integer(l_lit)),
                        SymExpression::Literal(Literal::Integer(r_lit)),
                    ) => SymExpression::Literal(Literal::Boolean(l_lit < r_lit)),
                    (l_simple, r_simple) => {
                        match (Interval::infer(&l_simple, i), Interval::infer(&r_simple, i)) {
                            //check if intervals have no intersection
                            (Interval(a, b), Interval(c, d)) if b < c => {
                                SymExpression::Literal(Literal::Boolean(true))
                            }
                            (Interval(a, b), Interval(c, d)) if d < a => {
                                SymExpression::Literal(Literal::Boolean(false))
                            }
                            _ => SymExpression::LT(Box::new(l_simple), Box::new(r_simple)),
                        }
                    }
                }
            }
            SymExpression::Plus(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (
                        SymExpression::Literal(Literal::Integer(l_lit)),
                        SymExpression::Literal(Literal::Integer(r_lit)),
                    ) => SymExpression::Literal(Literal::Integer(l_lit + r_lit)),
                    (l_simple, r_simple) => {
                        SymExpression::Plus(Box::new(l_simple), Box::new(r_simple))
                    }
                }
            }
            SymExpression::Minus(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (
                        SymExpression::Literal(Literal::Integer(l_lit)),
                        SymExpression::Literal(Literal::Integer(r_lit)),
                    ) => SymExpression::Literal(Literal::Integer(l_lit - r_lit)),
                    (l_simple, r_simple) => {
                        SymExpression::Minus(Box::new(l_simple), Box::new(r_simple))
                    }
                }
            }
            SymExpression::Multiply(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (
                        SymExpression::Literal(Literal::Integer(l_lit)),
                        SymExpression::Literal(Literal::Integer(r_lit)),
                    ) => SymExpression::Literal(Literal::Integer(l_lit * r_lit)),
                    (l_simple, r_simple) => {
                        SymExpression::Multiply(Box::new(l_simple), Box::new(r_simple))
                    }
                }
            }
            SymExpression::Divide(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (
                        SymExpression::Literal(Literal::Integer(l_lit)),
                        SymExpression::Literal(Literal::Integer(r_lit)),
                    ) => SymExpression::Literal(Literal::Integer(l_lit / r_lit)),
                    (l_simple, r_simple) => {
                        SymExpression::Divide(Box::new(l_simple), Box::new(r_simple))
                    }
                }
            }
            SymExpression::Mod(l, r) => {
                match (l.clone().eval(i, eval_refs), r.clone().eval(i, eval_refs)) {
                    (
                        SymExpression::Literal(Literal::Integer(l_lit)),
                        SymExpression::Literal(Literal::Integer(r_lit)),
                    ) => SymExpression::Literal(Literal::Integer(l_lit % r_lit)),
                    (l_simple, r_simple) => {
                        SymExpression::Mod(Box::new(l_simple), Box::new(r_simple))
                    }
                }
            }
            SymExpression::Not(expr) => match expr.clone().eval(i, eval_refs) {
                SymExpression::Not(inner_expr) => inner_expr.clone().eval(i, eval_refs),
                SymExpression::Literal(Literal::Boolean(b)) => {
                    SymExpression::Literal(Literal::Boolean(!b))
                }
                simple_expr => SymExpression::Not(Box::new(simple_expr)),
            },
            SymExpression::Negative(expr) => match expr.clone().eval(i, eval_refs) {
                SymExpression::Negative(inner_expr) => inner_expr.clone().eval(i, eval_refs),
                SymExpression::Literal(Literal::Integer(n)) => {
                    SymExpression::Literal(Literal::Integer(-n))
                }
                expr => SymExpression::Negative(Box::new(expr)),
            },
            SymExpression::Literal(_) => self,
            //evaluate point interval
            SymExpression::FreeVariable(_, x) => match Interval::infer(&self, i) {
                Interval(a, b) if a == b && a.is_finite() => {
                    SymExpression::Literal(Literal::Integer(a.finite().unwrap()))
                }
                _ => self,
            },
            SymExpression::Reference(r) => match (r, eval_refs) {
                (Reference::Evaluated(_), _) => self,
                (Reference::Lazy { r, class }, Some(er)) if er.contains(r) => {
                    SymExpression::Reference(Reference::Evaluated(*r))
                }
                _ => self,
            },
            SymExpression::Forall(_) => self,
            SymExpression::Exists(_, _, _, _) => self,
            SymExpression::Uninitialized => panic_with_diagnostics(
                "There is an uninitialized value in an expression. Did you declare all variables?",
                &self,
            ),
        }
    }
}

#[derive(Clone)]
pub struct Forall {
    r: Reference,
    index: Identifier,
    value: Identifier,
    inner_expr: Expression,
    captured_memory: SymMemory,
}

impl Forall {
    fn new(
        r: Reference,
        index: Identifier,
        value: Identifier,
        inner_expr: Expression,
        captured_memory: SymMemory,
    ) -> Self {
        Self {
            r,
            index,
            value,
            inner_expr,
            captured_memory,
        }
    }

    /// destructs a `Expression::forall(arr, index, value)` statement using the following algorithm:
    /// ``` ignore
    /// // asserts expression holds for all values in array
    /// let mut c = true
    /// for (i, v) in arr { c = c && [i |->index, v |-> value] in expr}     /// // substitute (i,v) into expression
    ///
    /// // asserts expression holds for all values > 0 and < #arr that are not in symbolic array
    /// o = true
    /// for (i,v) in arr {o = index != i && o}
    /// e = (0 < index && o && index < #arr && value == 0 ==> expr
    ///
    /// return c && e
    /// ```
    pub fn construct(&self, current_memory: &SymMemory) -> SymExpression {
        let (r, index, value, inner_expr, captured_memory) = (
            &self.r,
            &self.index,
            &self.value,
            &self.inner_expr,
            &self.captured_memory,
        );
        let index_id = SymExpression::FreeVariable(SymType::Int, index.clone());

        let (sym_ty, arr, size_expr, _) = match current_memory.heap_get_unsafe(r) {
            ReferenceValue::Array(a) => a,
            _ => todo!(),
        };

        // foreach (i, v) pair in arr:
        // - C = for each[i |-> index, v |-> value]expr && C
        // - O = index != i && O
        let mut c = SymExpression::Literal(Literal::Boolean(true));
        let mut o = SymExpression::Literal(Literal::Boolean(true));
        for (i, v) in arr.into_iter() {
            // we insert index and value substitutions into stack
            // and build a new inner_expression with said substitutions
            let mut extended_memory = captured_memory.clone();
            extended_memory.stack_insert(index.clone(), i.clone());
            extended_memory.stack_insert(value.clone(), v.clone());

            c = SymExpression::And(
                Box::new(c),
                Box::new(SymExpression::new(&extended_memory, inner_expr.clone())),
            );

            let ne = SymExpression::NE(Box::new(index_id.clone()), Box::new(i.clone()));
            o = SymExpression::And(Box::new(ne), Box::new(o));
        }

        // E = index >= 0 && O && index < #arr ==> expr
        let i_geq_0 = SymExpression::GEQ(
            Box::new(index_id.clone()),
            Box::new(SymExpression::Literal(Literal::Integer(0))),
        );

        // get size from heap and make i < size expression
        let size = match current_memory.heap_get_unsafe(r) {
            ReferenceValue::Array((_, _, size, _)) => size,
            _ => todo!(),
        };

        let i_lt_size = SymExpression::LT(Box::new(index_id.clone()), Box::new(size.clone()));

        // build inner expression with index and value as freevars
        let mut extended_memory = captured_memory.clone();
        extended_memory.stack_insert(
            index.clone(),
            SymExpression::FreeVariable(SymType::Int, index.clone()),
        );
        extended_memory.stack_insert(
            value.clone(),
            SymExpression::FreeVariable(sym_ty.clone(), value.clone()),
        );
        let inner_expr = SymExpression::new(&extended_memory, inner_expr.clone());

        let e = SymExpression::Implies(
            Box::new(SymExpression::And(
                Box::new(i_geq_0),
                Box::new(SymExpression::And(Box::new(o), Box::new(i_lt_size))),
            )),
            Box::new(inner_expr),
        );

        SymExpression::And(Box::new(c), Box::new(e))
    }
}

/// Intermediate type use to implement custom `Hash` for Expression
/// while also using the default hasher for the 'base values'
#[derive(Hash)]
enum HashExpression {
    Plus(Vec<u64>),
    Multiply(Vec<u64>),
    Divide(u64, u64),
    Mod(u64, u64),
    Negative(u64),
    FreeVariable(SymType, Identifier),
    Reference(Reference),
    Forall(Reference, Identifier, Identifier, SymExpression),
    Exists(Reference, Identifier, Identifier, SymExpression),
    Implies(u64, u64),
    And(Vec<u64>),
    Or(Vec<u64>),
    EQ(u64, u64),
    NE(u64, u64),
    LT(u64, u64),
    GT(u64, u64),
    GEQ(u64, u64),
    LEQ(u64, u64),
    Not(u64),
    LazyReference(u64),
    Uninitialized,
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
fn map_hash<T: Hash>(v: &Vec<T>) -> Vec<u64> {
    v.into_iter()
        .map(|t| calculate_hash(&t))
        .collect::<Vec<u64>>()
}

impl Hash for SymExpression {
    fn hash<H: Hasher>(&self, state: &mut H) {
        fn collect_sum(expr: SymExpression) -> Vec<SymExpression> {
            match expr {
                SymExpression::Plus(l_expr, r_expr) => {
                    let mut sum = collect_sum(*l_expr);
                    sum.append(&mut collect_sum(*r_expr));
                    sum
                }
                SymExpression::Minus(l_expr, r_expr) => collect_sum(SymExpression::Plus(
                    l_expr,
                    Box::new(SymExpression::Negative(r_expr)),
                )),
                _ => vec![expr],
            }
        }

        fn collect_mult(expr: SymExpression) -> Vec<SymExpression> {
            match expr {
                SymExpression::Multiply(l_expr, r_expr) => {
                    let mut mult = collect_mult(*l_expr);
                    mult.append(&mut collect_mult(*r_expr));
                    mult
                }
                _ => vec![expr],
            }
        }
        fn collect_and(expr: SymExpression) -> Vec<SymExpression> {
            match expr {
                SymExpression::And(l_expr, r_expr) => {
                    let mut conj = collect_mult(*l_expr);
                    conj.append(&mut collect_mult(*r_expr));
                    conj
                }
                _ => vec![expr],
            }
        }
        fn collect_or(expr: SymExpression) -> Vec<SymExpression> {
            match expr {
                SymExpression::Or(l_expr, r_expr) => {
                    let mut or = collect_mult(*l_expr);
                    or.append(&mut collect_mult(*r_expr));
                    or
                }
                _ => vec![expr],
            }
        }

        match self {
            SymExpression::Plus(_, _) => {
                let sum = collect_sum(self.clone());
                let mut hashed_sum = map_hash(&sum);
                hashed_sum.sort();
                HashExpression::Plus(hashed_sum).hash(state)
            }
            SymExpression::Multiply(_, _) => {
                let mult = collect_mult(self.clone());
                let mut hashed_mult = map_hash(&mult);
                hashed_mult.sort();
                HashExpression::Multiply(hashed_mult).hash(state)
            }
            SymExpression::And(_, _) => {
                let and = collect_and(self.clone());
                let mut hashed_and = map_hash(&and);
                hashed_and.sort();
                HashExpression::And(hashed_and).hash(state)
            }
            SymExpression::Or(_, _) => {
                let or = collect_or(self.clone());
                let mut hashed_or = map_hash(&or);
                hashed_or.sort();
                HashExpression::Or(hashed_or).hash(state)
            }
            SymExpression::Minus(l_expr, r_expr) => SymExpression::Plus(
                l_expr.clone(),
                Box::new(SymExpression::Negative(r_expr.clone())),
            )
            .hash(state),
            SymExpression::Divide(l_expr, r_expr) => {
                HashExpression::Divide(calculate_hash(&*l_expr), calculate_hash(&*r_expr))
                    .hash(state)
            }
            SymExpression::Mod(l_expr, r_expr) => {
                HashExpression::Mod(calculate_hash(&*l_expr), calculate_hash(&*r_expr)).hash(state)
            }
            SymExpression::Negative(expr) => {
                HashExpression::Negative(calculate_hash(&*expr)).hash(state)
            }
            SymExpression::FreeVariable(ty, id) => {
                HashExpression::FreeVariable(ty.clone(), id.clone()).hash(state)
            }
            SymExpression::Reference(r) => HashExpression::Reference(r.clone()).hash(state),
            SymExpression::Literal(lit) => lit.hash(state),
            SymExpression::Implies(l_expr, r_expr) => {
                HashExpression::Implies(calculate_hash(&*l_expr), calculate_hash(&*r_expr))
                    .hash(state)
            }
            SymExpression::EQ(l_expr, r_expr) => {
                HashExpression::EQ(calculate_hash(&*l_expr), calculate_hash(&*r_expr)).hash(state)
            }
            SymExpression::NE(l_expr, r_expr) => {
                HashExpression::NE(calculate_hash(&*l_expr), calculate_hash(&*r_expr)).hash(state)
            }
            SymExpression::LT(l_expr, r_expr) => {
                HashExpression::LT(calculate_hash(&*l_expr), calculate_hash(&*r_expr)).hash(state)
            }
            SymExpression::GT(l_expr, r_expr) => {
                HashExpression::GT(calculate_hash(&*l_expr), calculate_hash(&*r_expr)).hash(state)
            }
            SymExpression::GEQ(l_expr, r_expr) => {
                HashExpression::GEQ(calculate_hash(&*l_expr), calculate_hash(&*r_expr)).hash(state)
            }
            SymExpression::LEQ(l_expr, r_expr) => {
                HashExpression::LEQ(calculate_hash(&*l_expr), calculate_hash(&*r_expr)).hash(state)
            }
            SymExpression::Not(expr) => HashExpression::Not(calculate_hash(&*expr)).hash(state),
            SymExpression::Uninitialized => HashExpression::Uninitialized.hash(state),
            SymExpression::Forall(_) => rand::random::<u64>().hash(state),
            SymExpression::Exists(_, _, _, _) => rand::random::<u64>().hash(state),
        }
    }
}

impl PartialEq for SymExpression {
    fn eq(&self, other: &Self) -> bool {
        calculate_hash(&self) == calculate_hash(other)
    }
}
impl Eq for SymExpression {}

impl fmt::Debug for PathConstraints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.combine_over_true())
    }
}
impl fmt::Debug for Forall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "forall {:?}, {}, {} : {:?}",
            self.r, self.index, self.value, self.inner_expr
        )
    }
}

impl fmt::Debug for SymExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymExpression::Implies(l_expr, r_expr) => write!(f, "({:?} ==> {:?})", l_expr, r_expr),
            SymExpression::Forall(forall) => {
                write!(f, "{:?}", forall)
            }
            SymExpression::Exists(arr, i, v, body) => {
                write!(f, "exists {}, {}, {} : {:?}", arr, i, v, body)
            }
            SymExpression::And(l_expr, r_expr) => write!(f, "({:?} && {:?})", l_expr, r_expr),
            SymExpression::Or(l_expr, r_expr) => write!(f, "({:?} || {:?})", l_expr, r_expr),
            SymExpression::EQ(l_expr, r_expr) => write!(f, "({:?} == {:?})", l_expr, r_expr),
            SymExpression::NE(l_expr, r_expr) => write!(f, "({:?} != {:?})", l_expr, r_expr),
            SymExpression::LT(l_expr, r_expr) => write!(f, "({:?} < {:?})", l_expr, r_expr),
            SymExpression::GT(l_expr, r_expr) => write!(f, "({:?} > {:?})", l_expr, r_expr),
            SymExpression::GEQ(l_expr, r_expr) => write!(f, "({:?} >= {:?})", l_expr, r_expr),
            SymExpression::LEQ(l_expr, r_expr) => write!(f, "({:?} <= {:?})", l_expr, r_expr),
            SymExpression::Plus(l_expr, r_expr) => write!(f, "({:?} + {:?})", l_expr, r_expr),
            SymExpression::Minus(l_expr, r_expr) => write!(f, "({:?} - {:?})", l_expr, r_expr),
            SymExpression::Multiply(l_expr, r_expr) => write!(f, "({:?} * {:?})", l_expr, r_expr),
            SymExpression::Divide(l_expr, r_expr) => write!(f, "({:?} / {:?})", l_expr, r_expr),
            SymExpression::Mod(l_expr, r_expr) => write!(f, "({:?} % {:?})", l_expr, r_expr),
            SymExpression::Negative(expr) => write!(f, "-{:?}", expr),
            SymExpression::Not(expr) => write!(f, "!{:?}", expr),
            SymExpression::Literal(Literal::Boolean(val)) => write!(f, "{:?}", val),
            SymExpression::Literal(Literal::Integer(val)) => write!(f, "{:?}", val),
            SymExpression::FreeVariable(_, fv) => write!(f, "{}", fv),
            SymExpression::Reference(r) => write!(f, "{:?}", r),
            SymExpression::Uninitialized => write!(f, "Uninitialized"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    lalrpop_mod!(pub parser);

    fn parse(i: &str) -> SymExpression {
        let expr = parser::VerificationExpressionParser::new()
            .parse(i)
            .unwrap();
        SymExpression::new(&SymMemory::new(Program(vec![])), expr)
    }

    #[test]
    fn sum_expr_equivalence() {
        let e1 = parse("1+2+3");
        let e2 = parse("3+2+1");
        let e3 = parse("2+3+1");
        assert!(calculate_hash(&e1) == calculate_hash(&e2));
        assert!(calculate_hash(&e1) == calculate_hash(&e3));
    }
    #[test]
    fn mult_expr_equivalence() {
        let e1 = parse("2*3*4+5-1");
        let e2 = parse("3*2*4+5-1");
        let e3 = parse("2*3*4+(-1)+5");

        assert!(calculate_hash(&e1) == calculate_hash(&e2));
        assert!(calculate_hash(&e1) == calculate_hash(&e3));
    }
    #[test]
    fn more_expr_equivalence() {
        let e1 = parse("(7*4+5/8%5)+3");
        let e2 = parse("3+(4*7+5/8%5)");
        assert!(calculate_hash(&e1) == calculate_hash(&e2));
    }
    #[test]
    fn int_comparison() {
        let e1 = parse("1");
        let e2 = parse("1");
        let e3 = parse("123");
        assert!(calculate_hash(&e1) == calculate_hash(&e2));
        assert!(calculate_hash(&e2) != calculate_hash(&e3));
    }
}
