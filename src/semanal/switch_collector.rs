use super::*;
use std::collections::hash_set::HashSet;
use visitor_trait::*;

struct SwitchCollector;

impl Visitor for SwitchCollector {
    fn visit_statement(&mut self, statement: &mut Statement) -> Result<(), SemanticError> {
        match statement {
            Statement::Switch { cases, body, .. } => {
                *cases = collect_switch_cases(body)?;

                let mut seen = HashSet::new();
                for (expr, _) in cases.iter() {
                    if let Some(Expression::Constant(value)) = expr {
                        if !seen.insert(value) {
                            return Err(SemanticError::DuplicateCase);
                        }
                    }
                }

                let default_count = cases.iter().filter(|(expr, _)| expr.is_none()).count();
                if default_count > 1 {
                    return Err(SemanticError::DuplicateDefault);
                }

                if let Statement::Compound(block) = body.as_ref() {
                    check_block_for_decs(block)?;
                }

                self.visit_statement(body)?;
            },
            _ => { walk_statement(self, statement)?; }
        }
        Ok(())
    }
}

fn collect_cases_in_block(items: &Vec<BlockItem>) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    for item in items {
        match item {
            BlockItem::S(Statement::Case { expr, lab }) => {
                cases.push((Some(expr.clone()), lab.clone()));
            },
            BlockItem::S(Statement::Default { lab }) => {
                cases.push((None, lab.clone()));
            },
            BlockItem::S(Statement::Compound(bl)) => {
                cases.append(&mut collect_cases_in_block(&bl.items)?);
            },
            BlockItem::S(Statement::If(_, yes, no)) => {
                cases.append(&mut collect_switch_cases(yes)?);
                if let Some(no) = no {
                    cases.append(&mut collect_switch_cases(no)?);
                }
            },
            BlockItem::S(Statement::Label(_, body)) => {
                cases.append(&mut collect_switch_cases(body)?);
            },
            BlockItem::S(Statement::For {body,..}) |
            BlockItem::S(Statement::While {body,..}) |
            BlockItem::S(Statement::DoWhile {body,..}) => {
                if let Statement::Compound(bl) = body.as_ref() {
                    cases.append(&mut collect_cases_in_block(&bl.items)?);
                } else {
                    cases.append(&mut collect_switch_cases(body)?);
                }
            },
            BlockItem::S(_) | BlockItem::D(_) => {},
        }
    }
    Ok(cases)
}

fn collect_switch_cases(st: &Statement) -> Result<Vec<(Option<Expression>, String)>, SemanticError> {
    let mut cases = Vec::new();
    if let Statement::Compound(block) = st {
        cases.append(&mut collect_cases_in_block(&block.items)?);
    } else if let Statement::Case { expr, lab } = st {
        cases.push((Some(expr.clone()), lab.clone()));
    } else if let Statement::Default { lab } = st {
        cases.push((None, lab.clone()));
    }
    Ok(cases)
}

fn check_block_for_decs(block: &Block) -> Result<(), SemanticError> {
    let items = &block.items;
    for window in items.windows(2) {
        if let [BlockItem::S(Statement::Case { .. } | Statement::Default { .. }), BlockItem::D(_)] = window {
            return Err(SemanticError::DecInCase);
        }
    }
    Ok(())
}

pub fn switch_collection_pass(program: &mut Program) -> Result<(), SemanticError> {
    let mut collector = SwitchCollector{};
    collector.visit_program(program)
}
