//! Retain logical root identities and base merges before statepoint rewriting.

use super::*;

pub(super) fn finalize(
    module: &LlvmModule<'_>,
    expected: &ExpectedSafepoints,
    profile: ValidatedBackendProfile,
) -> Result<(), CodegenError> {
    let metadata = module
        .get_context()
        .get_kind_id(STATEPOINT_ROOT_IDENTITY_METADATA);
    let mut sites = BTreeMap::new();
    let mut identities = BTreeMap::<u64, BTreeSet<(u8, u32, u64)>>::new();
    for function in module.get_functions() {
        let function_name = function.get_name().to_str().map_err(|error| {
            CodegenError::backend(format!("LLVM function name: {error}"), error)
        })?;
        for block in function.get_basic_blocks() {
            for instruction in block.get_instructions() {
                retain_merge_base(instruction, profile)?;
                if let Some((id, root)) = root_identity::read(instruction, metadata)? {
                    let site = expected.sites.get(&id).ok_or_else(|| {
                        CodegenError::internal(format!(
                            "root restoration refers to unknown SafepointId {id}"
                        ))
                        .with_code(scoop_source::DiagnosticCode::ICE1101)
                    })?;
                    if site.function != function_name
                        || !matches!(site.statepoint, ExpectedStatepoint::Relocating(_))
                    {
                        return Err(CodegenError::internal(format!(
                            "root restoration for {id} has the wrong owner or call protocol"
                        ))
                        .with_code(scoop_source::DiagnosticCode::ICE1101));
                    }
                    if !identities.entry(id).or_default().insert(root.key()) {
                        return Err(CodegenError::internal(format!(
                            "statepoint {id} repeats typed root identity {root:?}"
                        ))
                        .with_code(scoop_source::DiagnosticCode::ICE1101));
                    }
                    let value = root_identity::restored_value(
                        instruction,
                        profile.managed_address_space_contract(),
                    )?;
                    if !constant_roots::is_constant_root(value)
                        && !constant_roots::mark_base_merge(value)?
                    {
                        return Err(CodegenError::internal(
                            "a final managed root merge contains a derived address".into(),
                        )
                        .with_code(scoop_source::DiagnosticCode::ICE1101));
                    }
                }
                let Some(id) = site_id(instruction)? else {
                    continue;
                };
                let site = expected.sites.get(&id).ok_or_else(|| {
                    CodegenError::internal(format!(
                        "optimized LLVM contains unexpected SafepointId {id}"
                    ))
                    .with_code(scoop_source::DiagnosticCode::ICE1101)
                })?;
                if site.function != function_name {
                    return Err(CodegenError::internal(format!(
                        "optimized SafepointId {id} changed its callable owner"
                    ))
                    .with_code(scoop_source::DiagnosticCode::ICE1101));
                }
                let terminal = instruction
                    .get_next_instruction()
                    .is_some_and(|next| next.get_opcode() == InstructionOpcode::Unreachable);
                if sites.insert(id, terminal).is_some() {
                    return Err(CodegenError::internal(format!(
                        "optimization duplicated SafepointId {id}"
                    ))
                    .with_code(scoop_source::DiagnosticCode::ICE1101));
                }
            }
        }
    }
    for (id, terminal) in &sites {
        let site = &expected.sites[id];
        let ExpectedStatepoint::Relocating(initial) = &site.statepoint else {
            continue;
        };
        let actual = identities.remove(id).unwrap_or_default();
        let required = initial.identities();
        // LLVM may discard every continuation use after a call that now ends
        // directly in unreachable. Returning and exceptional paths still need
        // all restorations; a terminal site cannot introduce unplanned roots.
        if actual != required && (!*terminal || !actual.is_subset(&required)) {
            return Err(CodegenError::internal(format!(
                "optimized statepoint {id} in {} has incomplete typed leaf restorations: expected {:?}, observed {actual:?}",
                site.function, required
            )).with_code(scoop_source::DiagnosticCode::ICE1101));
        }
    }
    if let Some(id) = identities.keys().next() {
        return Err(CodegenError::internal(format!(
            "restoration remains for deleted SafepointId {id}"
        ))
        .with_code(scoop_source::DiagnosticCode::ICE1101));
    }
    Ok(())
}

// Path folding can replace a restore with null while a shared successor keeps
// the original merge alive. Preserve its base fact independently of that use.
fn retain_merge_base(
    instruction: InstructionValue<'_>,
    profile: ValidatedBackendProfile,
) -> Result<(), CodegenError> {
    if matches!(
        instruction.get_opcode(),
        InstructionOpcode::Phi | InstructionOpcode::Select
    ) && instruction.get_type().is_pointer_type()
    {
        // SAFETY: the opcode and type establish a scalar pointer instruction.
        let value = unsafe { BasicValueEnum::new(instruction.as_value_ref()) }.into_pointer_value();
        if value.get_type().get_address_space()
            == profile.managed_address_space_contract().inkwell()
            && !constant_roots::is_constant_root(value)
        {
            // Non-root derived merges may exist; they cannot be marked as bases.
            constant_roots::mark_base_merge(value)?;
        }
    }
    Ok(())
}

fn site_id(instruction: InstructionValue<'_>) -> Result<Option<u64>, CodegenError> {
    if !matches!(
        instruction.get_opcode(),
        InstructionOpcode::Call | InstructionOpcode::Invoke
    ) {
        return Ok(None);
    }
    // SAFETY: the opcode above establishes a CallBase.
    let call = unsafe { CallSiteValue::new(instruction.as_value_ref()) };
    if let Some(attribute) = call.get_string_attribute(AttributeLoc::Function, "statepoint-id") {
        return attribute
            .get_string_value()
            .to_str()
            .ok()
            .and_then(|text| text.parse().ok())
            .map(Some)
            .ok_or_else(|| {
                CodegenError::internal("invalid statepoint-id call attribute".into())
                    .with_code(scoop_source::DiagnosticCode::ICE1101)
            });
    }
    let Some(callee) = call.get_called_fn_value() else {
        return Ok(None);
    };
    if !callee
        .get_name()
        .to_bytes()
        .starts_with(b"llvm.experimental.gc.statepoint")
    {
        return Ok(None);
    }
    let raw = instruction.as_value_ref();
    if unsafe { LLVMGetNumArgOperands(raw) } == 0 {
        return Err(
            CodegenError::internal("statepoint id operand is missing".into())
                .with_code(scoop_source::DiagnosticCode::ICE1101),
        );
    }
    let operand = unsafe { LLVMGetOperand(raw, 0) };
    if unsafe { LLVMIsAConstantInt(operand) }.is_null() {
        return Err(
            CodegenError::internal("statepoint id operand is not constant".into())
                .with_code(scoop_source::DiagnosticCode::ICE1101),
        );
    }
    Ok(Some(unsafe { LLVMConstIntGetZExtValue(operand) }))
}
