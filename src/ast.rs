use std::fmt;
/*
use non_empty_vec::NonEmpty;

//naming convention:
// - syntactical labels are taken as is from Stefan's thesis
// - each syntactical label's first symbol is transformed to uppercase (program -> Program)
// - labels with only 1 ´option´ are type aliases, , 1 < options are enums
*/

pub type Program = Vec<Class>;

pub type Class = (Identifier, Members);

pub type Members = Vec<Member>;

pub enum Member {
    //Constructor(Constructor),
    Method(Method),
    //Field(Field)
}

pub enum Method {
    Static(Methodcontent),
    Nonstatic(Methodcontent)
}

//TODO: add args hier
pub type Methodcontent = (Type, Identifier, Parameters, Statements);

pub type Parameters = Vec<Parameter>;

pub type Parameter = (Type, Identifier);

pub type Statements = Vec<Statement>;

#[derive(Clone)]
pub enum Statement {
    DeclareAssign(DeclareAssign),
    Declaration(Declaration),
    Assignment(Assignment),
    Call(Invocation),
    Ite(Ite),
    Return(Expression),
    Block(Box<Statements>),
    Assert(Expression),
    Assume(Expression),
    While(While)
}

// Todo: add to syntax & semantics in thesis
pub type DeclareAssign = (Type, Identifier, Rhs);

pub type Declaration = (Type, Identifier);

pub type While = (Expression, Box<Statement>);

#[derive(Clone)]
pub enum Type {
    Void,
    Int,
    Bool
}

pub type Assignment = (Lhs, Rhs);

#[derive(Debug, Clone)]
pub enum Lhs {
    Identifier(String),
}

#[derive(Debug, Clone)]
pub enum Rhs {
    Expression(Expression),
    Invocation(Invocation)
}

//TODO: add args hier
pub type Invocation = (Identifier, Identifier, Arguments);

pub type Arguments = Vec<Expression>;

pub type Ite = (Expression, Box<Statement>, Box<Statement>);

#[derive(Clone)]
pub enum Expression {

    //expression1
    Forall(Identifier, Box<Expression>),
    Exists(Identifier, Box<Expression>),
    
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
}

#[derive(Debug, Clone)]
pub enum Literal {
    Boolean(bool),
    Integer(i64),
}

pub type Identifier = String;

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        
        match self {
            Statement::DeclareAssign((t, id, rhs)) => write!(f, "{:?} {:?}", t, Statement::Assignment((Lhs::Identifier(id.to_string()), rhs.clone()))),
            Statement::Declaration((t, id)) => write!(f, "{:?} {};", t, id),
            Statement::Assignment((Lhs::Identifier(id), Rhs::Expression(expr))) => write!(f, "{} := {:?};", id, expr),
            Statement::Assignment((Lhs::Identifier(id), Rhs::Invocation((class, fun, args)))) => write!(f, "{} := {}.{}({:?});", id, class, fun, args),
            Statement::Call((l, r, args)) => write!(f, "{}.{}({:?});", l, r, args),
            Statement::Ite((cond, if_expr, else_expr)) => write!(f, "if ({:?}) then {:?} else {:?}", cond, if_expr, else_expr),
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
            Expression::Forall(id, body) => write!(f, "forall {} : {:?}", id, body),
            Expression::Exists(id, body) => write!(f, "exists {} : {:?}", id, body),
            Expression::Implies(l_expr, r_expr) => write!(f, "{:?} ==> {:?}", l_expr, r_expr),
            Expression::And(l_expr, r_expr) => write!(f, "{:?} && {:?}", l_expr, r_expr),
            Expression::Or(l_expr, r_expr) => write!(f, "{:?} || {:?}", l_expr, r_expr),
            Expression::EQ(l_expr, r_expr) => write!(f, "{:?} == {:?}", l_expr, r_expr),
            Expression::NE(l_expr, r_expr) => write!(f, "{:?} != {:?}", l_expr, r_expr),
            Expression::LT(l_expr, r_expr) => write!(f, "{:?} < {:?}", l_expr, r_expr),
            Expression::GT(l_expr, r_expr) => write!(f, "{:?} > {:?}", l_expr, r_expr),
            Expression::GEQ(l_expr, r_expr) => write!(f, "{:?} <= {:?}", l_expr, r_expr),
            Expression::LEQ(l_expr, r_expr) => write!(f, "{:?} >= {:?}", l_expr, r_expr),
            Expression::Plus(l_expr, r_expr) => write!(f, "{:?} + {:?}", l_expr, r_expr),
            Expression::Minus(l_expr, r_expr) => write!(f, "{:?} - {:?}", l_expr, r_expr),
            Expression::Multiply(l_expr, r_expr) => write!(f, "{:?} * {:?}", l_expr, r_expr),
            Expression::Divide(l_expr, r_expr) => write!(f, "{:?} / {:?}", l_expr, r_expr),
            Expression::Mod(l_expr, r_expr) => write!(f, "{:?} % {:?}", l_expr, r_expr),
            Expression::Negative(expr) => write!(f, "-{:?}", expr),
            Expression::Not(expr) => write!(f, "!{:?}", expr),
            Expression::Identifier(id) => write!(f, "{}", id),
            Expression::Literal(Literal::Boolean(val)) => write!(f, "{:?}", val),
            Expression::Literal(Literal::Integer(val)) => write!(f, "{:?}", val),
        }   

    }
}
impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        
        match self {
            Type::Void => write!(f, "void"),
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
        }   

    }
}