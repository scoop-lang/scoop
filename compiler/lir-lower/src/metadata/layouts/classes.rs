//! Shared declared-object geometry for one immutable MIR module and target.

use super::*;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct DeclaredClassLayout {
    pub(crate) offsets: Vec<u64>,
    pub(crate) size: u64,
    pub(crate) align: u64,
    pub(crate) scan: lir::RefScan,
}

/// Compute only layouts actually consumed by descriptors, bodies or debug info.
/// Shared records keep field access allocation-free after the first query.
/// The borrowed inputs cannot change while a cached result is in use.
pub(crate) struct ClassLayouts<'a> {
    context: &'a LoweringContext,
    module: &'a mir::Module,
    enums: &'a lir::EnumDefs,
    declared: RefCell<HashMap<mir::ClassId, Rc<DeclaredClassLayout>>>,
}

impl<'a> ClassLayouts<'a> {
    pub(crate) fn new(
        context: &'a LoweringContext,
        module: &'a mir::Module,
        enums: &'a lir::EnumDefs,
    ) -> Self {
        Self {
            context,
            module,
            enums,
            declared: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn get(&self, class: mir::ClassId) -> StorageResult<Rc<DeclaredClassLayout>> {
        if let Some(layout) = self.declared.borrow().get(&class) {
            return Ok(layout.clone());
        }
        let declared = self.definition(class)?;
        let (offsets, size, align, scan) =
            declared_class_layout(self.context, self.module, self.enums, declared)?;
        let layout = Rc::new(DeclaredClassLayout {
            offsets,
            size,
            align,
            scan,
        });
        self.declared.borrow_mut().insert(class, layout.clone());
        Ok(layout)
    }

    fn definition(&self, class: mir::ClassId) -> StorageResult<&'a mir::DeclaredClass> {
        let definition = self
            .module
            .classes
            .values()
            .nth(u32::from(class.into_raw()) as usize)
            .ok_or(StorageLoweringError::MissingClass(class))?;
        match &definition.representation {
            mir::ClassRepresentation::Declared(declared) => Ok(declared),
            _ => Err(StorageLoweringError::InvalidRepresentation(
                "declared object storage requires a declared class",
            )),
        }
    }

    fn field(&self, class: mir::ClassId, index: u32) -> StorageResult<&'a mir::Field> {
        self.definition(class)?
            .fields
            .get(index as usize)
            .ok_or(StorageLoweringError::MissingClassField { class, index })
    }

    pub(crate) fn field_offset(
        &self,
        class: mir::ClassId,
        index: u32,
        ty: &mir::Type,
    ) -> StorageResult<u64> {
        if &self.field(class, index)?.ty != ty {
            return Err(StorageLoweringError::ClassFieldTypeMismatch { class, index });
        }
        self.get(class)?
            .offsets
            .get(index as usize)
            .copied()
            .ok_or(StorageLoweringError::MissingClassField { class, index })
    }
}

/// Preserve the complete base prefix, including tail padding; zero-sized
/// fields keep offset zero and do not advance the allocation cursor.
fn class_shape_with_prefix(
    context: &LoweringContext,
    module: &mir::Module,
    enums: &lir::EnumDefs,
    def: &mir::DeclaredClass,
) -> StorageResult<(Vec<u64>, u64, u64, lir::RefScan)> {
    let mut chain = vec![def];
    let mut current = def;
    let header = context.object_header_layout();
    let mut prefix =
        lir::StorageGeometryV1::new(context.target_profile(), header.size, header.align)?;
    let mut prefix_scan = lir::RefScan::None;
    let mut seen = std::collections::HashSet::new();
    while let Some(base) = current.base_class {
        // Integer subtype boxes share one scalar payload with the parent box.
        // Their runtime parent relation does not append inherited fields.
        if module
            .meta
            .boxed_types
            .iter()
            .any(|boxed| boxed.class() == base)
        {
            break;
        }
        if !seen.insert(base) {
            return Err(StorageLoweringError::InvalidRepresentation(
                "class base cycle",
            ));
        }
        current = match &module
            .classes
            .values()
            .nth(u32::from(base.into_raw()) as usize)
            .ok_or(StorageLoweringError::MissingClass(base))?
            .representation
        {
            mir::ClassRepresentation::Declared(base) => base,
            mir::ClassRepresentation::Intrinsic(mir::IntrinsicClassRepresentation::Throwable) => {
                let shape = lir::throwable_instance_shape(context.target_profile())?;
                prefix = lir::StorageGeometryV1::new(
                    context.target_profile(),
                    shape.minimum_size(),
                    shape.instance_alignment(),
                )?;
                prefix_scan = shape.object_scan().clone();
                break;
            }
            mir::ClassRepresentation::Intrinsic(_) => {
                return Err(StorageLoweringError::InvalidRepresentation(
                    "a declared class requires a declared base",
                ));
            }
        };
        chain.push(current);
    }
    let enum_shape = |id: mir::EnumId| Ok(repr_shape(context, &enums[enum_def_id(id)].repr));
    let mut offsets = Vec::new();
    let mut inherited_fields: &[mir::Field] = &[];
    for definition in chain.into_iter().rev() {
        let fields = definition.fields.as_slice();
        let inherited_count = offsets.len();
        if inherited_count > fields.len()
            || fields
                .iter()
                .zip(inherited_fields)
                .any(|(field, inherited)| field.name != inherited.name || field.ty != inherited.ty)
        {
            return Err(StorageLoweringError::InvalidRepresentation(
                "class fields do not preserve the complete base prefix",
            ));
        }
        let mut cursor = lir::StorageLayoutCursorV1::with_prefix(prefix);
        for field in &fields[inherited_count..] {
            let (size, align) = size_align(context, module, &enum_shape, &field.ty)?;
            let geometry = lir::StorageGeometryV1::new(context.target_profile(), size, align)?;
            offsets.push(cursor.push(geometry)?.offset());
        }
        prefix = cursor.finish()?;
        inherited_fields = fields;
    }
    Ok((
        offsets,
        prefix.size(),
        prefix.alignment().get(),
        prefix_scan,
    ))
}

fn declared_class_layout(
    context: &LoweringContext,
    module: &mir::Module,
    enums: &lir::EnumDefs,
    declared: &mir::DeclaredClass,
) -> StorageResult<(Vec<u64>, u64, u64, lir::RefScan)> {
    let (offsets, size, align, prefix_scan) =
        class_shape_with_prefix(context, module, enums, declared)?;
    let mut scans = Vec::with_capacity(declared.fields.len() + 1);
    scans.push(prefix_scan);
    for (field, &offset) in declared.fields.iter().zip(&offsets) {
        scans.push(ref_scan(context, module, enums, &field.ty, offset)?);
    }
    let scan = sequence(scans)?;
    Ok((offsets, size, align, scan))
}
