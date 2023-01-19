use rustc_hash::FxHashMap;
use petgraph::graph::{Graph, NodeIndex};
use crate::{ast::*, shared::Scope};

pub enum Node {
    EnteringMain(Parameters),
    Statement(Statement),
    /// classname, methodname and list of expressions we assign to parameters
    EnterRoutine(Routine),
    /// classname, methodname and variable name we assign retval to
    LeaveRoutine(Routine),
    End,
}

/// describes the actions we have to perform upon traversing edge
#[derive(Clone)]
pub enum Action {
    EnterScope {
        to: Scope,
    },
    // declare retval with correct type in current scope
    DeclareRetval {
        ty: Type,
    },
    AssignArgs {
        params: Parameters,
        args: Vec<Expression>,
    },
    /// copy ref of object method is called on to 'this'
    DeclareThis {
        obj: Lhs,
        class: Identifier,
    },
    /// Initialise object of class on heap and make lhs a reference to object
    InitObj {
        from: Class,
        to: Lhs,
    },
    /// lifts value of retval 1 scope higher
    LiftRetval,
    LeaveScope {
        from: Scope,
    },
}

pub type Edge = Vec<Action>;

pub type CFG = Graph<Node, Edge>;

/// Maps the collection type routine (covering all methods & constructors) to a tuple of start- and endnode for the subgraph of that routine
pub type FunEnv = FxHashMap<Routine, (NodeIndex, NodeIndex)>;

/// Abstraction type over methods & constructors
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Routine {
    Method {
        class: Identifier,
        method: Identifier,
    },
    Constructor {
        class: Identifier,
    },
}

/// Map objectname to the class of said object e.g. c.increment() can only be performed if we know where to find the increment function
pub type TypeStack = Vec<FxHashMap<Identifier, Class>>;

/// Given a generated subgraph, this struct denotes the last node & which edge comes from it should we want to extend it
#[derive(Clone)]
pub struct Start {
    pub node: NodeIndex,
    pub edge: Edge,
}