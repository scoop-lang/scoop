use scoop_lir::*;

const POINTER: ScalarLayout = ScalarLayout::new(8, 8);
const RAW_POINTER: PointerRepresentation = PointerRepresentation::new(
    POINTER,
    PointerNullEncoding::AllZeroBits,
    InternalPointerCarrier::BitPreservingU64,
);
const SCALARS: [ScalarLayout; 7] = [
    ScalarLayout::new(1, 1),
    ScalarLayout::new(1, 1),
    ScalarLayout::new(2, 2),
    ScalarLayout::new(4, 4),
    ScalarLayout::new(8, 8),
    ScalarLayout::new(4, 4),
    ScalarLayout::new(8, 8),
];

pub(crate) struct LayoutFacts {
    pub llvm_data_layout: &'static str,
    pub object_target: ObjectTargetProfileV1,
    pub architecture: TargetArchitecture,
}

pub(crate) const fn describe(
    triple: &'static str,
    facts: LayoutFacts,
    procedure_abi: ProcedureAbi,
    scoop_abi_classifier: ScoopAbiClassifier,
    exception_handling: TargetExceptionHandling,
) -> TargetLayoutDescription {
    TargetLayoutDescription {
        canonical_triple: triple,
        llvm_data_layout: facts.llvm_data_layout,
        object_target: facts.object_target,
        architecture: facts.architecture,
        procedure_abi,
        scoop_abi_classifier,
        exception_handling,
        byte_order: ByteOrder::Little,
        scalar_layouts: SCALARS,
        managed_pointer: POINTER,
        data_pointer: RAW_POINTER,
        code_pointer: RAW_POINTER,
        metadata_pointer: POINTER,
        stack_alignment: 16,
        maximum_managed_alignment: 16,
        maximum_managed_object_size: i64::MAX as u64,
    }
}
