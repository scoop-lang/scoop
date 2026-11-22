//! Locate the two contributions of a generated AMD64 runtime-function record.

use std::collections::BTreeMap;

use object::{Object, ObjectSection, ObjectSymbol, RelocationKind, RelocationTarget, SectionIndex};

use crate::CodegenError;
use crate::coff_object::error;

pub(super) struct UnwindRecord {
    pub(super) table: SectionIndex,
    pub(super) info: SectionIndex,
}

pub(super) fn read(file: &object::File<'_>) -> Result<BTreeMap<usize, UnwindRecord>, CodegenError> {
    let mut records = BTreeMap::new();
    for table in file.sections().filter(|section| {
        section
            .name()
            .is_ok_and(|name| name == ".pdata" || name.starts_with(".pdata$"))
    }) {
        if table.size() != 12 {
            return Err(error(
                "generated bridge needs one AMD64 runtime-function record per section",
            ));
        }
        let rva_target = |offset| -> Result<SectionIndex, CodegenError> {
            let relocation = table
                .relocations()
                .find(|(site, _)| *site == offset)
                .ok_or_else(|| error("missing AMD64 unwind RVA relocation"))?
                .1;
            if relocation.kind() != RelocationKind::ImageOffset || relocation.size() != 32 {
                return Err(error("invalid AMD64 unwind RVA relocation"));
            }
            match relocation.target() {
                RelocationTarget::Section(index) => Ok(index),
                RelocationTarget::Symbol(index) => file
                    .symbol_by_index(index)
                    .map_err(error)?
                    .section_index()
                    .ok_or_else(|| error("undefined AMD64 unwind target")),
                _ => Err(error("invalid AMD64 unwind target")),
            }
        };
        records.insert(
            rva_target(0)?.0,
            UnwindRecord {
                table: table.index(),
                info: rva_target(8)?,
            },
        );
    }
    Ok(records)
}
