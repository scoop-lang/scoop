use super::*;

use crate::call_resolution::specificity::{
    ApplicableDeclaration, LambdaEffectDeclaration, LambdaEffectReceiver,
};

impl Candidate {
    pub(super) fn lambda_effect_declaration(
        &self,
        state: &mut Lowerer,
        arguments: &[ast::CallArgument],
    ) -> Option<LambdaEffectDeclaration> {
        let mut signature = self.view.signature.clone();
        signature.return_type = state.infer_function_result(self.function)?;
        let receiver = match self.view.receiver {
            crate::call_resolution::candidates::ReceiverShape::None => LambdaEffectReceiver::None,
            crate::call_resolution::candidates::ReceiverShape::Extension(ty) => {
                LambdaEffectReceiver::Extension(ty)
            }
            crate::call_resolution::candidates::ReceiverShape::Instance => {
                LambdaEffectReceiver::Instance(
                    state.functions[self.function]
                        .method
                        .expect("a member has its declaring owner")
                        .owner,
                )
            }
        };
        Some(LambdaEffectDeclaration::new(
            &signature,
            receiver,
            hir::RequiredCallEffects {
                is_suspend: self.view.effects.is_suspend,
                safety: self.view.effects.attributes.safety,
                gc_effect: self.view.effects.attributes.gc_effect,
            },
            self.argument_map
                .as_ref()
                .expect("an applicable candidate has an argument map"),
            arguments,
        ))
    }
}

impl Lowerer {
    /// Return the ordinary MSC pool before literal default-kind preference.
    pub(super) fn most_specific_candidates(
        &self,
        prepared: &[Candidate],
        applicable: &[usize],
    ) -> Vec<usize> {
        let declarations = applicable
            .iter()
            .map(|&index| {
                let candidate = &prepared[index];
                ApplicableDeclaration {
                    declaration: candidate.view.forwarding(&candidate.params),
                    parameterized: candidate.own_type_param_count != 0,
                    defaults: candidate
                        .argument_map
                        .as_ref()
                        .expect("an applicable candidate has an argument map")
                        .explicit_default_count(),
                    vararg: candidate
                        .view
                        .signature
                        .value_parameters
                        .iter()
                        .any(|parameter| parameter.is_vararg()),
                }
            })
            .collect::<Vec<_>>();
        self.most_specific_declarations(&declarations)
            .into_iter()
            .map(|index| applicable[index])
            .collect()
    }
}
