use super::*;

pub(super) struct PendingContainer {
    class: hir::ClassId,
    use_process: bool,
    pub(super) origin: hir::DefinitionOrigin,
    pub(super) before: Option<hir::TestMethod>,
    pub(super) after: Option<hir::TestMethod>,
    pub(super) cases: Vec<(hir::TestMethod, Option<String>, bool)>,
}

impl PendingContainer {
    pub(super) fn new(
        class: hir::ClassId,
        origin: hir::DefinitionOrigin,
        use_process: bool,
    ) -> Self {
        Self {
            class,
            use_process,
            origin,
            before: None,
            after: None,
            cases: Vec::new(),
        }
    }
}

impl Lowerer {
    pub(super) fn finish_test_container(
        &mut self,
        pending: PendingContainer,
    ) -> Option<hir::TestContainer> {
        let constructor = self.select_test_constructor(pending.class, pending.origin.span)?;
        let identity = self
            .nominal_identity(Owner::Class(pending.class))
            .source()
            .expect("test containers are source classes");
        let mut parts = identity
            .declaration()
            .package()
            .segments()
            .iter()
            .map(|part| part.as_str())
            .collect::<Vec<_>>();
        parts.push(&self.classes[pending.class].name);
        let qualified_name = parts.join(".");
        let cone = self.current_cone();
        let cone_name = self
            .source_names
            .get(&cone)
            .cloned()
            .unwrap_or_else(|| cone.to_string());
        let suffix =
            if self.classes[pending.class].access.declared == hir::DeclaredVisibility::Private {
                format!(
                    " ({})",
                    self.intrinsic_sources[pending.origin.file as usize]
                        .identity
                        .logical_path()
                        .as_str()
                )
            } else {
                String::new()
            };
        let cases = pending
            .cases
            .into_iter()
            .map(|(method, ignore_reason, use_process)| hir::TestCase {
                display_name: format!("{cone_name}::{qualified_name}::{}{suffix}", method.name),
                method,
                use_process: pending.use_process || use_process,
                ignore_reason,
            })
            .collect();
        Some(hir::TestContainer {
            class: pending.class,
            constructor,
            qualified_name,
            origin: pending.origin,
            before: pending.before,
            after: pending.after,
            cases,
        })
    }
}
