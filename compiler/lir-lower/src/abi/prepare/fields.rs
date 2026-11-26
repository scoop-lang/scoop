//! Share complete field storage nodes instead of expanding nested leaves.

use super::*;

impl SignatureBuilder<'_> {
    pub(super) fn aggregate_fields(
        &mut self,
        ty: &lir::LirType,
    ) -> StorageResult<Vec<lir::AbiAggregateField>> {
        let fields = match ty {
            lir::LirType::Struct(id) => {
                let definition = &self.structs[*id];
                if let lir::StructRepresentation::Intrinsic(
                    lir::IntrinsicTypeRepresentation::MaybeUninit { value, .. },
                ) = &definition.representation
                {
                    vec![(0, definition.align, value.as_ref().clone())]
                } else {
                    (0..definition.field_count())
                        .map(|index| {
                            let layout = definition
                                .field_layout(index)
                                .expect("existing field has a layout");
                            (
                                layout.offset,
                                layout.access_align,
                                definition
                                    .field_storage_type(index)
                                    .expect("existing field has a type"),
                            )
                        })
                        .collect()
                }
            }
            lir::LirType::Aggregate(fields) => {
                let mut cursor = lir::StorageLayoutCursorV1::new(
                    self.context.target_profile(),
                    lir::StoragePlacementPolicyV1::Ordinary,
                )?;
                fields
                    .iter()
                    .map(|ty| {
                        let (size, alignment) =
                            safepoints::lir_size_align(self.context, ty, self.structs, self.enums)?;
                        let placement = cursor.push(lir::StorageGeometryV1::new(
                            self.context.target_profile(),
                            size,
                            alignment,
                        )?)?;
                        Ok((
                            placement.offset(),
                            placement.access_alignment().get(),
                            ty.clone(),
                        ))
                    })
                    .collect::<StorageResult<Vec<_>>>()?
            }
            _ => {
                return Err(crate::StorageLoweringError::InvalidRepresentation(
                    "aggregate ABI classification requires GC-free value storage",
                ));
            }
        };
        fields
            .into_iter()
            .map(|(offset, access_alignment, ty)| {
                Ok(lir::AbiAggregateField {
                    offset,
                    access_alignment,
                    value: self.value(ty)?,
                })
            })
            .collect()
    }
}
