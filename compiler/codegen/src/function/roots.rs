use super::*;

impl<'ctx> FnEmitter<'_, 'ctx> {
    /// Allocate a temporary in the entry block. Enum values are
    /// materialized through memory (see EnumWrap / EnumField); allocas
    /// must dominate every use, so they go before the first instruction
    /// of the entry block alongside the locals' slots.
    pub(super) fn entry_alloca(
        &self,
        ty: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, CodegenError> {
        let builder = self.builder;
        let current = builder.get_insert_block().ok_or_else(|| {
            CodegenError::internal("builder has no insertion block".to_string())
                .with_code(scoop_source::DiagnosticCode::ICE1101)
        })?;
        match self.entry_block.get_first_instruction() {
            Some(first) => builder.position_at(self.entry_block, &first),
            None => builder.position_at_end(self.entry_block),
        }
        let alloca = builder.build_alloca(ty, name).map_err(|e| {
            CodegenError::backend(
                format!(
                    "alloca {name} @{symbol}: {e}",
                    symbol = self.function.symbol()
                ),
                e,
            )
        })?;
        builder.position_at_end(current);
        Ok(alloca)
    }

    pub(super) fn statepoint_source_type(
        &self,
        source: scoop_lir::CallerRootSource,
    ) -> Result<&LirType, CodegenError> {
        match source {
            scoop_lir::CallerRootSource::Param(index) => self
                .function
                .signature
                .arguments()
                .get(index as usize)
                .map(scoop_lir::AbiArgument::logical_storage_type)
                .ok_or_else(|| {
                    CodegenError::internal(format!("statepoint param {index} is out of range"))
                        .with_code(scoop_source::DiagnosticCode::ICE1101)
                }),
            scoop_lir::CallerRootSource::Local(id) => Ok(self.function.locals[id].ty()),
            scoop_lir::CallerRootSource::Temp(id) => Ok(&self.function.temps[id].ty),
        }
    }

    pub(super) fn root_source_storage(
        &self,
        source: scoop_lir::CallerRootSource,
    ) -> Result<RootStorage<'ctx>, CodegenError> {
        let source_ty = self.statepoint_source_type(source)?;
        if machine_types::contains_machine_scalar(self.structs, self.enums, source_ty) {
            return Err(CodegenError::internal(format!(
                "root source in @{} cannot contain an internal machine scalar",
                self.function.symbol()
            ))
            .with_code(scoop_source::DiagnosticCode::ICE1101));
        }
        match source {
            scoop_lir::CallerRootSource::Local(id) => Ok(RootStorage {
                pointer: self.local_pointer(id)?,
                ty: basic_ty(
                    self.context,
                    self.structs,
                    self.enums,
                    self.managed_address_space,
                    self.function.locals[id].ty(),
                )?,
            }),
            scoop_lir::CallerRootSource::Param(_) | scoop_lir::CallerRootSource::Temp(_) => {
                self.root_storage.get(&source).copied().ok_or_else(|| {
                    CodegenError::internal("root source has no canonical storage".to_string())
                        .with_code(scoop_source::DiagnosticCode::ICE1101)
                })
            }
        }
    }

    /// Turn the complete LIR live set into an AS1 SSA value for each
    /// managed leaf. Only leaves needed by a continuation receive deliberate
    /// post-call uses; dead direct AS1 operands remain call-duration roots.
    pub(super) fn materialize_statepoint_live(
        &mut self,
        live: &scoop_lir::StatepointLiveSet,
        safepoint: scoop_lir::SafepointId,
    ) -> Result<MaterializedStatepointLive<'ctx>, CodegenError> {
        let mut arguments = HashMap::with_capacity(live.as_slice().len());
        let mut leaves = Vec::new();
        for item in live.as_slice() {
            let source_ty = self.statepoint_source_type(item.source)?;
            if source_ty != &item.ty {
                return Err(CodegenError::internal(format!(
                    "statepoint {} source type is incomplete or inconsistent",
                    safepoint.get()
                ))
                .with_code(scoop_source::DiagnosticCode::ICE1101));
            }
            if machine_types::contains_machine_scalar(self.structs, self.enums, &item.ty) {
                return Err(CodegenError::internal(format!(
                    "statepoint {} cannot treat an internal machine scalar as a managed root",
                    safepoint.get()
                ))
                .with_code(scoop_source::DiagnosticCode::ICE1101));
            }
            let llvm_ty = basic_ty(
                self.context,
                self.structs,
                self.enums,
                self.managed_address_space,
                &item.ty,
            )?;
            let storage = match item.source {
                scoop_lir::CallerRootSource::Local(id) => self.local_pointer(id)?,
                scoop_lir::CallerRootSource::Param(_) | scoop_lir::CallerRootSource::Temp(_) => {
                    let canonical = self.root_storage.get(&item.source).ok_or_else(|| {
                        CodegenError::internal(format!(
                            "statepoint {} source lacks canonical root storage",
                            safepoint.get()
                        ))
                        .with_code(scoop_source::DiagnosticCode::ICE1101)
                    })?;
                    if canonical.ty != llvm_ty {
                        return Err(CodegenError::internal(format!(
                            "statepoint {} root storage disagrees with source type",
                            safepoint.get()
                        ))
                        .with_code(scoop_source::DiagnosticCode::ICE1101));
                    }
                    canonical.pointer
                }
            };
            let source_size = self.target_data.get_store_size(&llvm_ty);
            for leaf in item.leaves.as_slice() {
                if leaf
                    .byte_offset
                    .checked_add(8)
                    .is_none_or(|end| end > source_size)
                {
                    return Err(CodegenError::internal(format!(
                        "statepoint {} managed leaf offset {} is outside its concrete type",
                        safepoint.get(),
                        leaf.byte_offset
                    ))
                    .with_code(scoop_source::DiagnosticCode::ICE1101));
                }
                // SAFETY: the LIR leaf path was computed from the concrete
                // physical layout and was range-checked immediately above.
                let leaf_storage = unsafe {
                    self.builder.build_gep(
                        self.context.i8_type(),
                        storage,
                        &[self.context.i64_type().const_int(leaf.byte_offset, false)],
                        &format!("statepoint_{}_leaf", safepoint.get()),
                    )
                }
                .map_err(|error| {
                    CodegenError::backend(format!("address statepoint leaf: {error}"), error)
                })?;
                let source_value = self
                    .builder
                    .build_load(
                        managed_ptr_ty(self.context, self.managed_address_space),
                        leaf_storage,
                        &format!("statepoint_{}_source", safepoint.get()),
                    )
                    .map_err(|error| {
                        CodegenError::backend(format!("load statepoint leaf: {error}"), error)
                    })?
                    .into_pointer_value();
                leaves.push(StatepointLiveLeaf {
                    storage: leaf_storage,
                    value: source_value,
                    source: item.source,
                    byte_offset: leaf.byte_offset,
                    restore: item.restore,
                });
            }
            let value = match llvm_ty {
                BasicTypeEnum::PointerType(pointer)
                    if pointer == managed_ptr_ty(self.context, self.managed_address_space)
                        && item.leaves.as_slice().len() == 1
                        && item.leaves.as_slice()[0].byte_offset == 0 =>
                {
                    leaves
                        .last()
                        .expect("the single managed leaf was materialized")
                        .value
                        .into()
                }
                _ if self.is_interface_storage(&item.ty) => self.interface_with_object(
                    storage,
                    leaves
                        .last()
                        .expect("an interface has one managed leaf")
                        .value
                        .into(),
                )?,
                _ => self
                    .builder
                    .build_load(
                        llvm_ty,
                        storage,
                        &format!("statepoint_{}_argument", safepoint.get()),
                    )
                    .map_err(|error| {
                        CodegenError::backend(
                            format!("reload materialized statepoint source: {error}"),
                            error,
                        )
                    })?,
            };
            arguments.insert(item.source, value);
        }
        Ok(MaterializedStatepointLive { arguments, leaves })
    }

    pub(super) fn restore_statepoint_live(
        &mut self,
        live: MaterializedStatepointLive<'ctx>,
        safepoint: scoop_lir::SafepointId,
    ) -> Result<(), CodegenError> {
        for leaf in live.leaves {
            if !leaf.restore {
                continue;
            }
            statepoint::mark_root_identity(
                self.context,
                self.llvm,
                self.builder,
                leaf.value,
                safepoint,
                leaf.source,
                leaf.byte_offset,
            )?;
            let store = self
                .builder
                .build_store(leaf.storage, leaf.value)
                .map_err(|error| {
                    CodegenError::backend(
                        format!("restore statepoint {} leaf: {error}", safepoint.get()),
                        error,
                    )
                })?;
            store.set_volatile(true).map_err(|error| {
                CodegenError::backend(
                    format!(
                        "make statepoint {} leaf store volatile: {error}",
                        safepoint.get()
                    ),
                    error,
                )
            })?;
            statepoint::mark_storage_access(self.context, store)?;
        }
        Ok(())
    }

    pub(super) fn prepare_root_storage(&mut self) -> Result<(), CodegenError> {
        for source in root_storage_sources(self.function) {
            if matches!(source, scoop_lir::CallerRootSource::Local(_)) {
                continue;
            }
            let lir_type = self.statepoint_source_type(source)?.clone();
            let ty = basic_ty(
                self.context,
                self.structs,
                self.enums,
                self.managed_address_space,
                &lir_type,
            )?;
            let storage = match source {
                scoop_lir::CallerRootSource::Param(index) => {
                    let logical_index = index as usize;
                    let argument = self
                        .function
                        .signature
                        .arguments()
                        .get(logical_index)
                        .ok_or_else(|| {
                            CodegenError::internal(format!("rooted param {index} is out of range"))
                                .with_code(scoop_source::DiagnosticCode::ICE1101)
                        })?;
                    let location = self
                        .function
                        .signature
                        .argument_location(logical_index)
                        .expect("an existing ABI argument has a location");
                    match (argument, location) {
                        (
                            scoop_lir::AbiArgument::Direct(scoop_lir::AbiDirectValue::DirectParts(
                                parts,
                            )),
                            scoop_lir::AbiArgumentLocation::Parts { first, .. },
                        ) => {
                            let initial = self.incoming_direct_parts(parts, first)?;
                            let pointer = self.entry_alloca(ty, "managed_root_storage")?;
                            self.builder.build_store(pointer, initial).map_err(|e| {
                                CodegenError::backend(format!("initialize interface root: {e}"), e)
                            })?;
                            RootStorage { pointer, ty }
                        }
                        (
                            scoop_lir::AbiArgument::Direct(_),
                            scoop_lir::AbiArgumentLocation::Parameter(physical_index),
                        ) => {
                            let physical_index =
                                u32::try_from(physical_index).map_err(|cause| {
                                    CodegenError::internal_cause(format!(
                                    "physical parameter for rooted param {index} exceeds u32::MAX"
                                ), cause).with_code(scoop_source::DiagnosticCode::ICE1101)
                                })?;
                            let initial = self
                                .llvm_function
                                .get_nth_param(physical_index)
                                .ok_or_else(|| {
                                    CodegenError::internal(format!(
                                        "physical parameter for rooted param {index} is out of range"
                                    )).with_code(scoop_source::DiagnosticCode::ICE1101)
                                })?;
                            let pointer = self.entry_alloca(ty, "managed_root_storage")?;
                            self.builder
                                .build_store(pointer, initial)
                                .map_err(|error| {
                                    CodegenError::backend(
                                        format!("initialize managed root storage: {error}"),
                                        error,
                                    )
                                })?;
                            RootStorage { pointer, ty }
                        }
                        (
                            scoop_lir::AbiArgument::Indirect(_),
                            scoop_lir::AbiArgumentLocation::Parameter(physical_index),
                        ) => {
                            let physical_index =
                                u32::try_from(physical_index).map_err(|cause| {
                                    CodegenError::internal_cause(format!(
                                    "physical parameter for rooted param {index} exceeds u32::MAX"
                                ), cause).with_code(scoop_source::DiagnosticCode::ICE1101)
                                })?;
                            let parameter = self
                                .llvm_function
                                .get_nth_param(physical_index)
                                .ok_or_else(|| {
                                    CodegenError::internal(format!(
                                        "physical parameter for rooted param {index} is out of range"
                                    )).with_code(scoop_source::DiagnosticCode::ICE1101)
                                })?;
                            let BasicValueEnum::PointerValue(pointer) = parameter else {
                                return Err(CodegenError::internal(format!(
                                    "indirect rooted param {index} does not use pointer storage"
                                ))
                                .with_code(scoop_source::DiagnosticCode::ICE1101));
                            };
                            RootStorage { pointer, ty }
                        }
                        (
                            scoop_lir::AbiArgument::ElidedZst(_),
                            scoop_lir::AbiArgumentLocation::Elided,
                        ) => {
                            return Err(CodegenError::internal(format!(
                                "elided param {index} appears in a root plan despite its empty scan"
                            ))
                            .with_code(scoop_source::DiagnosticCode::ICE1101));
                        }
                        _ => {
                            return Err(CodegenError::internal(format!(
                                "rooted param {index} has an inconsistent Scoop ABI location"
                            ))
                            .with_code(scoop_source::DiagnosticCode::ICE1101));
                        }
                    }
                }
                scoop_lir::CallerRootSource::Temp(_) => {
                    let pointer = self.entry_alloca(ty, "managed_root_storage")?;
                    self.builder
                        .build_store(pointer, ty.const_zero())
                        .map_err(|error| {
                            CodegenError::backend(
                                format!("initialize managed root storage: {error}"),
                                error,
                            )
                        })?;
                    RootStorage { pointer, ty }
                }
                scoop_lir::CallerRootSource::Local(_) => {
                    return Err(CodegenError::internal(
                        "local root storage must reuse its ordinary alloca".to_string(),
                    )
                    .with_code(scoop_source::DiagnosticCode::ICE1101));
                }
            };
            if self.root_storage.insert(source, storage).is_some() {
                return Err(CodegenError::internal(
                    "complete root plans repeat one canonical source".to_string(),
                )
                .with_code(scoop_source::DiagnosticCode::ICE1101));
            }
        }
        Ok(())
    }

    pub(super) fn materialize_invoke_root_table(
        &mut self,
        roots: &[scoop_lir::ExceptionalRoot],
    ) -> Result<Option<CompilerRootTable<'ctx>>, CodegenError> {
        let context = self.context;
        let ptr = ptr_ty(context);
        let i64_type = context.i64_type();
        self.compiler_invoke_index += 1;
        if roots.is_empty() {
            return Ok(None);
        }
        let mut entries = Vec::with_capacity(roots.len());
        for exceptional in roots {
            let root = &exceptional.root;
            let storage = self.root_source_storage(root.source)?;
            let descriptor = self
                .runtime_scans
                .emit(root.scan.as_ref_scan())?
                .ok_or_else(|| {
                    CodegenError::internal("exceptional root has an empty runtime scan".to_string())
                        .with_code(scoop_source::DiagnosticCode::ICE1101)
                })?;
            entries.push((storage.pointer, descriptor));
        }

        let entry_type = context.struct_type(&[ptr.into(), ptr.into()], false);
        let entries_pointer = if entries.is_empty() {
            ptr.const_null()
        } else {
            let entry_count = u32::try_from(entries.len()).map_err(|cause| {
                CodegenError::internal_cause(
                    "compiler-root entry count exceeds u32::MAX".to_string(),
                    cause,
                )
                .with_code(scoop_source::DiagnosticCode::ICE1101)
            })?;
            let array_type = entry_type.array_type(entry_count);
            let array = self.entry_alloca(array_type.into(), "compiler_root_entries")?;
            for (entry_index, (base, scan)) in entries.into_iter().enumerate() {
                let entry_index = u64::try_from(entry_index).map_err(|cause| {
                    CodegenError::internal_cause(
                        "compiler-root entry index exceeds u64::MAX".to_string(),
                        cause,
                    )
                    .with_code(scoop_source::DiagnosticCode::ICE1101)
                })?;
                // SAFETY: `entry_index` is within the fixed entries array.
                let entry = unsafe {
                    self.builder.build_gep(
                        array_type,
                        array,
                        &[
                            context.i32_type().const_zero(),
                            context.i32_type().const_int(entry_index, false),
                        ],
                        "compiler_root_entry",
                    )
                }
                .map_err(|error| {
                    CodegenError::backend(format!("compiler-root entry GEP: {error}"), error)
                })?;
                let base_field = self
                    .builder
                    .build_struct_gep(entry_type, entry, 0, "compiler_root_base")
                    .map_err(|error| {
                        CodegenError::backend(format!("compiler-root base GEP: {error}"), error)
                    })?;
                let scan_field = self
                    .builder
                    .build_struct_gep(entry_type, entry, 1, "compiler_root_scan")
                    .map_err(|error| {
                        CodegenError::backend(format!("compiler-root scan GEP: {error}"), error)
                    })?;
                self.builder
                    .build_store(base_field, base)
                    .and_then(|_| self.builder.build_store(scan_field, scan))
                    .map_err(|error| {
                        CodegenError::backend(
                            format!("publish compiler-root entry: {error}"),
                            error,
                        )
                    })?;
            }
            array
        };
        let table_type = context.struct_type(&[ptr.into(), i64_type.into()], false);
        let table = self.entry_alloca(table_type.into(), "invoke_root_table")?;
        let entry_field = self
            .builder
            .build_struct_gep(table_type, table, 0, "invoke_root_entries")
            .map_err(|error| {
                CodegenError::backend(format!("root table entries GEP: {error}"), error)
            })?;
        let count_field = self
            .builder
            .build_struct_gep(table_type, table, 1, "invoke_root_count")
            .map_err(|error| {
                CodegenError::backend(format!("root table count GEP: {error}"), error)
            })?;
        let count = u64::try_from(roots.len()).map_err(|cause| {
            CodegenError::internal_cause("root table count exceeds u64::MAX".to_string(), cause)
                .with_code(scoop_source::DiagnosticCode::ICE1101)
        })?;
        self.builder
            .build_store(entry_field, entries_pointer)
            .and_then(|_| {
                self.builder
                    .build_store(count_field, i64_type.const_int(count, false))
            })
            .map_err(|error| {
                CodegenError::backend(format!("initialize invoke root table: {error}"), error)
            })?;
        Ok(Some(CompilerRootTable { pointer: table }))
    }

    pub(super) fn validate_compiler_root_sources(
        &self,
        sources: impl IntoIterator<Item = scoop_lir::CallerRootSource>,
    ) -> Result<(), CodegenError> {
        for source in sources {
            if matches!(source, scoop_lir::CallerRootSource::Local(_)) {
                continue;
            }
            if !self.root_storage.contains_key(&source) {
                return Err(CodegenError::internal(
                    "compiler-root edge names a source without canonical storage".to_string(),
                )
                .with_code(scoop_source::DiagnosticCode::ICE1101));
            }
        }
        Ok(())
    }

    /// Establish post-call values from the collector-updated canonical storage.
    /// Volatile loads and stores keep continuation values independent of any
    /// pre-call SSA copy on both normal and exceptional edges.
    pub(super) fn reload_published_roots(
        &self,
        sources: impl IntoIterator<Item = scoop_lir::CallerRootSource>,
    ) -> Result<Vec<ReloadedRoot<'ctx>>, CodegenError> {
        let mut reloaded = Vec::new();
        for source in sources {
            let storage = self.root_source_storage(source)?;
            let value = self
                .builder
                .build_load(storage.ty, storage.pointer, "published_root_reload")
                .map_err(|error| {
                    CodegenError::backend(format!("reload published root: {error}"), error)
                })?;
            value
                .as_instruction_value()
                .expect("a root reload is an instruction")
                .set_volatile(true)
                .map_err(|error| {
                    CodegenError::backend(
                        format!("make published root reload volatile: {error}"),
                        error,
                    )
                })?;
            reloaded.push(ReloadedRoot { storage, value });
        }
        Ok(reloaded)
    }

    pub(super) fn restore_reloaded_roots(
        &self,
        roots: Vec<ReloadedRoot<'ctx>>,
    ) -> Result<(), CodegenError> {
        for root in roots {
            let store = self
                .builder
                .build_store(root.storage.pointer, root.value)
                .map_err(|error| {
                    CodegenError::backend(format!("restore reloaded root: {error}"), error)
                })?;
            store.set_volatile(true).map_err(|error| {
                CodegenError::backend(format!("make reloaded root store volatile: {error}"), error)
            })?;
        }
        Ok(())
    }

    pub(super) fn finish_unwind_compiler_roots(&mut self) -> Result<(), CodegenError> {
        if !self.compiler_unwind_blocks.contains(&self.current_block) {
            return Ok(());
        }
        let sources = self
            .unwind_root_sources
            .get(&self.current_block)
            .cloned()
            .unwrap_or_default();
        self.validate_compiler_root_sources(sources.iter().copied())?;
        let reloaded = self.reload_published_roots(sources.iter().copied())?;
        self.restore_reloaded_roots(reloaded)?;
        if let Some(debug) = &self.debug {
            debug.borrow().invalidate_managed(
                self.builder
                    .get_insert_block()
                    .expect("unwind continuation"),
            );
            for source in sources {
                self.observe_debug_root(source)?;
            }
        }
        Ok(())
    }
}
