use super::*;
use scoop_identity::{ConeIdentity, NormalizedSourcePath, SourceIdentity};
use scoop_protocol::StageDumpSet;

fn source(path: &str, text: &str) -> scoop_hir::GeneratedMethodSource {
    scoop_hir::GeneratedMethodSource {
        identity: SourceIdentity::new(
            ConeIdentity::SINGLE_FILE,
            NormalizedSourcePath::new(path).unwrap(),
        ),
        name: path.to_owned(),
        text: text.to_owned().into(),
    }
}

#[test]
fn generated_dump_preserves_body_text_and_labels_each_fragment() {
    let first = "// 中文\r\n{ return \"quoted\\n\" } /* suffix */";
    let second = "{\n    return value\n}\n";
    let sources = [
        source("generated/function/first.scoop", first),
        source("generated/generic-function/second.scoop", second),
    ];
    let dumps = generated_dumps(&sources, StageDumpPolicy::Stages(StageDumpSet::all()));
    assert_eq!(dumps.len(), 1);
    assert_eq!(dumps[0].kind(), StageDumpKind::Generated);
    assert_eq!(
        dumps[0].text(),
        format!(
            "// Source: generated/function/first.scoop\n{first}\n\n\
             // Source: generated/generic-function/second.scoop\n{second}\n"
        ),
    );
    assert!(generated_dumps(&sources, StageDumpPolicy::None).is_empty());
    assert!(
        generated_dumps(
            &sources,
            StageDumpPolicy::Stages(StageDumpSet::one(StageDumpKind::Hir)),
        )
        .is_empty()
    );
}

#[test]
fn cones_without_generators_still_have_a_completed_empty_observation() {
    let dumps = generated_dumps(
        &[],
        StageDumpPolicy::Stages(StageDumpSet::one(StageDumpKind::Generated)),
    );
    assert_eq!(dumps.len(), 1);
    assert_eq!(dumps[0].kind(), StageDumpKind::Generated);
    assert_eq!(dumps[0].text(), "");
}
