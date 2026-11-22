//! Exercise real GNU assembler output without loading LLVM.

use std::fmt::Write;
use std::process::Command;

use super::*;
use crate::link_object::{
    ObjectRelocationShapeV1, ObjectStackmapSectionError, ObjectSymbolAttributesV1,
    ObjectSymbolKindV1, PlannedStrongObjectSymbolRoleV1, VerifiedObjectRelocationShapeV1,
    VerifiedRelocationTargetV1, read_member_object_definitions_v1, read_object_stackmap_section_v3,
    verify_current_cone_strong_relocation_closure_v1, verify_member_object_relocations_v1,
};

#[test]
fn builtin_elf_preserves_large_section_indices_names_groups_and_signed_addends() {
    let mut assembly = String::new();
    for index in 0..300 {
        assembly.push_str(&format!(
            ".section .rodata.scoop_long_section_name_{index},\"a\",@progbits\n.quad {index}\n"
        ));
    }
    assembly.push_str(
        r#"
.section .data,"aw",@progbits
.quad external + 17
.text
.globl entry
.hidden entry
.type entry,@function
entry:
call external
ret
.size entry,.-entry
.section .tdata,"awT",@progbits
.globl late_tls
.hidden late_tls
.type late_tls,@tls_object
late_tls: .quad 42
.size late_tls,8
.section .rodata.odr,"aG",@progbits,odr,comdat
.weak odr
.hidden odr
.type odr,@object
odr: .quad 7
.size odr,8
.section .note.GNU-stack,"",@progbits
"#,
    );
    let bytes = assemble(&assembly);
    for target in [
        scoop_target_data::LINUX_X86_64_GNU_ID,
        scoop_target_data::LINUX_X86_64_MUSL_ID,
    ] {
        let envelope = validate_elf_object_envelope_v1(&bytes, target).unwrap();
        assert_eq!(envelope.target(), target);
        assert!(
            envelope
                .sections()
                .iter()
                .any(|s| s.section_name() == b".rodata.scoop_long_section_name_299")
        );
        let tls = envelope
            .symbols()
            .iter()
            .find(|s| s.name() == b"late_tls")
            .unwrap();
        assert!(tls.section_ordinal().unwrap().get() > 255);
        assert_eq!(tls.kind(), ObjectSymbolKindV1::ExternalStrongDefinition);
        assert!(
            matches!(tls.attributes(), ObjectSymbolAttributesV1::Elf { size: 8, info, other }
            if info & 15 == object::elf::STT_TLS && other == object::elf::STV_HIDDEN)
        );
        let odr = envelope
            .symbols()
            .iter()
            .find(|s| s.name() == b"odr")
            .unwrap();
        assert_eq!(odr.kind(), ObjectSymbolKindV1::ExternalWeakDefinition);
        for relocation in envelope.relocations() {
            assert_eq!(relocation.encoded_value(), 0);
            let ObjectRelocationShapeV1::ElfRela {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                kind,
                addend,
                storage_width,
                ..
            } = relocation.shape()
            else {
                panic!("RELA")
            };
            match kind {
                object::elf::R_X86_64_64 => assert_eq!((addend, storage_width), (17, 8)),
                object::elf::R_X86_64_PLT32 => assert_eq!((addend, storage_width), (-4, 4)),
                _ => panic!("unexpected relocation {kind}"),
            }
        }
        assert_eq!(envelope.relocations().len(), 2);
    }
}

#[test]
fn stackmap_requires_one_relocation_at_each_function_address() {
    let assembly = r#"
.text
.globl entry
.type entry,@function
entry: ret
.size entry,.-entry
.section .llvm_stackmaps,"a",@progbits
.long 3, 1, 0, 0
.Laddress: .quad 0
.quad 0, 0
.reloc .Laddress, R_X86_64_64, entry
"#;
    let bytes = assemble(assembly);
    let sections = builtin::sections(&bytes, scoop_target_data::LINUX_X86_64_GNU_ID).unwrap();
    let stackmap = read_object_stackmap_section_v3(&bytes, &sections)
        .unwrap()
        .unwrap();
    assert_eq!(stackmap.functions().len(), 1);
    assert_eq!(
        stackmap.functions()[0].parsed().function_address_offset(),
        16
    );

    let bytes = assemble(&format!("{assembly}.reloc .Laddress, R_X86_64_64, entry\n"));
    let sections = builtin::sections(&bytes, scoop_target_data::LINUX_X86_64_GNU_ID).unwrap();
    assert_eq!(
        read_object_stackmap_section_v3(&bytes, &sections),
        Err(ObjectStackmapSectionError::DuplicateRelocation { offset: 16 })
    );
}

#[test]
fn same_offset_rela_records_keep_separate_symbol_bindings() {
    let target = scoop_target_data::LINUX_X86_64_GNU;
    let fixture = crate::link_object::symbol_verification::tests::build_target_fixture(
        target,
        scoop_identity::ConeIdentity::CORE,
        "entry",
        false,
    );
    let mut assembly = String::from(
        r#"
.text
.Lsite: .long 0
.reloc .Lsite, R_X86_64_PC32, .text
.globl external
.reloc .Lsite, R_X86_64_PC32, external
"#,
    );
    for symbol in fixture.symbols.symbols() {
        let name = std::str::from_utf8(symbol.macho_name()).unwrap();
        let offset = match symbol.role() {
            PlannedStrongObjectSymbolRoleV1::PrimaryDefinition { .. } => {
                writeln!(assembly, ".type {name},@function\n.size {name},4").unwrap();
                0
            }
            PlannedStrongObjectSymbolRoleV1::AtomBoundaryStart { .. } => 0,
            PlannedStrongObjectSymbolRoleV1::AtomBoundaryEnd { .. } => 4,
        };
        writeln!(
            assembly,
            ".globl {name}\n.hidden {name}\n.set {name}, .Lsite+{offset}"
        )
        .unwrap();
    }

    let bytes = assemble(&assembly);
    let sections = builtin::sections(&bytes, target.id()).unwrap();
    let definitions =
        read_member_object_definitions_v1(&bytes, sections, &fixture.symbols).unwrap();
    let member = verify_member_object_relocations_v1(definitions).unwrap();
    let closure = verify_current_cone_strong_relocation_closure_v1(vec![member]).unwrap();
    let mut relocs = closure.members()[0].relocations_for_atom(fixture.atom);
    let (index, local) = relocs.next().unwrap();
    assert!(matches!(
        local.shape(),
        VerifiedObjectRelocationShapeV1::ElfRela {
            target: VerifiedRelocationTargetV1::SectionBase { .. },
            ..
        }
    ));
    assert!(closure.bindings_for_relocation(index, local).is_empty());
    let (index, external) = relocs.next().unwrap();
    assert_eq!(local.offset_within_atom(), external.offset_within_atom());
    assert_eq!(
        closure.bindings_for_relocation(index, external)[0].symbol(),
        b"external"
    );
    assert!(relocs.next().is_none());
}

fn assemble(assembly: &str) -> Vec<u8> {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("facts.s");
    let output = directory.path().join("facts.o");
    std::fs::write(&source, assembly).unwrap();
    let result = Command::new("gcc")
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::read(output).unwrap()
}
