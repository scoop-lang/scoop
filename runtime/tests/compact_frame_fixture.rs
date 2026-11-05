//! Build the five frame-ABI probes' C maps from their actual LLVM machine output.

use std::fmt::Write;
use std::path::Path;

use object::{Object, ObjectSection};

pub fn write(object: &Path, header: &Path) {
    let bytes = std::fs::read(object).unwrap();
    let file = object::File::parse(bytes.as_slice()).unwrap();
    let section = file.section_by_name(".llvm_stackmaps").unwrap();
    let parsed = scoop_slib::parse_llvm_stackmap_section_v3(section.data().unwrap()).unwrap();
    assert_eq!(parsed.record_count(), 5);
    let mut source = String::new();
    let mut entries = Vec::new();
    for function in parsed.functions() {
        assert_eq!(function.records().len(), 1);
        let site = &function.records()[0];
        let id = site.safepoint_id();
        let symbol = ["probe0", "probe1", "probe2", "probe3", "outer"][id as usize - 1];
        assert_eq!(site.locations().len(), 5);
        let root = site.locations()[3];
        assert_eq!(root, site.locations()[4]);
        assert_eq!((root.kind(), root.size()), (3, 8));
        assert!(matches!(root.dwarf_register(), 6 | 7));
        assert_eq!(root.offset() % 8, 0);
        let encoded = root.offset() as u32 | u32::from(root.dwarf_register() == 6);
        let offset = site.instruction_offset();
        let stack_size = function.stack_size();
        writeln!(source, "static const struct {{ ScoopCallableGcMapV1 map; ScoopGcSiteV1 site; uint32_t roots[2]; }} gc_{id} = {{").unwrap();
        writeln!(source, "  {{{{SCOOP_CALLABLE_GC_MAP_MAGIC_V1, SCOOP_RUNTIME_METADATA_ABI_VERSION_V1, 32}}, {stack_size}, 1, 2}},").unwrap();
        writeln!(
            source,
            "  {{{id}, {offset}, 0}}, {{1, UINT32_C({encoded})}} }};"
        )
        .unwrap();
        entries.push(format!(
            "{{(uintptr_t){symbol} + {offset}, &gc_{id}.map, &gc_{id}.site}}"
        ));
    }
    writeln!(
        source,
        "static void init_probe_maps(ScoopStackMapIndex *index) {{"
    )
    .unwrap();
    writeln!(
        source,
        "  const ScoopGcPcEntry entries[] = {{{}}};",
        entries.join(",")
    )
    .unwrap();
    writeln!(
        source,
        "  index->record_count = sizeof entries / sizeof *entries;"
    )
    .unwrap();
    writeln!(
        source,
        "  index->records = malloc(sizeof entries); assert(index->records != NULL);"
    )
    .unwrap();
    writeln!(source, "  memcpy(index->records, entries, sizeof entries);").unwrap();
    writeln!(
        source,
        "  uintptr_t duplicate; assert(scoop_stackmap_finish_index(index, &duplicate));\n}}"
    )
    .unwrap();
    std::fs::write(header, source).unwrap();
}
