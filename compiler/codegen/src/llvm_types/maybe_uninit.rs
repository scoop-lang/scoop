use super::*;

#[cfg(test)]
mod tests;

/// Explicit raw storage retains all padding. Managed slots remain AS1 pointers,
/// including when zero, so SROA and statepoint rewriting preserve GC identity.
pub(super) fn storage_type<'ctx>(
    context: &'ctx Context,
    address_space: ManagedAddressSpace,
    size: u64,
    align: u64,
    scan: &RefScan,
) -> Result<StructType<'ctx>, CodegenError> {
    let mut fields = vec![alignment_anchor(context, align)?];
    let mut references = Vec::new();
    flatten_ref_scan(scan, &mut references);
    let mut cursor = 0;
    for offset in references {
        let end = offset.checked_add(8).ok_or_else(|| {
            CodegenError::internal("MaybeUninit reference offset overflow".into())
        })?;
        if offset < cursor || offset % 8 != 0 || end > size {
            return Err(CodegenError::internal(
                "MaybeUninit reference lies outside its storage".into(),
            ));
        }
        push_raw_storage(context, &mut fields, offset - cursor, align)?;
        fields.push(managed_ptr_ty(context, address_space).into());
        cursor = end;
    }
    push_raw_storage(context, &mut fields, size - cursor, align)?;
    Ok(context.struct_type(&fields, false))
}
