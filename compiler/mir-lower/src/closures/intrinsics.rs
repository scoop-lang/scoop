use super::targets::ReferenceDefinition;
use super::*;

pub(super) fn reference_definition(
    module: &hir::Module,
    callable: Option<hir::CallableTarget>,
) -> Option<ReferenceDefinition> {
    let hir::CallableTarget::Local(hir::Callable::Function(function)) = callable? else {
        return None;
    };
    match &module.functions[function].kind {
        hir::FunctionKind::Intrinsic(hir::IntrinsicFunction {
            kind: hir::IntrinsicFunctionKind::MaybeUninit(kind),
            ..
        }) => Some(ReferenceDefinition::MaybeUninit(*kind)),
        hir::FunctionKind::Intrinsic(hir::IntrinsicFunction {
            kind: hir::IntrinsicFunctionKind::NativeFunctionPointer(kind),
            ..
        }) => Some(ReferenceDefinition::NativeFunctionPointer(match kind {
            scoop_hir::NativeFunctionPointerIntrinsic::AsFunPtr => mir::CAbiCallMode::NativeSafe,
            scoop_hir::NativeFunctionPointerIntrinsic::AsGcLeaf => mir::CAbiCallMode::GcLeaf,
        })),
        _ => None,
    }
}
