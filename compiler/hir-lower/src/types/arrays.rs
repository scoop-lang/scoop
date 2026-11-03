use super::*;

impl Lowerer {
    pub(crate) fn array_class(&self, kind: ArrayKind) -> hir::ClassId {
        let intrinsic = match kind {
            ArrayKind::Immutable => hir::IntrinsicTypeKind::Array,
            ArrayKind::Mutable => hir::IntrinsicTypeKind::MutableArray,
        };
        let &(owner, _) = self
            .intrinsic_type_owners
            .get(&intrinsic)
            .expect("the intrinsic core contract is validated before type resolution");
        let IntrinsicTypeOwner::Class(class) = owner else {
            unreachable!("the intrinsic registry fixes array declarations as classes")
        };
        class
    }

    pub(crate) fn array_class_kind(&self, class: hir::ClassId) -> Option<ArrayKind> {
        match self.classes[class].representation {
            hir::ClassRepresentation::Intrinsic(hir::IntrinsicTypeKind::Array) => {
                Some(ArrayKind::Immutable)
            }
            hir::ClassRepresentation::Intrinsic(hir::IntrinsicTypeKind::MutableArray) => {
                Some(ArrayKind::Mutable)
            }
            hir::ClassRepresentation::Declared
            | hir::ClassRepresentation::Intrinsic(
                hir::IntrinsicTypeKind::Unit
                | hir::IntrinsicTypeKind::Atomic(_)
                | hir::IntrinsicTypeKind::MaybeUninit
                | hir::IntrinsicTypeKind::Integer(_)
                | hir::IntrinsicTypeKind::Float(_)
                | hir::IntrinsicTypeKind::Char
                | hir::IntrinsicTypeKind::Boolean
                | hir::IntrinsicTypeKind::String
                | hir::IntrinsicTypeKind::Any
                | hir::IntrinsicTypeKind::Nothing
                | hir::IntrinsicTypeKind::Throwable
                | hir::IntrinsicTypeKind::Ptr
                | hir::IntrinsicTypeKind::FunPtr
                | hir::IntrinsicTypeKind::GcLeafFunPtr,
            ) => None,
        }
    }

    pub(crate) fn array_type(&mut self, kind: ArrayKind, element: TypeId) -> TypeId {
        if let crate::CoreLoweringAuthority::Imported(protocols) = &self.core {
            let fundamental = protocols.fundamental_types();
            let owner = match kind {
                ArrayKind::Immutable => fundamental.array(),
                ArrayKind::Mutable => fundamental.mutable_array(),
            };
            return self
                .imported_nominal_application(
                    hir::SourceNominalId::GenericTemplate(owner.persistent()),
                    vec![element],
                )
                .expect("the checked array protocol has a complete dependency declaration");
        }
        let template = self.array_class(kind);
        self.class_application(template, vec![element])
    }

    /// Return the exact array family application carried by a class type.
    /// Ordinary classes and fixed intrinsic classes return `None`.
    pub(crate) fn array_type_info(&self, ty: TypeId) -> Option<ArrayType> {
        let application = self.nominal_application(ty)?;
        let kind = match self.nominal_intrinsic_kind(application.template)? {
            hir::IntrinsicTypeKind::Array => ArrayKind::Immutable,
            hir::IntrinsicTypeKind::MutableArray => ArrayKind::Mutable,
            _ => return None,
        };
        let [element] = application.arguments.as_slice() else {
            unreachable!("a checked array application has one element type")
        };
        Some(ArrayType {
            kind,
            element: *element,
        })
    }

    pub(crate) fn array_element_ty(&self, ty: TypeId) -> Option<TypeId> {
        self.array_type_info(ty).map(|array| array.element)
    }

    /// Infer the literal's concrete array through its declared supertypes.
    /// The ordinary solver keeps interface arguments invariant and leaves
    /// unrelated or element-free expectations without an inferred element.
    pub(crate) fn expected_array_literal(
        &mut self,
        expected: TypeId,
        span: ast::Span,
    ) -> Option<TypeId> {
        if self.array_type_info(expected).is_some() {
            return Some(expected);
        }
        self.nominal_application(expected)?;
        use crate::call_resolution::candidates::NominalConstructorSource;
        use crate::call_resolution::constraints::{
            Constraint, ConstraintOrigin, InferenceSession, TypeTerm,
        };
        for kind in [ArrayKind::Immutable, ArrayKind::Mutable] {
            let source = match &self.core {
                crate::CoreLoweringAuthority::Defined => {
                    NominalConstructorSource::IntrinsicClass(self.array_class(kind))
                }
                crate::CoreLoweringAuthority::Imported(protocols) => {
                    let fundamental = protocols.fundamental_types();
                    let owner = match kind {
                        ArrayKind::Immutable => fundamental.array(),
                        ArrayKind::Mutable => fundamental.mutable_array(),
                    };
                    NominalConstructorSource::ImportedArray(hir::SourceNominalId::GenericTemplate(
                        owner.persistent(),
                    ))
                }
            };
            let view = self.nominal_constructor_view(source, span);
            let mut session = InferenceSession::new();
            let environment = session.add_environment(&view.signature.owner_parameters, &[]);
            session.push(
                Constraint::Subtype(
                    TypeTerm::Type(view.signature.return_type),
                    TypeTerm::Rigid(expected),
                ),
                ConstraintOrigin::ExpectedResult,
            );
            if let Ok(solution) = self.solve_constraints(&session) {
                let element = solution.type_for(session.owner_variables(environment)[0]);
                return Some(self.array_type(kind, element));
            }
        }
        None
    }
}
