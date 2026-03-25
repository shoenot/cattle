use std::collections::HashMap;
use std::fmt;

use crate::parser::*;

#[derive(Debug)]
pub enum SemanticError {
    UseBeforeDeclaration(String),
    InvalidLValue,
    InvalidExpression,
    DoubleDeclaration,
    UndeclaredLabel(String),
    DuplicateLabel(String),
    LabelBeforeDeclaration(String),
    LabelWithoutStatement,
    BreakOutsideLoop,
    ContOutsideLoop,
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemanticError::UseBeforeDeclaration(n) => write!(f, "Used {} before it was declared", n),
            SemanticError::InvalidLValue => write!(f, "Invalid lvalue"),
            SemanticError::InvalidExpression => write!(f, "Invalid expression"),
            SemanticError::DoubleDeclaration => write!(f, "Variable declared"),
            SemanticError::UndeclaredLabel(n) => write!(f, "Undeclared label {}", n),
            SemanticError::DuplicateLabel(n) => write!(f, "Duplicate label {}", n),
            SemanticError::LabelWithoutStatement => write!(f, "Label without statement after it"),
            SemanticError::LabelBeforeDeclaration(n) => write!(f, "Using label {} before a declaration", n),
            SemanticError::BreakOutsideLoop => write!(f, "Break outside loop"),
            SemanticError::ContOutsideLoop => write!(f, "Cont outside loop"),
        }
    }
}

impl std::error::Error for SemanticError {}

pub fn semantic_analysis(program: &mut Program) -> Result<HashMap<String, (String, usize)>, SemanticError> {
    let map = variable_resolution_pass(program)?;
    label_semantic_analysis_pass(program)?;
    loop_labeling_pass(program)?;
    Ok(map)
}

// Variable and label resolution pass
struct Counter {
    count: usize,
    current_block: usize,
}

impl Counter {
    fn namegen(&mut self, name: &str) -> String {
        let new = format!("{}.{}_{}", name, self.count, self.current_block);
        self.count += 1;
        new
    }
    
    fn labelgen(&mut self, name: &str) -> String {
        format!("userlab.{}", name)
    }
}

fn resolve_block(block: &mut Block,
    var_map: &mut HashMap<String, (String, usize)>,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    for blockitem in &mut block.items {
        match blockitem {
            BlockItem::S(s) => resolve_statement(s, var_map, label_map, counter)?,
            BlockItem::D(d) => resolve_declaration(d, var_map, counter)?,
        }
    }
    Ok(())
}

fn resolve_program_vars(program: &mut Program,
    var_map: &mut HashMap<String, (String, usize)>,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    resolve_block(&mut program.function.body, var_map, label_map, counter)?;
    Ok(())
}

fn process_label(name: &mut String,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    let labelname = name.clone();
    if let Some((mangled, status)) = label_map.get_mut(&labelname) {
        let mangled = mangled.clone();
        if *status {
            return Err(SemanticError::DuplicateLabel(mangled));
        } else {
            *status = true;
            *name = mangled;
            Ok(())
        }
    } else {
        let newlabel = counter.labelgen(&labelname);
        label_map.insert(labelname, (newlabel.clone(), true));
        *name = newlabel;
        Ok(())
    }
}

fn process_goto(name: &mut String,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    let labelname = name.clone();
    if label_map.contains_key(&labelname) {
        let (newname, _) = label_map.get(&labelname).unwrap();
        *name = String::from(newname);
        Ok(())
    } else {
        let newlabel = counter.labelgen(&labelname);
        label_map.insert(labelname, (newlabel.clone(), false));
        *name = newlabel;
        Ok(())
    }
}

fn resolve_statement(statement: &mut Statement,
    var_map: &mut HashMap<String, (String, usize)>,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    match statement {
        Statement::Return(e) => resolve_expression(e, var_map, counter)?,
        Statement::Expression(e) => resolve_expression(e, var_map, counter)?,
        Statement::If(c,y,mn) => {
            resolve_expression(c, var_map, counter)?;
            resolve_statement(y, var_map, label_map, counter)?;
            if let Some(n) = mn {
                resolve_statement(n, var_map, label_map, counter)?;
            }
        },
        Statement::Null => return Ok(()),
        Statement::Label(name) => process_label(name, label_map, counter)?,
        Statement::Goto(name) => process_goto(name, label_map, counter)?,
        Statement::Compound(block) => {
            let mut new_map = var_map.clone();
            counter.current_block += 1;
            resolve_block(block, &mut new_map, label_map, counter)?;
            counter.current_block -= 1;
        },
        Statement::While { cond, body, lab:_ } |
        Statement::DoWhile { body, cond, lab:_ } => {
            resolve_expression(cond, var_map, counter)?;
            resolve_statement(body, var_map, label_map, counter)?;
        },
        Statement::For { init, cond, post, body, lab:_ } => {
            let mut new_map = var_map.clone();
            counter.current_block += 1;
            resolve_for_init(init, &mut new_map, counter)?;
            if let Some(cond) = cond { resolve_expression(cond, &mut new_map, counter)?; }
            if let Some(post) = post { resolve_expression(post, &mut new_map, counter)?; }
            resolve_statement(body, &mut new_map, label_map, counter)?;
            counter.current_block -= 1;
        },
        Statement::Break(_) | Statement::Continue(_) => {},
    }
    Ok(())
}

fn resolve_declaration(declaration: &mut Declaration,
    var_map: &mut HashMap<String, (String, usize)>,
    counter: &mut Counter) -> Result<(), SemanticError> {

    if let Some((_, blk)) = var_map.get(&declaration.identifier) {
        if *blk == counter.current_block {
            return Err(SemanticError::DoubleDeclaration)
        }
    }

    let newname = counter.namegen(&declaration.identifier);
    var_map.insert(declaration.identifier.clone(), (newname.clone(), counter.current_block));
    declaration.identifier = newname;

    match &mut declaration.init {
        None => return Ok(()),
        Some(e) => {
            resolve_expression(e, var_map, counter)?;
            Ok(())
        },
    }
}

fn resolve_expression(expression: &mut Expression,
    var_map: &mut HashMap<String, (String, usize)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    match expression {
        Expression::Var(x) => {
            if let Some((name, _)) = var_map.get(x) {
                *x = name.into();
            } else {
                return Err(SemanticError::UseBeforeDeclaration(x.clone()));
            }
        },
        Expression::Assignment(lhs, rhs) => {
            match lhs.as_mut() {
                Expression::Var(_) => {},
                _ => {
                    eprintln!("Invalid L-value: {:?}", lhs);
                    return Err(SemanticError::InvalidLValue);
                }
            }
            resolve_expression(lhs.as_mut(), var_map, counter)?;
            resolve_expression(rhs.as_mut(), var_map, counter)?;
        },
        Expression::Unary(_, exp) => resolve_expression(exp.as_mut(), var_map, counter)?,
        Expression::Binary(_, exp1, exp2) => {
            resolve_expression(exp1.as_mut(), var_map, counter)?;
            resolve_expression(exp2.as_mut(), var_map, counter)?;
        },
        Expression::Conditional(exp1, exp2, exp3) => {
            resolve_expression(exp1.as_mut(), var_map, counter)?;
            resolve_expression(exp2.as_mut(), var_map, counter)?;
            resolve_expression(exp3.as_mut(), var_map, counter)?;
        },
        Expression::PostfixIncrement(exp) | Expression::PrefixIncrement(exp) | 
        Expression::PostfixDecrement(exp) | Expression::PrefixDecrement(exp) => {
            match **exp {
                Expression::Var(_) => resolve_expression(exp.as_mut(), var_map, counter)?,
                _ => return Err(SemanticError::InvalidLValue),
            }
        },
        Expression::Constant(_) => return Ok(()),
    }
    Ok(())
}

fn resolve_for_init(init: &mut ForInit, var_map: &mut HashMap<String, (String, usize)>, counter: &mut Counter) -> Result<(), SemanticError> {
    if let ForInit::InitDec(dec) = init {
        resolve_declaration(dec, var_map, counter)?;
    } else if let ForInit::InitExp(Some(exp)) = init {
        resolve_expression(exp, var_map, counter)?;
    }
    Ok(())
}

fn check_undeclared_label(label_map: HashMap<String, (String, bool)>) -> Result<(), SemanticError> {
    for (key, (_, status)) in &label_map {
        if !status {
            return Err(SemanticError::UndeclaredLabel(key.clone()));
        }
    }
    Ok(())
}

// Label semantic analysis pass 
fn check_label_before_dec(items: &Vec<BlockItem>) -> Result<(), SemanticError> {
    // Check if label is before a declaration 
    for i in 0..items.len() {
        if let BlockItem::S(Statement::Compound(block)) = &items[i] {
            check_label_before_dec(&block.items)?;
        }

        if i + 1 < items.len() {
            if let BlockItem::S(Statement::Label(name)) = &items[i] {
                if let BlockItem::D(_) = &items[i+1] {
                    return Err(SemanticError::LabelBeforeDeclaration(name.clone()));
                }
            } 
        }
    }

    // Check if label is the last statement in a block
    if let Some(BlockItem::S(Statement::Label(_))) = items.last() {
        return Err(SemanticError::LabelWithoutStatement);
    }
    Ok(())
}

fn label_semantic_analysis_pass(program: &mut Program) -> Result<(), SemanticError> {
    let items = &program.function.body.items;
    check_label_before_dec(items)
}

fn variable_resolution_pass(program: &mut Program) -> Result<HashMap<String, (String, usize)>, SemanticError>{
    let mut var_map = HashMap::new();
    let mut label_map = HashMap::new();
    let mut counter = Counter{count: 0, current_block: 1};
    resolve_program_vars(program, &mut var_map, &mut label_map, &mut counter)?;
    check_undeclared_label(label_map)?;
    Ok(var_map)
}

// Loop labeling pass
struct LoopCounter {
    counter: usize,
    stack: Vec<String>,
}

impl LoopCounter {
    fn genlabel(&mut self) -> String {
        let label = format!("loop.{}", self.counter);
        self.counter += 1;
        label
    }
}

fn loop_labeling_pass(program: &mut Program) -> Result<(), SemanticError> {
    let mut count = LoopCounter{ counter: 0, stack: Vec::new() };
    assign_loop_labels(&mut program.function.body.items, &mut count)?;
    Ok(())
}

fn assign_label(st: &mut Statement, count: &mut LoopCounter) -> Result<(), SemanticError> {
    match st {
        Statement::DoWhile { body, cond:_, lab } |
        Statement::While { cond:_, body, lab } |
        Statement::For { init:_, cond:_, post:_, body, lab } => {
            let newlab = count.genlabel();
            count.stack.push(newlab);
            *lab = count.stack.last().unwrap().into();
            assign_label(body, count)?;
            count.stack.pop();
        },
        Statement::Break(lab) => {
            if let Some(newlab) = count.stack.last() {
                *lab = newlab.into();
            } else {
                return Err(SemanticError::BreakOutsideLoop);
            }
        },
        Statement::Continue(lab) => {
            if let Some(newlab) = count.stack.last() {
                *lab = newlab.into();
            } else {
                return Err(SemanticError::ContOutsideLoop);
            }
        },
        Statement::Compound(block) => {
            assign_loop_labels(&mut block.items, count)?;
        },
        Statement::If(_, yes, no) => {
            assign_label(yes, count)?;
            if let Some(no) = no {
                assign_label(no, count)?;
            }
        },
        _ => {},
    }
    Ok(())
}

fn assign_loop_labels(items: &mut Vec<BlockItem>, count: &mut LoopCounter) -> Result<(), SemanticError> {
    for item in items {
        if let BlockItem::S(st) = item {
            assign_label(st, count)?;
        }
    }
    Ok(())
}
