//! Physical registration counts from the statepoints LLVM actually lowered.
use super::*;
use inkwell::llvm_sys::core::{LLVMGetFirstUse, LLVMGetNextUse, LLVMGetUser, LLVMIsACallInst};

#[derive(Debug)]
pub(crate) struct LoweredSafepoints {
    pub(super) roots: BTreeMap<u64, usize>,
}

impl LoweredSafepoints {
    pub(crate) fn site_count(&self) -> usize {
        self.roots.len()
    }

    pub(crate) fn root_count(&self, id: u64) -> Option<usize> {
        self.roots.get(&id).copied()
    }
}

/// Match instruction selection's removal of unused relocations immediately
/// before unreachable. Do not simplify other CFG edges after root planning.
pub(super) fn discard_terminal_relocations(module: &LlvmModule<'_>) {
    for function in module.get_functions() {
        for block in function.get_basic_blocks() {
            let Some(terminal) = block.get_terminator() else {
                continue;
            };
            if terminal.get_opcode() != InstructionOpcode::Unreachable {
                continue;
            }
            let mut previous = terminal.get_previous_instruction();
            while let Some(instruction) = previous {
                if instruction.get_opcode() != InstructionOpcode::Call
                    || instruction.get_first_use().is_some()
                {
                    break;
                }
                // SAFETY: this instruction is a CallBase in verified RS4GC output.
                let call = unsafe { CallSiteValue::new(instruction.as_value_ref()) };
                if call.get_called_fn_value().is_none_or(|callee| {
                    !callee
                        .get_name()
                        .to_bytes()
                        .starts_with(b"llvm.experimental.gc.relocate")
                }) {
                    break;
                }
                previous = instruction.get_previous_instruction();
                instruction.erase_from_basic_block();
            }
        }
    }
}

/// A returning call's argument may have no continuation use. LLVM's target
/// passes can discard its unused gc.relocate before selecting the stack map.
fn retain_call_duration_root<'ctx>(
    module: &LlvmModule<'ctx>,
    builder: &Builder<'ctx>,
    relocation: CallSiteValue<'ctx>,
) -> Result<(), CodegenError> {
    let value = relocation
        .try_as_basic_value()
        .expect_basic("gc.relocate returns the relocated value");
    let instruction = value.as_instruction_value().expect("gc.relocate is a call");
    if instruction.get_first_use().is_none() {
        let next = instruction
            .get_next_instruction()
            .expect("gc.relocate precedes the block terminator");
        builder.position_before(&next);
        root_identity::build_fake_use(module, builder, value)?;
    }
    Ok(())
}

pub(super) fn finalize(module: &LlvmModule<'_>) -> Result<LoweredSafepoints, CodegenError> {
    let builder = module.get_context().create_builder();
    let mut roots = BTreeMap::new();
    for function in module.get_functions() {
        for block in function.get_basic_blocks() {
            for instruction in block.get_instructions() {
                if !matches!(
                    instruction.get_opcode(),
                    InstructionOpcode::Call | InstructionOpcode::Invoke
                ) {
                    continue;
                }
                // SAFETY: this instruction is a CallBase in LLVM's successful output.
                let call = unsafe { CallSiteValue::new(instruction.as_value_ref()) };
                if call.get_called_fn_value().is_none_or(|callee| {
                    !callee
                        .get_name()
                        .to_bytes()
                        .starts_with(b"llvm.experimental.gc.statepoint")
                }) {
                    continue;
                }
                let id = unsafe {
                    LLVMConstIntGetZExtValue(LLVMGetOperand(instruction.as_value_ref(), 0))
                };
                let live = call
                    .get_operand_bundles()
                    .find(|bundle| bundle.get_tag() == Ok("gc-live"))
                    .map(|bundle| bundle.get_args().collect::<Vec<_>>())
                    .unwrap_or_default();
                let mut derived = BTreeSet::new();
                // Scoop invokes and native transitions use explicit roots and
                // zero-live statepoints. Ordinary relocating calls use token users.
                let mut usage = unsafe { LLVMGetFirstUse(instruction.as_value_ref()) };
                while !usage.is_null() {
                    let user = unsafe { LLVMGetUser(usage) };
                    if !unsafe { LLVMIsACallInst(user) }.is_null() {
                        let relocation = unsafe { CallSiteValue::new(user) };
                        if relocation.get_called_fn_value().is_some_and(|callee| {
                            callee
                                .get_name()
                                .to_bytes()
                                .starts_with(b"llvm.experimental.gc.relocate")
                        }) {
                            let index = unsafe { LLVMConstIntGetZExtValue(LLVMGetOperand(user, 2)) }
                                as usize;
                            derived.insert(live[index].as_value_ref() as usize);
                            retain_call_duration_root(module, &builder, relocation)?;
                        }
                    }
                    usage = unsafe { LLVMGetNextUse(usage) };
                }
                roots.insert(id, derived.len());
            }
        }
    }
    Ok(LoweredSafepoints { roots })
}
