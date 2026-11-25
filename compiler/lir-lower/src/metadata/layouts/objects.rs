use super::*;

/// Inline element storage of an intrinsic Array or MutableArray class.
pub(crate) fn array_element_layout(
    context: &LoweringContext,
    module: &mir::Module,
    enums: &lir::EnumDefs,
    element: &mir::Type,
) -> StorageResult<(u64, u64, lir::RefScan)> {
    let enum_shape = |id: mir::EnumId| Ok(repr_shape(context, &enums[enum_def_id(id)].repr));
    let (size, align) = size_align(context, module, &enum_shape, element)?;
    Ok((size, align, ref_scan(context, module, enums, element, 0)?))
}

pub(crate) fn class_definition_layout(
    context: &LoweringContext,
    module: &mir::Module,
    enums: &lir::EnumDefs,
    identity: lir::LayoutIdentity,
    def: &mir::ClassDef,
    id: mir::ClassId,
    class_layouts: &ClassLayouts<'_>,
) -> StorageResult<lir::Layout> {
    Ok(match &def.representation {
        mir::ClassRepresentation::Declared(declared) => {
            let layout = class_layouts.get(id)?;
            lir::Layout {
                identity,
                name: def.name.clone(),
                size: layout.size,
                align: layout.align,
                fields: declared
                    .fields
                    .iter()
                    .zip(layout.offsets.iter().copied())
                    .map(|(field, offset)| {
                        let enum_shape =
                            |id: mir::EnumId| Ok(repr_shape(context, &enums[enum_def_id(id)].repr));
                        let (_, access_align) =
                            size_align(context, module, &enum_shape, &field.ty)?;
                        Ok(lir::FieldLayout {
                            offset,
                            access_align,
                        })
                    })
                    .collect::<StorageResult<Vec<_>>>()?,
                c_layout: None,
                interior_mutable: false,
                kind: lir::LayoutKind::Plain {
                    scan: layout.scan.clone(),
                },
            }
        }
        mir::ClassRepresentation::Intrinsic(representation) => {
            let (size, align, kind) = match representation {
                mir::IntrinsicClassRepresentation::Throwable => {
                    let shape = lir::throwable_instance_shape(context.target_profile())?;
                    return Ok(lir::Layout {
                        identity,
                        name: def.name.clone(),
                        size: shape.minimum_size(),
                        align: shape.instance_alignment(),
                        fields: Vec::new(),
                        c_layout: None,
                        interior_mutable: false,
                        kind: lir::LayoutKind::Plain {
                            scan: shape.object_scan().clone(),
                        },
                    });
                }
                mir::IntrinsicClassRepresentation::Atomic(storage) => {
                    let layout = atomic_object_layout(context, storage.kind())?;
                    return Ok(lir::Layout {
                        identity,
                        name: def.name.clone(),
                        size: layout.size,
                        align: layout.align,
                        fields: Vec::new(),
                        c_layout: None,
                        interior_mutable: false,
                        kind: lir::LayoutKind::Plain { scan: layout.scan },
                    });
                }
                mir::IntrinsicClassRepresentation::Any => {
                    (0, 1, lir::IntrinsicTypeRepresentation::Any)
                }
                mir::IntrinsicClassRepresentation::Nothing => {
                    (0, 1, lir::IntrinsicTypeRepresentation::Nothing)
                }
                mir::IntrinsicClassRepresentation::String => {
                    let layout = context.string_layout();
                    (
                        layout.size,
                        layout.align,
                        lir::IntrinsicTypeRepresentation::String,
                    )
                }
                mir::IntrinsicClassRepresentation::Array { .. }
                | mir::IntrinsicClassRepresentation::MutableArray { .. } => {
                    unreachable!("intrinsic arrays use the typed ArrayType metadata arena")
                }
            };
            lir::Layout {
                identity,
                name: def.name.clone(),
                size,
                align,
                fields: Vec::new(),
                c_layout: None,
                interior_mutable: false,
                kind: lir::LayoutKind::Intrinsic(kind),
            }
        }
    })
}

/// Closure object layout: the 16-byte managed header, one non-scanned code
/// pointer at offset 16, then naturally aligned inline capture fields.
pub(crate) fn closure_shape(
    context: &LoweringContext,
    module: &mir::Module,
    enums: &lir::EnumDefs,
    def: &mir::ClosureClass,
) -> StorageResult<(Vec<u64>, u64, u64, lir::RefScan)> {
    let enum_shape = |id: mir::EnumId| Ok(repr_shape(context, &enums[enum_def_id(id)].repr));
    let mut offsets = Vec::with_capacity(def.captures.len());
    let (_, prefix) = context.closure_prefix();
    let prefix = lir::StorageGeometryV1::new(context.target_profile(), prefix.size, prefix.align)?;
    let mut cursor = lir::StorageLayoutCursorV1::with_prefix(prefix);
    for capture in &def.captures {
        let (size, align) = size_align(context, module, &enum_shape, &capture.ty)?;
        offsets.push(
            cursor
                .push(lir::StorageGeometryV1::new(
                    context.target_profile(),
                    size,
                    align,
                )?)?
                .offset(),
        );
    }
    let fields: Vec<_> = def
        .captures
        .iter()
        .map(|capture| capture.ty.clone())
        .collect();
    let scan = scan_fields(context, module, enums, &fields, &offsets, 0)?;
    let geometry = cursor.finish()?;
    Ok((offsets, geometry.size(), geometry.alignment().get(), scan))
}
