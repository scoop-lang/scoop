use super::*;
use crate::Owner;

impl Lowerer {
    pub(crate) fn error_with_invariance(
        &mut self,
        code: scoop_source::DiagnosticCode,
        span: scoop_ast::Span,
        message: String,
        found: TypeId,
        expected: TypeId,
    ) {
        let mut diagnostic = scoop_ast::Diagnostic::at(code, span, message);
        diagnostic.set_file(self.current_text_file());
        if let Some(detail) = self.nominal_invariance_detail(found, expected) {
            diagnostic.notes.push(
                scoop_source::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0501,
                    detail,
                )
                .with_note(scoop_source::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::H0501,
                    "construct the target application directly or convert explicitly",
                )),
            );
        }
        self.diagnostics.push(diagnostic);
    }

    fn nominal_invariance_detail(&mut self, found: TypeId, expected: TypeId) -> Option<String> {
        let target = self.nominal_application(expected)?;
        let mut pending = vec![found];
        let mut seen = Vec::new();
        while let Some(ty) = pending.pop() {
            if seen.iter().any(|&visited| self.types_equal(visited, ty)) {
                continue;
            }
            seen.push(ty);
            if let Some(source) = self.nominal_application(ty)
                && source.template == target.template
                && source.arguments.len() == target.arguments.len()
                && let Some(index) = source
                    .arguments
                    .iter()
                    .zip(&target.arguments)
                    .position(|(&source, &target)| !self.types_equal(source, target))
            {
                let parameter = self.nominal_parameter_name(target.template, index);
                let source_application = self.type_name(source.ty);
                let target_application = self.type_name(target.ty);
                let source_argument = self.type_name(source.arguments[index]);
                let target_argument = self.type_name(target.arguments[index]);
                return Some(format!(
                    "nominal application `{}` is invariant: first differing type argument {} (`{parameter}`) is `{source_argument}` in `{source_application}`, but `{target_argument}` in `{target_application}`",
                    self.nominal_template_name(target.template),
                    index + 1,
                ));
            }
            pending.extend(self.direct_nominal_supertypes(ty));
        }
        None
    }

    fn nominal_parameter_name(&self, template: hir::SourceNominalId, index: usize) -> &str {
        if let Some(owner) = self.nominal_owners.get(&template) {
            let parameters = match *owner {
                Owner::Struct(id) => &self.structs[id].type_params,
                Owner::Class(id) => &self.classes[id].type_params,
                Owner::Enum(id) => &self.enums[id].type_params,
                Owner::Interface(id) => &self.interfaces[id].type_params,
                Owner::Object(_) => unreachable!("objects cannot have type arguments"),
            };
            return &parameters[index].name;
        }
        self.dependencies
            .as_ref()
            .and_then(|dependencies| dependencies.nominal_declaration(template))
            .expect("a resolved nominal retains its original declaration")
            .interface
            .type_parameters()
            .binders()[index]
            .name()
            .as_str()
    }
}
