use super::*;

impl Lowerer {
    pub(crate) fn validate_context_query_intrinsic(&mut self) {
        let Some(&(function, _)) = self
            .intrinsic_functions
            .get(&hir::IntrinsicFunctionKind::ContextOrNone)
        else {
            return;
        };
        self.current_file = self.function_files[&function];
        let declaration = &self.functions[function];
        let signature = &self.signatures[&function];
        let valid = declaration.method.is_none()
            && !self.extension_receivers.contains_key(&function)
            && !signature.is_suspend
            && signature.owner_type_param_count == 0
            && signature.type_params.len() == 1
            && signature.type_params[0].kind() == hir::TypeParamKind::Ref
            && signature.params.is_empty()
            && self
                .as_option(signature.return_ty)
                .is_some_and(|payload| self.is_type_param(payload, 0))
            && signature.attributes == hir::FunctionAttributes::default();
        if !valid {
            self.error(
                declaration.span,
                "intrinsic `context_or_none` must be an ordinary top-level `fun <T : ref> contextOrNone(): Option<T>`".into(),
            );
        }
    }

    pub(super) fn missing_context_constructor(
        &mut self,
        files: &[ast::Span],
        throwable: ClassId,
    ) -> Option<hir::ClassConstructorId> {
        let Some(crate::NominalTarget::Class(class)) =
            self.core_nominal_target("MissingContextException")
        else {
            let file = self.core_diagnostic_file();
            self.current_file = file;
            self.error(
                files[file],
                "scoop.core must define class `MissingContextException`".into(),
            );
            return None;
        };
        self.current_file = self.class_files[&class];
        let constructor = self.classes[class]
            .constructors
            .iter()
            .copied()
            .find(|constructor| {
                let parameters = &self.class_constructors[*constructor].parameters;
                parameters.len() == 1 && self.as_option(parameters[0].ty) == Some(self.string)
            });
        if self.classes[class].modifier != hir::ClassModifier::Final
            || !self.classes[class].type_params.is_empty()
            || !self.class_descends_from(class, throwable)
            || constructor.is_none()
        {
            self.error(self.classes[class].span, "`MissingContextException` must be a non-generic final Throwable subtype with a `(String?)` constructor".into());
            return None;
        }
        constructor
    }
}
