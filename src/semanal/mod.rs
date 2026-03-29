use std::collections::HashMap;
use std::fmt;

use crate::parser::*;

mod visitor_trait;

mod identifier_resolver;
use identifier_resolver::identifier_resolution_pass;

mod switch_collector;
use switch_collector::switch_collection_pass;

mod loop_labeler;
use loop_labeler::loop_labeling_pass;

mod type_checker;
use type_checker::type_checking_pass;

#[derive(Debug)]
pub enum SemanticError {
    UseBeforeDeclaration(String),
    InvalidLValue,
    DoubleDeclaration(String),
    NestedFunctionDefinition(String),
    UndeclaredLabel(String),
    DuplicateLabel(String),
    BreakOutsideLoopOrSwitch,
    CaseOutsideSwitch,
    ContOutsideLoop,
    NonConstantCase,
    DuplicateCase,
    DuplicateDefault,
    DecInCase,
    IncompatibleFuncDeclaration(String),
    FuncCalledWithWrongNumArgs(String),
    VarCalledAsFunc(String),
    FuncUsedAsVar(String),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemanticError::UseBeforeDeclaration(n) => write!(f, "Used {} before it was declared", n),
            SemanticError::InvalidLValue => write!(f, "Invalid lvalue"),
            SemanticError::DoubleDeclaration(n) => write!(f, "Duplicate declaration of {}", n),
            SemanticError::NestedFunctionDefinition(n) => write!(f, "Nested declaration of {}", n),
            SemanticError::UndeclaredLabel(n) => write!(f, "Undeclared label {}", n),
            SemanticError::DuplicateLabel(n) => write!(f, "Duplicate label {}", n),
            SemanticError::BreakOutsideLoopOrSwitch => write!(f, "Break outside loop/switch"),
            SemanticError::CaseOutsideSwitch => write!(f, "Case outside switch"),
            SemanticError::ContOutsideLoop => write!(f, "Cont outside loop"),
            SemanticError::NonConstantCase => write!(f, "Non constant case"),
            SemanticError::DuplicateCase => write!(f, "Duplicate case"),
            SemanticError::DuplicateDefault => write!(f, "Duplicate label"),
            SemanticError::DecInCase => write!(f, "Dec in case"),
            SemanticError::IncompatibleFuncDeclaration(n) => write!(f, "Incompatible Function Declaration {}", n),
            SemanticError::FuncCalledWithWrongNumArgs(n) => write!(f, "Function {} called with wrong number of args", n),
            SemanticError::VarCalledAsFunc(n) => write!(f, "Variable {} called as a function", n),
            SemanticError::FuncUsedAsVar(n) => write!(f, "Function {} used as a variable", n),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub ident: String,
    pub datatype: Type,
    pub linkage: Option<bool>,
    pub stack_size: Option<i32>,
}

impl Symbol {
    fn new_func(ident: String, ftype: Type, linkage: bool) -> Symbol {
        Symbol {
            ident: ident,
            datatype: ftype,
            linkage: Some(linkage),
            stack_size: None,
        }
    }

    fn new_var(ident: String, vtype: Type) -> Symbol {
        Symbol { ident, datatype: vtype, linkage: None, stack_size: None }
    }
}

impl std::error::Error for SemanticError {}

pub fn semantic_analysis(program: &mut Program, symbols: &mut HashMap<String, Symbol>) 
    -> Result<HashMap<String, (String, bool)>,SemanticError> {
    let map = identifier_resolution_pass(program)?;
    loop_labeling_pass(program)?;
    switch_collection_pass(program)?;
    type_checking_pass(program, symbols)?;
    Ok(map)
}
