use std::collections::hash_map::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Long,
    UInt,
    ULong,
    FuncType{params: Vec<Box<Type>>, ret: Box<Type>},
}

impl Type {
    fn rank(&self) -> usize {
        match self {
            Type::Int => 1,
            Type::UInt => 2,
            Type::Long => 3,
            Type::ULong => 4,
            Type::FuncType { .. } => unreachable!(),
        }
    }

    fn is_signed(&self) -> bool {
        match self {
            Type::Int => true,
            Type::UInt => false,
            Type::Long => true,
            Type::ULong => false,
            Type::FuncType { .. } => unreachable!(),
        }
    }

    fn can_represent(&self, t: Type) -> u64 {
        match self {
            Type::Int => if t == ,
            Type::UInt => false,
            Type::Long => true,
            Type::ULong => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub ident: String,
    pub datatype: Type,
    pub attrs: IdentAttrs,
}

pub type SymbolTable = HashMap<String, Symbol>;

#[derive(Debug, Clone, PartialEq)]
pub enum IdentAttrs {
    FuncAttr{defined: bool, global: bool},
    StaticAttr{init: InitialValue, global: bool},
    LocalAttr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InitialValue {
    Tentative,
    Initial(StaticInit),
    NoInitializer,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StaticInit {
    IntInit(i32),
    LongInit(i64),
}

impl IdentAttrs {
    pub fn is_global(&self) -> bool {
        match &self {
            IdentAttrs::StaticAttr { init:_ , global } => *global,
            IdentAttrs::FuncAttr { defined:_ , global } => *global,
            IdentAttrs::LocalAttr => false,
        }
    }
}

