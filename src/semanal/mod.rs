use std::collections::HashMap;
use std::fmt;

use crate::parser::*;

mod res_idents;
use res_idents::identifier_resolution_pass;

mod labels;
use labels::label_generation_pass;

mod loop_labeler;
use loop_labeler::loop_labeling_pass;

mod type_check;
use type_check::type_checking_pass;

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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Type {
    Int,
    FuncType(usize),
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

trait Visitor {
    fn visit_program(&mut self, program: &mut Program) -> <(), SemanticError> {
        walk_program(self, program)
    }

    fn visit_block(&mut self, block: &mut Block) {
        walk_block(self, block);
    }

    fn visit_var_decl(&mut self, var: &mut VarDeclaration) {
        walk_var_decl(self, var);
    }

    fn visit_func_decl(&mut self, func: &mut FuncDeclaration) {
        walk_func_decl(self, func);
    }

    fn visit_statement(&mut self, statement: &mut Statement) {
        walk_statement(self, statement);
    }

    fn visit_expression(&mut self, expression: &mut Expression) {
        walk_expression(self, expression);
    }
}



fn walk_block(v: &mut impl Visitor, block: &mut Block) {
    for item in &mut block.items {
        match item {
            BlockItem::D(Decl::VarDecl(d)) => v.visit_var_decl(d),
            BlockItem::D(Decl::FuncDecl(f)) => v.visit_func_decl(f),
            BlockItem::S(s) => v.visit_statement(s),
        }
    }
}

fn walk_var_decl(v: &mut impl Visitor, var: &mut VarDeclaration) {
    if let Some(exp) = &mut var.init {
        v.visit_expression(exp);
    }
}

fn walk_func_decl(v: &mut impl Visitor, func: &mut FuncDeclaration) {
    if let Some(blk) = &mut func.body {
        v.visit_block(blk);
    }
}

fn walk_statement(v: &mut impl Visitor, statement: &mut Statement) {
    match statement {
        Statement::Return(exp) | Statement::Expression(exp) |
        Statement::Case { expr: exp, lab:_ } => {
            v.visit_expression(exp);
        },
        Statement::If(exp, y, mn) => {
            v.visit_expression(exp);
            v.visit_statement(y);
            if let Some(n) = mn {
                v.visit_statement(n);
            }
        },
        Statement::While { cond, body, lab:_ } | Statement::DoWhile { body, cond, lab:_ } => {
            v.visit_expression(cond);
            v.visit_statement(body);
        },
        Statement::For { init, cond, post, body, lab:_ } => {
            match init {
                ForInit::InitDec(d) => v.visit_var_decl(d),
                ForInit::InitExp(Some(e)) => v.visit_expression(e),
                _ => {}
            }
            if let Some(c) = cond {
                v.visit_expression(c);
            }
            if let Some(p) = post {
                v.visit_expression(p);
            }
            v.visit_statement(body);
        },
        Statement::Label(_, s) => v.visit_statement(s),
        Statement::Switch { scrutinee, body, lab:_, cases } => {
            v.visit_expression(scrutinee);
            v.visit_statement(body);
            for c in cases {
                if let (Some(e), _) = c {
                    v.visit_expression(e);
                }
            }
        },
        _ => {}
    }
}
fn walk_expression(v: &mut impl Visitor, expression: &mut Expression) {
    match expression {
        Expression::Assignment(exp1, exp2) |
        Expression::Binary(_, exp1, exp2) => {
            v.visit_expression(exp1.as_mut());
            v.visit_expression(exp2.as_mut());
        },
        Expression::Conditional(exp1, exp2, exp3) => {
            v.visit_expression(exp1.as_mut());
            v.visit_expression(exp2.as_mut());
            v.visit_expression(exp3.as_mut());
        },
        Expression::Unary(_, exp) |
        Expression::PostfixIncrement(exp) | Expression::PrefixIncrement(exp) | 
        Expression::PostfixDecrement(exp) | Expression::PrefixDecrement(exp) => {
            v.visit_expression(exp.as_mut());
        },
        Expression::FunctionCall(_, args) => {
            for exp in args {
                v.visit_expression(exp);
            }
        },
        Expression::Var(_) |
        Expression::Constant(_) => {},
    }
}


impl std::error::Error for SemanticError {}

pub fn semantic_analysis(program: &mut Program, symbols: &mut HashMap<String, Symbol>) 
    -> Result<HashMap<String, (String, bool)>,SemanticError> {
    let map = identifier_resolution_pass(program)?;
    loop_labeling_pass(program)?;
    label_generation_pass(program)?;
    type_checking_pass(program, symbols)?;
    Ok(map)
}
