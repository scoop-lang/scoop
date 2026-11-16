use super::*;

impl FunctionIdentityBuilder<'_> {
    pub(super) fn immediate_parent(
        &self,
        function: hir::FunctionId,
        root: hir::LexicalDefinitionRoot,
        path: &StructuralDefinitionPath,
    ) -> Result<Option<hir::FunctionId>, PersistentFunctionIdentityError> {
        let mut candidate = None;
        for (_, declaration) in self.lowerer.local_functions.iter() {
            let Some((source_function, source_root)) = declaration.source() else {
                continue;
            };
            self.consider_parent(
                function,
                root,
                path,
                source_function,
                source_root,
                &declaration.definition_path,
                &mut candidate,
            )?;
        }
        for (_, declaration) in self.lowerer.lambdas.iter() {
            let Some((source_function, source_root)) = declaration.definition.source() else {
                continue;
            };
            self.consider_parent(
                function,
                root,
                path,
                source_function,
                source_root,
                &declaration.definition_path,
                &mut candidate,
            )?;
        }
        for (_, declaration) in self.lowerer.anonymous_functions.iter() {
            let Some((source_function, source_root)) = declaration.definition.source() else {
                continue;
            };
            self.consider_parent(
                function,
                root,
                path,
                source_function,
                source_root,
                &declaration.definition_path,
                &mut candidate,
            )?;
        }
        Ok(candidate.map(|(_, parent)| parent))
    }

    #[allow(clippy::too_many_arguments)]
    fn consider_parent(
        &self,
        function: hir::FunctionId,
        root: hir::LexicalDefinitionRoot,
        path: &StructuralDefinitionPath,
        possible_parent: hir::FunctionId,
        possible_root: hir::LexicalDefinitionRoot,
        possible_path: &StructuralDefinitionPath,
        candidate: &mut Option<(usize, hir::FunctionId)>,
    ) -> Result<(), PersistentFunctionIdentityError> {
        let possible_segments = possible_path.segments();
        if possible_root != root
            || possible_segments.len() >= path.segments().len()
            || !path.segments().starts_with(possible_segments)
        {
            return Ok(());
        }
        if possible_parent == function {
            return Err(self.failure(function, Detail::CyclicLexicalParent));
        }
        match candidate {
            Some((length, parent)) if *length == possible_segments.len() => {
                if *parent != possible_parent {
                    return Err(self.failure(function, Detail::AmbiguousLexicalParent));
                }
            }
            Some((length, _)) if *length > possible_segments.len() => {}
            _ => *candidate = Some((possible_segments.len(), possible_parent)),
        }
        Ok(())
    }
}
