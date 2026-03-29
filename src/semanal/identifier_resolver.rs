// Var and label resolution pass
use std::collections::hash_map::HashMap;
use crate::parser::*;
use super::SemanticError;

// (mangled_name, has_linkage)
type IdentMap = HashMap<String, (String, bool)>;

struct Counter {
    count: usize,
}

impl Counter {
    fn namegen(&mut self, name: &str) -> String {
        let new = format!("{}.{}", name, self.count);
        self.count += 1;
        new
    }
    
    fn labelgen(&mut self, name: &str) -> String {
        format!("userlab.{}", name)
    }
}

fn resolve_block(block: &mut Block,
    ident_map: &mut IdentMap,
    outer: &IdentMap,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {

    for blockitem in &mut block.items {
        match blockitem {
            BlockItem::S(s) => resolve_statement(s, ident_map, label_map, counter)?,
            BlockItem::D(Decl::VarDecl(d)) => resolve_var_declaration(d, ident_map, outer, counter)?,
            BlockItem::D(Decl::FuncDecl(f)) => resolve_func_declaration(f, ident_map, outer, label_map, counter)?,
        }
    }
    Ok(())
}

fn resolve_program_idents(program: &mut Program,
    ident_map: &mut IdentMap,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    // at the top level there is no outer scope
    let outer = ident_map.clone();
    for function in &mut program.functions {
        resolve_func_declaration(function, ident_map, &outer, label_map, counter)?;
    }
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
    ident_map: &mut IdentMap,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {
    match statement {
        Statement::Return(e) => resolve_expression(e, ident_map, counter)?,
        Statement::Expression(e) => resolve_expression(e, ident_map, counter)?,
        Statement::If(c,y,mn) => {
            resolve_expression(c, ident_map, counter)?;
            resolve_statement(y, ident_map, label_map, counter)?;
            if let Some(n) = mn {
                resolve_statement(n, ident_map, label_map, counter)?;
            }
        },
        Statement::Label(name, st) => {
            process_label(name, label_map, counter)?;
            resolve_statement(st, ident_map, label_map, counter)?;
        },
        Statement::Goto(name) => process_goto(name, label_map, counter)?,
        Statement::Compound(block) => {
            let mut new_map = ident_map.clone();
            let outer = ident_map.clone();
            resolve_block(block, &mut new_map, &outer, label_map, counter)?;
        },
        Statement::While { cond, body, lab:_ } |
        Statement::DoWhile { body, cond, lab:_ } => {
            resolve_expression(cond, ident_map, counter)?;
            resolve_statement(body, ident_map, label_map, counter)?;
        },
        Statement::For { init, cond, post, body, lab:_ } => {
            let mut new_map = ident_map.clone();
            // snapshot before any for-init declarations
            let outer = new_map.clone();
            resolve_for_init(init, &mut new_map, &outer, counter)?;
            if let Some(cond) = cond { resolve_expression(cond, &mut new_map, counter)?; }
            if let Some(post) = post { resolve_expression(post, &mut new_map, counter)?; }
            resolve_statement(body, &mut new_map, label_map, counter)?;
        },
        Statement::Switch { scrutinee, body, .. } => {
            resolve_expression(scrutinee, ident_map, counter)?;
            resolve_statement(body, ident_map, label_map, counter)?;
        },
        Statement::Case{expr, ..} => {
            match eval_constant(expr) {
                Some(value) => *expr = Expression::Constant(value),
                None => return Err(SemanticError::NonConstantCase),
            }
        },
        Statement::Null | Statement::Break(_) | 
        Statement::Continue(_) | Statement::Default{..} => {},
    }
    Ok(())
}

fn resolve_var_declaration(declaration: &mut VarDeclaration,
    ident_map: &mut IdentMap,
    outer: &IdentMap,
    counter: &mut Counter) -> Result<(), SemanticError> {

    // duplicate if present in current map but NOT in outer snapshot
    if ident_map.contains_key(&declaration.identifier) && !outer.contains_key(&declaration.identifier) {
        let name = ident_map[&declaration.identifier].0.clone();
        return Err(SemanticError::DoubleDeclaration(name));
    }

    let newname = counter.namegen(&declaration.identifier);
    ident_map.insert(declaration.identifier.clone(), (newname.clone(), false));
    declaration.identifier = newname;

    match &mut declaration.init {
        None => return Ok(()),
        Some(e) => {
            resolve_expression(e, ident_map, counter)?;
            Ok(())
        },
    }
}

fn resolve_func_declaration(declaration: &mut FuncDeclaration,
    ident_map: &mut IdentMap,
    outer: &IdentMap,
    label_map: &mut HashMap<String, (String, bool)>,
    counter: &mut Counter) -> Result<(), SemanticError> {

    if !outer.is_empty() {
        if declaration.body.is_some() {
            return Err(SemanticError::NestedFunctionDefinition(declaration.identifier.to_string()));
        }
    }

    if let Some((name, linkage)) = ident_map.get(&declaration.identifier) {
        if !outer.contains_key(&declaration.identifier) && !*linkage {
            return Err(SemanticError::DoubleDeclaration(name.to_string()));
        }
    }
    
    ident_map.insert(declaration.identifier.clone(), (declaration.identifier.clone(), true));
    
    let mut inner_map = ident_map.clone();
    let param_outer = inner_map.clone();
    let mut new_params = Vec::new();
    for param in &declaration.params {
        new_params.push(resolve_parameter(param, &mut inner_map, &param_outer, counter)?);
    }
    declaration.params = new_params;

    match &mut declaration.body {
        None => return Ok(()),
        Some(e) => {
            resolve_block(e, &mut inner_map, &param_outer, label_map, counter)?;
            Ok(())
        },
    }
}

fn resolve_parameter(param: &String,
    ident_map: &mut IdentMap,
    outer: &IdentMap,
    counter: &mut Counter) -> Result<String, SemanticError> {
    // duplicate param if it's in the current map but not in the pre-param snapshot
    if ident_map.contains_key(param) && !outer.contains_key(param) {
        let name = ident_map[param].0.clone();
        return Err(SemanticError::DoubleDeclaration(name));
    }

    let newname = counter.namegen(param);
    ident_map.insert(param.to_string(), (newname.clone(), false));
    Ok(newname)
}

fn resolve_expression(expression: &mut Expression,
    ident_map: &IdentMap,
    counter: &mut Counter) -> Result<(), SemanticError> {
    match expression {
        Expression::Var(x) => {
            if let Some((name, _)) = ident_map.get(x) {
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
            resolve_expression(lhs.as_mut(), ident_map, counter)?;
            resolve_expression(rhs.as_mut(), ident_map, counter)?;
        },
        Expression::Unary(_, exp) => resolve_expression(exp.as_mut(), ident_map, counter)?,
        Expression::Binary(_, exp1, exp2) => {
            resolve_expression(exp1.as_mut(), ident_map, counter)?;
            resolve_expression(exp2.as_mut(), ident_map, counter)?;
        },
        Expression::Conditional(exp1, exp2, exp3) => {
            resolve_expression(exp1.as_mut(), ident_map, counter)?;
            resolve_expression(exp2.as_mut(), ident_map, counter)?;
            resolve_expression(exp3.as_mut(), ident_map, counter)?;
        },
        Expression::PostfixIncrement(exp) | Expression::PrefixIncrement(exp) | 
        Expression::PostfixDecrement(exp) | Expression::PrefixDecrement(exp) => {
            match **exp {
                Expression::Var(_) => resolve_expression(exp.as_mut(), ident_map, counter)?,
                _ => return Err(SemanticError::InvalidLValue),
            }
        },
        Expression::Constant(_) => return Ok(()),
        Expression::FunctionCall(name, args) => {
            if let Some((new_name, _)) = ident_map.get(name) {
                *name = new_name.into();
                for arg in args {
                    resolve_expression(arg, ident_map, counter)?;
                } 
            } else {
                return Err(SemanticError::UseBeforeDeclaration(name.to_string()));
            }
        },
    }
    Ok(())
}

fn resolve_for_init(init: &mut ForInit,
    ident_map: &mut IdentMap,
    outer: &IdentMap,
    counter: &mut Counter) -> Result<(), SemanticError> {
    if let ForInit::InitDec(dec) = init {
        resolve_var_declaration(dec, ident_map, outer, counter)?;
    } else if let ForInit::InitExp(Some(exp)) = init {
        resolve_expression(exp, ident_map, counter)?;
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

fn eval_constant(expr: &Expression) -> Option<i32> {
    match expr {
        Expression::Constant(n) => Some(*n),
        Expression::Unary(op, expr) => {
            let val = eval_constant(expr)?;
            match op {
                UnaryOp::Negate => Some(-val),
                UnaryOp::Complement => Some(!val),
                UnaryOp::Not => Some((val == 0) as i32),
            }
        },
        Expression::Binary(op, left, right) => {
            let l = eval_constant(left)?;
            let r = eval_constant(right)?;
            match op {
                BinaryOp::Add             => Some(l + r),
                BinaryOp::Subtract        => Some(l - r),
                BinaryOp::Multiply        => Some(l * r),
                BinaryOp::Divide          => if r == 0 { None } else { Some(l / r) },
                BinaryOp::Remainder       => if r == 0 { None } else { Some(l % r) },
                BinaryOp::LeftShift       => Some(l << r),
                BinaryOp::RightShift      => Some(l >> r),
                BinaryOp::LessThan        => Some((l < r) as i32),
                BinaryOp::LessOrEqual     => Some((l <= r) as i32),
                BinaryOp::GreaterThan     => Some((l > r) as i32),
                BinaryOp::GreaterOrEqual  => Some((l >= r) as i32),
                BinaryOp::Equal           => Some((l == r) as i32),
                BinaryOp::NotEqual        => Some((l != r) as i32),
                BinaryOp::BitwiseAnd      => Some(l & r),
                BinaryOp::BitwiseXor      => Some(l ^ r),
                BinaryOp::BitwiseOr       => Some(l | r),
                BinaryOp::LogicalAnd      => Some((l != 0 && r != 0) as i32),
                BinaryOp::LogicalOr       => Some((l != 0 || r != 0) as i32),
                _ => None,
            }
        },
        _ => None,
    }
}

pub fn identifier_resolution_pass(program: &mut Program) -> Result<IdentMap, SemanticError>{
    let mut ident_map = HashMap::new();
    let mut label_map = HashMap::new();
    let mut counter = Counter { count: 0 };
    resolve_program_idents(program, &mut ident_map, &mut label_map, &mut counter)?;
    check_undeclared_label(label_map)?;
    Ok(ident_map)
}
