use super::*;

#[derive(Debug)]
pub enum LayoutAbiSectionError {
    DuplicateProvider(ConeIdentity),
    DependencyTarget {
        provider: ConeIdentity,
    },
    NonCanonicalSelected {
        index: usize,
    },
    SelectedCurrentProvider,
    SelectionProvider {
        selected: ConeIdentity,
        exported: ConeIdentity,
    },
    SelectionTarget {
        selected: crate::LirTargetProfile,
        exported: crate::LirTargetProfile,
    },
    MissingSemanticProvider(ConeIdentity),
    MissingPhysicalProvider(ConeIdentity),
    MissingPhysicalSubject(crate::ExternalStrongShapeSubjectV1),
    MissingPhysicalSemantic(LayoutAbiDependencyV1),
    Exports(LayoutAbiExportConstituentsError),
    Semantic(LayoutAbiSemanticClosureError),
    Layout(crate::ExactLayoutTableError),
    Descriptor(crate::ExactDescriptorTableError),
    Dispatch(crate::ExactDispatchTableError),
    DescriptorDispatch(scoop_identity::PersistentDispatchTableId),
    Callable(crate::ExactCallableAbiTableError),
    ShapeSupport,
    ShapeSupportTable(crate::ParamFreeShapeSupportTableError),
    Physical(crate::ShapeLinkError),
    Dependency(LayoutAbiDependencyError),
    Identity(IdentityReferenceError),
    Encoding(scoop_wire::cbor::EncodeError),
    Resource(WireError),
}

macro_rules! from_error {
    ($source:ty, $variant:ident) => {
        impl From<$source> for LayoutAbiSectionError {
            fn from(error: $source) -> Self {
                Self::$variant(error)
            }
        }
    };
}

from_error!(LayoutAbiExportConstituentsError, Exports);
from_error!(LayoutAbiSemanticClosureError, Semantic);
from_error!(crate::ExactLayoutTableError, Layout);
from_error!(crate::ExactDescriptorTableError, Descriptor);
from_error!(crate::ExactDispatchTableError, Dispatch);
from_error!(crate::ExactCallableAbiTableError, Callable);
from_error!(crate::ParamFreeShapeSupportTableError, ShapeSupportTable);
from_error!(crate::ShapeLinkError, Physical);
from_error!(LayoutAbiDependencyError, Dependency);
from_error!(IdentityReferenceError, Identity);
from_error!(scoop_wire::cbor::EncodeError, Encoding);
from_error!(WireError, Resource);

impl std::fmt::Display for LayoutAbiSectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectionProvider { selected, exported } => write!(
                formatter,
                "layout/ABI imports belong to Cone {selected}, but local exports belong to {exported}"
            ),
            Self::SelectionTarget { selected, exported } => write!(
                formatter,
                "layout/ABI imports target {}, but local exports target {}",
                selected.id().canonical_name(),
                exported.id().canonical_name()
            ),
            _ => write!(formatter, "invalid complete layout/ABI section: {self:?}"),
        }
    }
}

impl std::error::Error for LayoutAbiSectionError {}
