//! Complete logical type descriptions, including open declaration binders.

use crate::*;

#[derive(Clone, Debug)]
pub enum SemanticTypeStructure {
    Unit,
    Any,
    Nominal(SemanticNominalDescription),
    Tuple(Vec<TypeId>),
    Function(FunctionType),
    Pointer(TypeId),
    NativeFunctionPointer(FunctionType),
    GcLeafNativeFunctionPointer(FunctionType),
    Parameter(TypeParamDecl),
    Companion { host: TypeParamDecl },
}

#[derive(Clone, Debug)]
pub struct SemanticNominalDescription {
    pub declaration: SourceNominalId,
    pub kind: StaticNominalKind,
    pub source_path: Option<Vec<String>>,
    pub class_hierarchy: Option<SemanticClassHierarchy>,
    pub arguments: Vec<TypeId>,
    pub parameters: Vec<TypeParamDecl>,
    /// The declared companion application, irrespective of source visibility.
    pub companion: Option<TypeId>,
    pub annotations: SemanticAnnotationTarget,
    pub origin: SemanticSourceLocation,
}

#[derive(Clone, Debug)]
pub struct SemanticClassHierarchy {
    pub modifier: ClassModifier,
    pub base: Option<TypeId>,
}
