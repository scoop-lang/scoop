//! Ordinary-effect defaults only compare otherwise identical declarations.

use scoop_ast as ast;
use scoop_hir as hir;

use crate::Lowerer;
use crate::call_resolution::arguments::{CandidateArgumentMap, SourceInputKind};
use crate::call_resolution::candidates::{
    DeclarationSignature, ValueParameterCalling, VarargOmission,
};

#[derive(Clone, Copy)]
pub(crate) enum LambdaEffectReceiver {
    None,
    Instance(hir::TypeId),
    Extension(hir::TypeId),
}

/// Comparison scratch retains declaration types, including omitted parameters
/// and results. MSC's source-mapped parameter types alone cannot prove symmetry.
pub(crate) struct LambdaEffectDeclaration {
    owner_parameters: Vec<hir::TypeParamDecl>,
    callable_parameters: Vec<hir::TypeParamDecl>,
    availability: Option<hir::ConditionalInterfaceGuard>,
    receiver: LambdaEffectReceiver,
    parameters: Vec<(hir::TypeId, ParameterCalling)>,
    result: hir::TypeId,
    effects: hir::RequiredCallEffects,
    lambda_parameters: Vec<(usize, usize)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParameterCalling {
    Required,
    Default,
    Vararg { has_default: bool },
}

impl LambdaEffectDeclaration {
    pub(crate) fn new<D>(
        signature: &DeclarationSignature<D>,
        receiver: LambdaEffectReceiver,
        effects: hir::RequiredCallEffects,
        mapping: &CandidateArgumentMap<D>,
        arguments: &[ast::CallArgument],
    ) -> Self {
        let lambda_parameters = mapping
            .source_order
            .iter()
            .filter_map(|&input| {
                let (parameter, kind) = mapping.source_binding(input);
                (kind == SourceInputKind::Value
                    && matches!(
                        arguments[input.index()].expression,
                        ast::Expr::Lambda {
                            is_suspend: false,
                            ..
                        }
                    ))
                .then_some((input.index(), parameter.index()))
            })
            .collect();
        Self {
            owner_parameters: signature.owner_parameters.clone(),
            callable_parameters: signature.callable_parameters.clone(),
            availability: signature.availability,
            receiver,
            parameters: signature
                .value_parameters
                .iter()
                .map(|parameter| {
                    let calling = match parameter.calling {
                        ValueParameterCalling::Required => ParameterCalling::Required,
                        ValueParameterCalling::Default(_) => ParameterCalling::Default,
                        ValueParameterCalling::Vararg { ref omission, .. } => {
                            ParameterCalling::Vararg {
                                has_default: matches!(omission, VarargOmission::Default(_)),
                            }
                        }
                    };
                    (parameter.ty, calling)
                })
                .collect(),
            result: signature.return_type,
            effects,
            lambda_parameters,
        }
    }
}

impl Lowerer {
    pub(crate) fn prefer_ordinary_lambdas(
        &self,
        candidates: &[usize],
        declarations: &[Option<LambdaEffectDeclaration>],
    ) -> Vec<usize> {
        candidates
            .iter()
            .copied()
            .filter(|&candidate| {
                !candidates.iter().copied().any(|other| {
                    other != candidate
                        && match (&declarations[other], &declarations[candidate]) {
                            (Some(left), Some(right)) => {
                                self.ordinary_lambda_dominates(left, right)
                            }
                            _ => false,
                        }
                })
            })
            .collect()
    }

    fn ordinary_lambda_dominates(
        &self,
        left: &LambdaEffectDeclaration,
        right: &LambdaEffectDeclaration,
    ) -> bool {
        if left.lambda_parameters.is_empty()
            || left.lambda_parameters != right.lambda_parameters
            || left.owner_parameters.len() != right.owner_parameters.len()
            || left.callable_parameters.len() != right.callable_parameters.len()
            || left.parameters.len() != right.parameters.len()
            || left.effects.safety != right.effects.safety
            || left.effects.gc_effect != right.effects.gc_effect
        {
            return false;
        }
        let binders = left
            .owner_parameters
            .iter()
            .chain(&left.callable_parameters)
            .zip(
                right
                    .owner_parameters
                    .iter()
                    .chain(&right.callable_parameters),
            )
            .collect::<Vec<_>>();
        let pairs = binders
            .iter()
            .map(|(left, right)| (left.id, right.id))
            .collect::<Vec<_>>();
        let equal = |left, right| self.signature_types_equal(left, right, &pairs);
        let receiver_equal = match (left.receiver, right.receiver) {
            (LambdaEffectReceiver::None, LambdaEffectReceiver::None) => true,
            (LambdaEffectReceiver::Instance(left), LambdaEffectReceiver::Instance(right))
            | (LambdaEffectReceiver::Extension(left), LambdaEffectReceiver::Extension(right)) => {
                equal(left, right)
            }
            _ => false,
        };
        let availability_equal = match (left.availability, right.availability) {
            (None, None) => true,
            (Some(left), Some(right)) => {
                left.declaration == right.declaration && equal(left.owner, right.owner)
            }
            _ => false,
        };
        if !receiver_equal
            || !availability_equal
            || !equal(left.result, right.result)
            || !binders
                .iter()
                .all(|(left, right)| self.same_parameter_conditions(left, right, &pairs))
        {
            return false;
        }
        let mut better = false;
        for (index, ((left_ty, left_calling), (right_ty, right_calling))) in
            left.parameters.iter().zip(&right.parameters).enumerate()
        {
            if left_calling != right_calling {
                return false;
            }
            if !left
                .lambda_parameters
                .iter()
                .any(|(_, parameter)| *parameter == index)
            {
                if !equal(*left_ty, *right_ty) {
                    return false;
                }
                continue;
            }
            let (hir::Type::Function(left), hir::Type::Function(right)) =
                (&self.types[*left_ty], &self.types[*right_ty])
            else {
                if !equal(*left_ty, *right_ty) {
                    return false;
                }
                continue;
            };
            let (left, right) = (&self.function_types[*left], &self.function_types[*right]);
            if left.receiver.is_some() != right.receiver.is_some()
                || left.arity() != right.arity()
                || !left
                    .parameters()
                    .zip(right.parameters())
                    .all(|(left, right)| equal(*left, *right))
                || !equal(left.return_type, right.return_type)
            {
                return false;
            }
            match (left.is_suspend, right.is_suspend) {
                (false, true) => better = true,
                (true, false) => return false,
                _ => {}
            }
        }
        better
    }
}
