use std::fmt;

#[derive(Debug)]
pub enum CurrentConeLirStageError {
    AbiPlanning(scoop_codegen::AbiPlanningError),
    Identity(scoop_identity::IdentityValidationError),
    DiagnosticCatalog(scoop_identity::ExactTypeDiagnosticCatalogError),

    DependencyProjection(scoop_slib::CrossConeLirSelectionProjectionError),
    Lowering(scoop_lir_lower::LirLoweringError),
    CrossConeBridge(scoop_lir_lower::CrossConeLirBridgeLoweringError),
}

impl fmt::Display for CurrentConeLirStageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AbiPlanning(source) => source.fmt(formatter),
            Self::Identity(source) => source.fmt(formatter),
            Self::DiagnosticCatalog(source) => source.fmt(formatter),

            Self::DependencyProjection(source) => source.fmt(formatter),
            Self::Lowering(source) => source.fmt(formatter),
            Self::CrossConeBridge(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for CurrentConeLirStageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::AbiPlanning(source) => source,
            Self::Identity(source) => source,
            Self::DiagnosticCatalog(source) => source,

            Self::DependencyProjection(source) => source,
            Self::Lowering(source) => source,
            Self::CrossConeBridge(source) => source,
        })
    }
}

#[derive(Debug)]
pub enum CurrentConeMirStageError {
    DependencyProjection(scoop_slib::CrossConeMirSelectionProjectionError),
    Lowering(scoop_mir_lower::CurrentConeMirLoweringError),
    ProductionSection(scoop_mir_lower::MirProductionLoweringError),
    CrossConeBridge(scoop_mir_lower::CrossConeMirBridgeLoweringError),
    Sealing(scoop_mir::ConeMirInputError),
}

impl fmt::Display for CurrentConeMirStageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DependencyProjection(source) => source.fmt(formatter),
            Self::Lowering(source) => source.fmt(formatter),
            Self::ProductionSection(source) => source.fmt(formatter),
            Self::CrossConeBridge(source) => source.fmt(formatter),
            Self::Sealing(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for CurrentConeMirStageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::DependencyProjection(source) => source,
            Self::Lowering(source) => source,
            Self::ProductionSection(source) => source,
            Self::CrossConeBridge(source) => source,
            Self::Sealing(source) => source,
        })
    }
}

impl From<scoop_lir_lower::LirLoweringError> for CurrentConeLirStageError {
    fn from(error: scoop_lir_lower::LirLoweringError) -> Self {
        Self::Lowering(error)
    }
}
impl From<scoop_codegen::AbiPlanningError> for CurrentConeLirStageError {
    fn from(error: scoop_codegen::AbiPlanningError) -> Self {
        Self::AbiPlanning(error)
    }
}

impl CurrentConeMirStageError {
    pub fn code(&self) -> scoop_source::DiagnosticCode {
        use scoop_source::DiagnosticCode as C;
        match self {
            Self::DependencyProjection(e) => match e {
                scoop_slib::CrossConeMirSelectionProjectionError::Resource(_) => C::E2205,
                _ => C::ICE0701,
            },
            Self::Lowering(_)
            | Self::ProductionSection(_)
            | Self::CrossConeBridge(_)
            | Self::Sealing(_) => C::ICE0701,
        }
    }
}

impl CurrentConeLirStageError {
    pub fn code(&self) -> scoop_source::DiagnosticCode {
        use scoop_source::DiagnosticCode as C;
        match self {
            Self::Identity(_) | Self::DiagnosticCatalog(_) => C::ICE0401,
            Self::AbiPlanning(_) | Self::DependencyProjection(_) | Self::CrossConeBridge(_) => {
                C::ICE1201
            }
            Self::Lowering(e) => match e {
                scoop_lir_lower::LirLoweringError::Capability(_) => C::E2411,
                _ => C::ICE1201,
            },
        }
    }
}
