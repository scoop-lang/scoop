use super::*;

#[test]
fn global_only_debug_unit_describes_existing_static_and_tls_storage() {
    let arm_linux = ValidatedBackendProfile::for_selection(
        scoop_target_data::selection(scoop_target_data::LINUX_AARCH64_GNU_ID).unwrap(),
    )
    .unwrap();
    for profile in [host_profile(), arm_linux] {
        check_global_debug_storage(profile.with_debug_info(scoop_lir::DebugInfoLevel::Full));
    }
}

fn check_global_debug_storage(profile: ValidatedBackendProfile) {
    let mut module = values_module();
    module.meta = string_metadata_for(profile.lir_target_profile());
    let ty = test_exact_type("debug.global.Long");
    module.meta.debug_types.insert(
        ty,
        scoop_lir::DebugType {
            name: "Long".into(),
            size: 8,
            align: 8,
            kind: scoop_lir::DebugTypeKind::Basic(scoop_lir::DebugEncoding::Signed),
        },
    );
    let scoop_source::DebugOrigin::Source(location) = variables::origin(1) else {
        unreachable!()
    };
    let mut symbols = Vec::new();
    for (name, thread_local) in [("counter", false), ("threadCounter", true)] {
        let identity = static_storage_identity(name);
        symbols.push(identity.symbol().to_owned());
        let global = module.globals.alloc(Global {
            address_kind: PointerKind::Raw,
            scan: RefScan::None,
            init: GlobalInit::RawStorage {
                identity,
                ty: LirType::I64,
                initializer: scoop_lir::LirConstantImage::Integer(
                    scoop_lir::LirIntegerConstant::Signed64(7),
                ),
                thread_local,
            },
        });
        module.meta.debug_globals.insert(
            global,
            scoop_lir::DebugGlobal {
                name: name.into(),
                location: location.clone(),
                ty,
            },
        );
    }
    let machine = profile.create_target_machine().unwrap();
    let context = Context::create();
    let llvm = emit_llvm_module(&context, &module, &machine, profile).unwrap();
    llvm.verify().unwrap();
    let ir = llvm.print_to_string().to_string();
    assert!(
        ir.contains("!DICompileUnit("),
        "a storage object must have its own debug unit: {ir}"
    );
    for name in ["counter", "threadCounter"] {
        assert!(
            ir.contains(&format!("!DIGlobalVariable(name: \"{name}\"")),
            "{ir}"
        );
    }
    for (index, symbol) in symbols.iter().enumerate() {
        let global = llvm.get_global(symbol).unwrap();
        assert_eq!(global.is_thread_local(), index == 1);
        assert!(ir.contains(&format!("linkageName: \"{symbol}\"")), "{ir}");
    }
    let expected = statepoint::expectations(&module).unwrap();
    statepoint::rewrite(&llvm, &machine, &expected, profile).unwrap();
    llvm.verify().unwrap();
    let object = machine
        .write_to_memory_buffer(&llvm, FileType::Object)
        .unwrap();
    let target = profile.lir_target_profile().id();
    if target == scoop_target_data::LINUX_AARCH64_GNU_ID {
        use object::{Object, ObjectSection, ObjectSymbol};
        let elf = scoop_slib::ValidatedElfObject::read(object.as_slice(), target).unwrap();
        let location = elf
            .relocations()
            .iter()
            .find(|r| {
                r.kind == object::elf::R_AARCH64_TLS_DTPREL
                    && elf
                        .file()
                        .symbol_by_index(r.symbol)
                        .unwrap()
                        .name()
                        .unwrap()
                        == symbols[1]
            })
            .expect("full DWARF contains the TLS variable's module-relative offset");
        assert_eq!(location.width, 8);
        assert_eq!(
            elf.file()
                .section_by_index(location.section)
                .unwrap()
                .name()
                .unwrap(),
            ".debug_info"
        );
    }
}
