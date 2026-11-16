//! Context key cells are associated atoms of their existing callable body.

pub use scoop_identity::ContextKey;

use scoop_identity::{
    DefinitionAtomRole, DefinitionAtomSubkey, ObjectDefinitionAtomId, ObjectDefinitionAtomKey,
    ObjectDefinitionPlanId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallableContextKeyCellV1 {
    pub key: ContextKey,
    pub atom: ObjectDefinitionAtomId,
}

pub fn context_key_cell_atom(
    plan: ObjectDefinitionPlanId,
    key: ContextKey,
) -> Result<ObjectDefinitionAtomId, scoop_wire::HashError> {
    ObjectDefinitionAtomId::from_key(&ObjectDefinitionAtomKey::new(
        plan,
        DefinitionAtomRole::ContextKeyCell,
        DefinitionAtomSubkey::ExactType(key.0),
    ))
}

pub fn context_key_table_atom(
    plan: ObjectDefinitionPlanId,
) -> Result<ObjectDefinitionAtomId, scoop_wire::HashError> {
    ObjectDefinitionAtomId::from_key(&ObjectDefinitionAtomKey::new(
        plan,
        DefinitionAtomRole::ContextKeyTable,
        DefinitionAtomSubkey::Singleton,
    ))
}

pub fn function_context_keys(function: &crate::Function) -> Vec<ContextKey> {
    let mut keys = std::collections::BTreeSet::new();
    for (_, block) in function.blocks.iter() {
        let calls = block
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                crate::Instruction::Call { site } => Some(site.args()),
                _ => None,
            });
        let invoke = match &block.terminator {
            crate::Terminator::Invoke { site } => Some(site.args()),
            _ => None,
        };
        for arguments in calls.chain(invoke) {
            for argument in arguments {
                if let crate::AbiCallArgument::Direct(crate::Value::ContextKeyCell(key)) = argument
                {
                    keys.insert(*key);
                }
            }
        }
    }
    keys.into_iter().collect()
}
