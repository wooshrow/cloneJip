//! Transforms a program path to a logical formula and test satisfiability using theorem prover Z3
//!

extern crate z3;

use rustc_hash::FxHashMap;
use std::fmt;
use std::rc::Rc;
use uuid::Uuid;
use z3::ast::{Ast, Bool, Dynamic, Int};
use z3::{ast, Context, SatResult, Solver};

use crate::ast::*;
use crate::shared::{panic_with_diagnostics, Error, Scope};

//----------------------//
// Symbolic expressions //
//----------------------//
pub type Reference = Uuid;

#[derive(Debug, Clone)]
pub enum SymValue {
    Free(String),
    Uninitialized,
    Expr(Expression)
}

#[derive(Debug, Clone)]
pub enum SymExpression {
    Int(SymValue),
    Bool(SymValue),
    Ref((Type, Reference)),
}

/// Consists of `identifier` (= classname) and a hashmap describing it's fields
pub type Object = (
    Identifier,
    FxHashMap<Identifier, (Type, SymExpression)>,
);

pub type _Array = (Type, Vec<SymExpression>);

#[derive(Debug, Clone)]
pub enum ReferenceValue {
    Object(Object),
    //Array(Array),
    /// Takes classname as input
    UninitializedObj(Identifier),
}

//-----------------//
// Symbolic memory //
//-----------------//

#[derive(Debug, Clone)]
struct Frame<'a> {
    pub scope: Scope,
    pub env: FxHashMap<&'a Identifier, SymExpression>,
}

type SymStack<'a> = Vec<Frame<'a>>;

type SymHeap = FxHashMap<Reference, ReferenceValue>;

#[derive(Clone)]
pub struct SymMemory<'ctx> {
    program: Program,
    ctx: &'ctx Context,
    stack: SymStack<'ctx>,
    heap: SymHeap,
}

impl<'ctx> SymMemory<'ctx> {
    pub fn new(p: Program, ctx: &'ctx Context) -> Self {
        SymMemory {
            program: p,
            ctx,
            stack: vec![Frame {
                scope: Scope { id: None },
                env: FxHashMap::default(),
            }],
            heap: FxHashMap::default(),
        }
    }
}

impl<'a> SymMemory<'a> {
    /// Insert mapping `Identifier |-> SymbolicExpression` in top most frame of stack
    pub fn stack_insert(&mut self, id: &'a Identifier, var: SymExpression) -> () {
        if let Some(s) = self.stack.last_mut() {
            s.env.insert(id, var);
        }
    }

    /// Insert mapping `Identifier |-> SymbolicExpression` in frame below top most frame of stack
    pub fn stack_insert_below(&mut self, id: &'a Identifier, var: SymExpression) -> () {
        let below_index = self.stack.len() - 2;
        match self.stack.get_mut(below_index) {
            Some(frame) => {
                frame.env.insert(id, var);
            }
            _ => (),
        }
    }

    /// Iterate over frames from stack returning the first variable with given `id`
    pub fn stack_get(&self, id: &'a Identifier) -> Option<SymExpression> {
        if id == "null" {
            return Some(SymExpression::Ref((Type::Void, Uuid::nil())));
        };

        for s in self.stack.iter().rev() {
            match s.env.get(&id) {
                Some(var) => return Some(var.clone()),
                None => (),
            }
        }
        return None;
    }

    // Push new frame with given scope
    pub fn stack_push(&mut self, scope: Scope) -> () {
        self.stack.push(Frame {
            scope: scope.clone(),
            env: FxHashMap::default(),
        })
    }
    pub fn stack_pop(&mut self) -> () {
        self.stack.pop();
    }

    /// Returns scope indexed from the top of the stack `get_scope(0) == top_scope`
    pub fn get_scope(&self, index: usize) -> &Scope {
        let position = self.stack.len() - (1 + index);
        match self.stack.get(position) {
            Some(frame) => &frame.scope,
            None => {
                panic_with_diagnostics(&format!("No scope exists at position {}", position), &self)
            }
        }
    }

    /// Insert mapping `Reference |-> ReferenceValue` into heap
    pub fn heap_insert(&mut self, r: Reference, v: ReferenceValue) -> () {
        self.heap.insert(r, v);
    }

    /// Get symbolic value of the object's field, panics if something goes wrong
    pub fn heap_get_field(&mut self, obj_name: &String, field_name: &String) -> SymExpression {
        match self.stack_get(obj_name) {
            Some(SymExpression::Ref((_, r))) => {
                let ref_val = self.heap.get(&r).map(|s| s.clone());
                match ref_val {
                    Some(ReferenceValue::Object((_, fields))) => match fields.get(field_name) {
                        Some((_, expr)) => expr.clone(),
                        None => panic_with_diagnostics(
                            &format!("Field {} does not exist on {}", field_name, obj_name),
                            &self,
                        ),
                    },

                    Some(ReferenceValue::UninitializedObj(class_name)) => {
                        let mut new_fields = FxHashMap::default();

                        // initialize newObj lazily
                        let members = self.program.get_class(&class_name).1.clone();
                        for member in members {
                            if let Member::Field((ty, field_name)) = member {
                                match ty {
                                    Type::Int => {
                                        new_fields.insert(
                                            field_name.clone(),
                                            (Type::Int, fresh_int(self.ctx, field_name.clone())),
                                        );
                                    }
                                    Type::Bool => {
                                        new_fields.insert(
                                            field_name.clone(),
                                            (Type::Bool, fresh_bool(self.ctx, field_name.clone())),
                                        );
                                    }
                                    Type::Classtype(n) => {
                                        // add new unitializedObject to the heap
                                        let next_r = Uuid::new_v4();
                                        self.heap_insert(
                                            next_r,
                                            ReferenceValue::UninitializedObj(n.clone()),
                                        );

                                        // insert unitialized object in the object's fields
                                        new_fields.insert(
                                            field_name.clone(),
                                            (
                                                Type::Classtype(n.clone()),
                                                SymExpression::Ref((
                                                    Type::Classtype(n.clone()),
                                                    next_r,
                                                )),
                                            ),
                                        );
                                    }
                                    Type::Void => {
                                        panic_with_diagnostics("Panic should never trigger", &self)
                                    }
                                }
                            }
                        }

                        // push new object under original reference to heap and recurse
                        let new_obj = ReferenceValue::Object((class_name.clone(), new_fields));
                        self.heap_insert(r, new_obj);
                        self.heap_get_field(obj_name, field_name)
                    }

                    _ => panic_with_diagnostics(
                        &format!("Reference of {} not found on heap", obj_name),
                        &self,
                    ),
                }
            }
            _ => panic_with_diagnostics(&format!("{} is not a reference", obj_name), &self),
        }
    }
    /// Update symbolic value of the object's field, panics if something goes wrong
    pub fn heap_update_field(
        &mut self,
        obj_name: &String,
        field_name: &'a String,
        var: SymExpression,
    ) -> () {
        match self.stack_get(obj_name) {
            Some(SymExpression::Ref((_, r))) => match self.heap.get_mut(&r) {
                Some(ReferenceValue::Object((_, fields))) => {
                    let ty = match fields.get(field_name) {
                        Some(field) => field,
                        None => panic_with_diagnostics(
                            &format!("Field {} does not exist on {}", field_name, obj_name),
                            &self,
                        ),
                    }
                    .0
                    .clone();
                    fields.insert(field_name.clone(), (ty, var));
                }
                _ => panic_with_diagnostics(
                    &format!(
                        "Reference of {} not found on heap while doing assignment '{}.{} := {:?}'",
                        obj_name, obj_name, field_name, var
                    ),
                    &self,
                ),
            },
            _ => panic_with_diagnostics(&format!("{} is not a reference", obj_name), &self),
        }
    }
}

impl fmt::Debug for SymMemory<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
State of Sym-Stack:
{:?}

State of Sym-Heap:
{:?}",
            self.stack, self.heap
        )
    }
}

//--------------//
// z3 bindings //
//-------------//
#[derive(Clone)]
pub enum PathConstraint<'a> {
    Assume(Bool<'a>),
    Assert(Bool<'a>),
}

impl fmt::Debug for PathConstraint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathConstraint::Assume(pc) => write!(f, "{}", pc),
            PathConstraint::Assert(pc) => write!(f, "{}", pc),
        }
    }
}

// todo: inline this
pub fn fresh_int<'ctx>(ctx: &'ctx Context, id: String) -> SymExpression {
    SymExpression::Int(SymValue::Free(id))
    //return SymbolicExpression::Int(Int::new_const(&ctx, id));
}

// todo: inline this
pub fn fresh_bool<'ctx>(ctx: &'ctx Context, id: String) -> SymExpression {
    SymExpression::Bool(SymValue::Free(id))
    //return SymbolicExpression::Bool(Bool::new_const(&ctx, id));
}

/// Combine the constraints in reversed order and check correctness
/// `solve_constraints(ctx, vec![assume x, assert y, assume z] = x -> (y && z)`
pub fn check_path<'ctx>(
    ctx: &'ctx Context,
    path_constraints: &Vec<PathConstraint<'ctx>>,
) -> Result<(), Error> {
    let mut constraints = Bool::from_bool(ctx, true);

    //reverse loop and combine constraints
    for constraint in path_constraints.iter().rev() {
        match constraint {
            PathConstraint::Assert(c) => constraints = Bool::and(&ctx, &[&c, &constraints]),
            PathConstraint::Assume(c) => constraints = Bool::implies(&c, &constraints),
        }
    }

    //println!("{}", constraints.not());

    let solver = Solver::new(&ctx);
    solver.assert(&constraints.not());
    let result = solver.check();
    let model = solver.get_model();

    //println!("{:?}", model);

    match (result, model) {
        (SatResult::Unsat, _) => return Ok(()),
        (SatResult::Sat, Some(model)) => {
            return Err(Error::Verification(format!(
                "Following counter-example was found:\n{:?}",
                model
            )));
        }
        _ => {
            return Err(Error::Verification(
                "Huh, verification gave an unkown result".to_string(),
            ))
        }
    };
}

pub fn expr_to_int<'ctx>(
    ctx: &'ctx Context,
    env: &SymMemory<'ctx>,
    expr: &'ctx Expression,
) -> Int<'ctx> {
    return unwrap_as_int(expr_to_dynamic(&ctx, Rc::new(env), expr));
}

pub fn expr_to_bool<'ctx>(
    ctx: &'ctx Context,
    env: &SymMemory<'ctx>,
    expr: &'ctx Expression,
) -> Bool<'ctx> {
    return unwrap_as_bool(expr_to_dynamic(&ctx, Rc::new(env), expr));
}

fn expr_to_dynamic<'ctx, 'a>(
    ctx: &'ctx Context,
    sym_memory: Rc<&SymMemory<'a>>,
    expr: &'a Expression,
) -> Dynamic<'ctx> {
    match expr {
        Expression::Exists(id, expr) => {
            let l = Int::fresh_const(ctx, id);
            let r = unwrap_as_bool(expr_to_dynamic(ctx, sym_memory, expr));

            return Dynamic::from(ast::exists_const(&ctx, &[&l], &[], &r));
        }
        Expression::And(l_expr, r_expr) => {
            let l = unwrap_as_bool(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_bool(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(Bool::and(ctx, &[&l, &r]));
        }
        Expression::Or(l_expr, r_expr) => {
            let l = unwrap_as_bool(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_bool(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(Bool::or(ctx, &[&l, &r]));
        }
        Expression::EQ(l_expr, r_expr) => {
            let l = expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr);
            let r = expr_to_dynamic(ctx, sym_memory, r_expr);

            return Dynamic::from(l._eq(&r));
        }
        Expression::NE(l_expr, r_expr) => {
            let l = expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr);
            let r = expr_to_dynamic(ctx, sym_memory, r_expr);

            return Dynamic::from(l._eq(&r).not());
        }
        Expression::LT(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(l.lt(&r));
        }
        Expression::GT(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(l.gt(&r));
        }
        Expression::GEQ(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(l.ge(&r));
        }
        Expression::LEQ(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(l.le(&r));
        }
        Expression::Plus(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(ast::Int::add(&ctx, &[&l, &r]));
        }
        Expression::Minus(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(ast::Int::sub(&ctx, &[&l, &r]));
        }
        Expression::Multiply(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(ast::Int::mul(&ctx, &[&l, &r]));
        }
        Expression::Divide(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(l.div(&r));
        }
        Expression::Mod(l_expr, r_expr) => {
            let l = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), l_expr));
            let r = unwrap_as_int(expr_to_dynamic(ctx, sym_memory, r_expr));

            return Dynamic::from(l.modulo(&r));
        }
        Expression::Negative(expr) => {
            let e = unwrap_as_int(expr_to_dynamic(ctx, Rc::clone(&sym_memory), expr));

            return Dynamic::from(e.unary_minus());
        }
        Expression::Not(expr) => {
            let expr = unwrap_as_bool(expr_to_dynamic(ctx, sym_memory, expr));

            return Dynamic::from(expr.not());
        }
        Expression::Identifier(id) => todo!(),
        Expression::Literal(Literal::Integer(n)) => Dynamic::from(ast::Int::from_i64(ctx, *n)),
        Expression::Literal(Literal::Boolean(b)) => Dynamic::from(ast::Bool::from_bool(ctx, *b)),
        otherwise => {
            panic_with_diagnostics(
                &format!(
                    "Expressions of the form {:?} are not parseable to a z3 ast",
                    otherwise
                ),
                &sym_memory,
            );
        }
    }
}

fn unwrap_as_bool(d: Dynamic) -> Bool {
    match d.as_bool() {
        Some(b) => b,
        None => panic_with_diagnostics(&format!("{} is not of type Bool", d), &()),
    }
}

fn unwrap_as_int(d: Dynamic) -> Int {
    match d.as_int() {
        Some(b) => b,
        None => panic_with_diagnostics(&format!("{} is not of type Int", d), &()),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use z3::Config;

    #[test]
    fn test_solving() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let x = Int::new_const(&ctx, "x");
        let y = Int::new_const(&ctx, "y");
        let solver = Solver::new(&ctx);
        solver.assert(&x.gt(&y));
        assert_eq!(solver.check(), SatResult::Sat);
    }

    #[test]
    fn manual_max() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let x = ast::Real::new_const(&ctx, "x");
        let y = ast::Real::new_const(&ctx, "y");
        let z = ast::Real::new_const(&ctx, "z");
        let x_plus_y = ast::Real::add(&ctx, &[&x, &y]);
        let x_plus_z = ast::Real::add(&ctx, &[&x, &z]);
        let substitutions = &[(&y, &z)];
        assert!(x_plus_y.substitute(substitutions) == x_plus_z);
    }
    #[test]
    fn exist_example() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Solver::new(&ctx);

        let x = ast::Int::new_const(&ctx, "x");
        let one = ast::Int::from_i64(&ctx, 1);

        let exists: ast::Bool = ast::exists_const(
            &ctx,
            &[&x],
            &[],
            &x._eq(&one), // hier gaat expression in
        )
        .try_into()
        .unwrap();

        println!("{:?}", exists);

        solver.assert(&exists.not());
    }
}
