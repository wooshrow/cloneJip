use uuid::Uuid;

use crate::shared::panic_with_diagnostics;
use std::fmt;
use std::hash::{Hash};
/*
use non_empty_vec::NonEmpty;

//naming convention:
// - syntactical labels are taken as is from Stefan's thesis
// - each syntactical label's first symbol is transformed to uppercase (program -> Program)
// - labels with only 1 ´option´ are type aliases, , 1 < options are enums
*/

#[derive(Clone)]
pub struct Program(pub Vec<Class>);

impl Program {
    pub fn get_class<'a>(&'a self, class_name: &str) -> &'a Class {
        match self.0.iter().find(|(id, _)| id == class_name) {
            Some(class) => return class,
            None => panic_with_diagnostics(&format!("Class {} doesn't exist", class_name), &()),
        }
    }

    pub fn get_methodcontent<'a>(
        &'a self,
        class_name: &Identifier,
        method_name: &Identifier,
    ) -> &'a Methodcontent {
        let class = self.get_class(class_name);

        for member in class.1.iter() {
            match member {
                Member::Method(method) => match method {
                    Method::Static(content @ (_, id, _, _, _)) => {
                        if id == method_name {
                            return &content;
                        }
                    }
                    Method::Nonstatic(content @ (_, id, _, _, _)) => {
                        if id == method_name {
                            return &content;
                        }
                    }
                },
                _ => (),
            }
        }
        panic_with_diagnostics(
            &format!("Static method {}.{} doesn't exist", class.0, method_name),
            &(),
        );
    }

    pub fn get_constructor<'a>(&'a self, class_name: &str) -> &'a Constructor {
        let class = self.get_class(class_name);

        for m in class.1.iter() {
            match m {
                Member::Constructor(c) => return &c,
                _ => continue,
            }
        }
        panic_with_diagnostics(
            &format!("Class {} does not have a constructor", class_name),
            &(),
        );
    }
}

/// `(class_name, members)`
pub type Class = (Identifier, Members);

pub type Members = Vec<Member>;

#[derive(Clone, Debug)]
pub enum Member {
    Constructor(Constructor),
    Method(Method),
    Field(Field),
}

pub type Constructor = (Identifier, Parameters, Specifications, Statements);

#[derive(Clone, Debug)]
pub enum Method {
    Static(Methodcontent),
    Nonstatic(Methodcontent),
}

//TODO: add args hier
pub type Methodcontent = (Type, Identifier, Parameters, Specifications, Statements);

pub type Parameters = Vec<Parameter>;

pub type Parameter = (Type, Identifier);

pub type Specifications = Vec<Specification>;

#[derive(Clone, Debug)]
pub enum Specification {
    Requires(Expression),
    Ensures(Expression),
}

pub type Field = (Type, Identifier);

pub type Statements = Vec<Statement>;

#[derive(Clone)]
pub enum Statement {
    DeclareAssign(DeclareAssign),
    Declaration(Declaration),
    Assignment(Assignment),
    Call(Invocation),
    Skip(Skip),
    Ite(Ite),
    Return(Expression),
    Block(Box<Statements>),
    Assert(Expression),
    Assume(Expression),
    While(While),
}

// Todo: add to syntax & semantics in thesis
pub type DeclareAssign = (Type, Identifier, Rhs);

pub type Declaration = (Type, Identifier);

pub type While = (Expression, Box<Statement>);

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Int,
    Bool,
    ClassType(Identifier),
    ArrayType(Box<Type>),
}

pub type Assignment = (Lhs, Rhs);

#[derive(Clone)]
pub enum Lhs {
    Identifier(String),
    AccessField(Identifier, Identifier),
    AccessArray(Identifier, Expression),
}

#[derive(Clone)]
pub enum Rhs {
    Expression(Expression),
    AccessField(Identifier, Identifier),
    AccessArray(Identifier, Expression),
    Invocation(Invocation),
    Newobject(Identifier, Arguments),
    NewArray(Type, Expression),
}

//TODO: add args hier
pub type Invocation = (Identifier, Identifier, Arguments);

pub type Arguments = Vec<Expression>;

#[derive(Clone)]
pub struct Skip;

pub type Ite = (Expression, Box<Statement>, Box<Statement>);

#[derive(Clone)]
pub enum Expression {
    //expression1
    ///(forall arr, index, value : expression) -> for all index value pairs in given array the expression holds
    Forall(Identifier, Identifier, Identifier, Box<Expression>),
    ///(exists arr, index, value : expression) -> for all index value pairs in given array the expression holds
    Exists(Identifier, Identifier, Identifier, Box<Expression>),

    //expression2
    Implies(Box<Expression>, Box<Expression>),

    //expression3
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),

    //expression4
    EQ(Box<Expression>, Box<Expression>),
    NE(Box<Expression>, Box<Expression>),

    //Expression5
    LT(Box<Expression>, Box<Expression>),
    GT(Box<Expression>, Box<Expression>),
    GEQ(Box<Expression>, Box<Expression>),
    LEQ(Box<Expression>, Box<Expression>),

    //Expression6
    Plus(Box<Expression>, Box<Expression>),
    Minus(Box<Expression>, Box<Expression>),

    //Expression7
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Mod(Box<Expression>, Box<Expression>),

    //Expression8
    Negative(Box<Expression>),
    Not(Box<Expression>),

    //expression9
    Identifier(Identifier),
    Literal(Literal),
    ArrLength(Identifier),

    //free var flows in the program from main() args
    FreeVariable(Type, Identifier),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    Boolean(bool),
    Integer(i64),
    Ref((Type, Uuid))
}

pub type Identifier = String;

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::DeclareAssign((t, id, rhs)) => write!(
                f,
                "{:?} {:?}",
                t,
                Statement::Assignment((Lhs::Identifier(id.to_string()), rhs.clone()))
            ),
            Statement::Declaration((t, id)) => write!(f, "{:?} {};", t, id),
            Statement::Assignment((lhs, rhs)) => write!(f, "{:?} := {:?}", lhs, rhs),
            Statement::Call((l, r, args)) => write!(f, "{}.{}({:?});", l, r, args),
            Statement::Skip(Skip) => write!(f, "skip "),
            Statement::Ite((cond, if_expr, else_expr)) => {
                write!(f, "if ({:?}) then {:?} else {:?}", cond, if_expr, else_expr)
            }
            Statement::Return(expr) => write!(f, "return {:?};", expr),
            Statement::Block(stmts) => write!(f, "{{ {:?} }}", stmts),
            Statement::Assert(expr) => write!(f, "assert {:?};", expr),
            Statement::Assume(expr) => write!(f, "assume {:?};", expr),
            Statement::While((cond, body)) => write!(f, "while ({:?}) {:?}", cond, body),
        }
    }
}
impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Forall(arr, i, v, body) => write!(f, "forall {}, {}, {} : {:?}", arr, i, v, body),
            Expression::Exists(arr, i, v, body) => write!(f, "exists {}, {}, {} : {:?}", arr, i, v, body),
            Expression::Implies(l_expr, r_expr) => write!(f, "({:?} ==> {:?})", l_expr, r_expr),
            Expression::And(l_expr, r_expr) => write!(f, "({:?} && {:?})", l_expr, r_expr),
            Expression::Or(l_expr, r_expr) => write!(f, "({:?} || {:?})", l_expr, r_expr),
            Expression::EQ(l_expr, r_expr) => write!(f, "({:?} == {:?})", l_expr, r_expr),
            Expression::NE(l_expr, r_expr) => write!(f, "({:?} != {:?})", l_expr, r_expr),
            Expression::LT(l_expr, r_expr) => write!(f, "({:?} < {:?})", l_expr, r_expr),
            Expression::GT(l_expr, r_expr) => write!(f, "({:?} > {:?})", l_expr, r_expr),
            Expression::GEQ(l_expr, r_expr) => write!(f, "({:?} >= {:?})", l_expr, r_expr),
            Expression::LEQ(l_expr, r_expr) => write!(f, "({:?} <= {:?})", l_expr, r_expr),
            Expression::Plus(l_expr, r_expr) => write!(f, "({:?} + {:?})", l_expr, r_expr),
            Expression::Minus(l_expr, r_expr) => write!(f, "({:?} - {:?})", l_expr, r_expr),
            Expression::Multiply(l_expr, r_expr) => write!(f, "({:?} * {:?})", l_expr, r_expr),
            Expression::Divide(l_expr, r_expr) => write!(f, "({:?} / {:?})", l_expr, r_expr),
            Expression::Mod(l_expr, r_expr) => write!(f, "({:?} % {:?})", l_expr, r_expr),
            Expression::Negative(expr) => write!(f, "-{:?}", expr),
            Expression::Not(expr) => write!(f, "!{:?}", expr),
            Expression::Identifier(id) => write!(f, "{}", id),
            Expression::FreeVariable(_, id) => write!(f, "{}", id),
            Expression::Literal(Literal::Boolean(val)) => write!(f, "{:?}", val),
            Expression::Literal(Literal::Integer(val)) => write!(f, "{:?}", val),
            Expression::Literal(Literal::Ref(r)) => write!(f, "Ref{:?}", r),
            Expression::ArrLength(id) => write!(f, "#{}", id),
        }
    }
}
impl fmt::Debug for Lhs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lhs::AccessField(class, field) => write!(f, "{}.{}", class, field),
            Lhs::AccessArray(id, index) => write!(f, "{}[{:?}]", id, index),
            Lhs::Identifier(id) => write!(f, "{}", id),
        }
    }
}
impl fmt::Debug for Rhs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rhs::Expression(expr) => write!(f, "{:?};", expr),
            Rhs::AccessField(class, field) => write!(f, "{}.{};", class, field),
            Rhs::AccessArray(class, index) => write!(f, "{}.[{:?}];", class, index),
            Rhs::Invocation((class, fun, args)) => write!(f, " {}.{}({:?});", class, fun, args),
            Rhs::Newobject(class, args) => write!(f, "new {}({:?});", class, args),
            Rhs::NewArray(ty, size) => write!(f, "new {:?}[{:?}]", ty, size),
        }
    }
}
impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => write!(f, "void"),
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::ClassType(name) => write!(f, "{}", name),
            Type::ArrayType(ty) => write!(f, "{:?}[]", ty),
        }
    }
}
