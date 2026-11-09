use super::*;
use object::{Object, ObjectSection, ObjectSymbol};
use scoop_process::CommandExt;

#[test]
fn darwin_archives_prepare_empty_code_labels() {
    let clang = std::env::var_os("SCOOP_TEST_CLANG").unwrap_or_else(|| "clang".into());
    for target in [
        scoop_target_data::DARWIN_AARCH64,
        scoop_target_data::DARWIN_X86_64,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("empty-code.s");
        let object = directory.path().join("empty-code.o");
        std::fs::write(
            &source,
            ".section __TEXT,__text,regular,pure_instructions\n\
             unused_local:\n\
             .globl _empty_public\n\
             _empty_public:\n\
             .section __DATA,__data\n\
             .globl _value\n\
             _value:\n\
             .quad 42\n",
        )
        .unwrap();
        let output = std::process::Command::new(&clang)
            .args(["-target", target.contract().canonical_triple(), "-c"])
            .arg(&source)
            .arg("-o")
            .arg(&object)
            .scoop_output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let original = std::fs::read(&object).unwrap();
        let before = object::File::parse(original.as_slice()).unwrap();
        assert!(
            before
                .symbols()
                .any(|symbol| symbol.name().unwrap() == "unused_local")
        );
        let prepared = prepare_object(target, Cow::Borrowed(&original)).unwrap();
        let (kind, extension) = format(target);
        let archive_path = directory.path().join(format!("runtime.{extension}"));
        write(
            &archive_path,
            kind,
            &[NewArchiveMember::new(
                prepared,
                &ar_archive_writer::DEFAULT_OBJECT_READER,
                "0.o".to_owned(),
            )],
        )
        .unwrap();
        let bytes = std::fs::read(&archive_path).unwrap();
        let archive = object::read::archive::ArchiveFile::parse(bytes.as_slice()).unwrap();
        let members: Vec<_> = archive.members().map(Result::unwrap).collect();
        assert_eq!(members.len(), 1);
        let file = object::File::parse(members[0].data(bytes.as_slice()).unwrap()).unwrap();
        assert!(
            file.symbols()
                .all(|symbol| symbol.name().unwrap() != "unused_local")
        );
        for name in ["_empty_public", "_value"] {
            assert!(file.symbols().any(|symbol| symbol.name().unwrap() == name));
        }
        assert_eq!(
            file.section_by_name("__data").unwrap().data().unwrap(),
            42u64.to_le_bytes()
        );
    }
}
