//! Structural use/def traversal shared by lowering and code emission.
//! Visitors preserve operand order and borrow variable-length argument lists.

use crate::*;

/// A parameter or body-local storage slot, independent of its GC representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ValueSlot {
    Param(u32),
    Local(LocalId),
    Temp(TempId),
}

impl ValueSlot {
    pub fn from_value(value: Value) -> Option<Self> {
        match value {
            Value::Param(index) => Some(Self::Param(index)),
            Value::Local(id) => Some(Self::Local(id)),
            Value::Temp(id) => Some(Self::Temp(id)),
            Value::CArgumentStorage(storage) => Some(Self::Local(storage.local())),
            Value::FloatConst(_)
            | Value::IntegerConst(_)
            | Value::MachineScalar(_)
            | Value::BoolConst(_)
            | Value::NullPointer(_)
            | Value::TypeDescriptor(_)
            | Value::RootScan(_)
            | Value::ContextKeyCell(_)
            | Value::Global(_)
            | Value::InitializationUnit(_) => None,
        }
    }
}

pub fn visit_instruction_uses(
    instruction: &Instruction,
    targets: &CallTargets,
    mut visit: impl FnMut(Value),
) {
    match instruction {
        Instruction::AtomicLoad { location, .. } => {
            visit(location.object);
        }
        Instruction::AtomicStore {
            location, value, ..
        }
        | Instruction::AtomicRmw {
            location, value, ..
        } => {
            visit(location.object);
            visit(*value);
        }
        Instruction::AtomicCmpXchg {
            location,
            expected,
            replacement,
            ..
        } => {
            visit(location.object);
            visit(*expected);
            visit(*replacement);
        }
        Instruction::BoxValue { payload, .. } => payload
            .source()
            .map(Value::Local)
            .into_iter()
            .for_each(&mut visit),
        Instruction::UnboxValue { object, .. } => {
            visit(*object);
        }
        Instruction::PublishReleaseReady { object } => {
            visit(*object);
        }
        Instruction::BinOp { lhs, rhs, .. }
        | Instruction::FloatBinary { lhs, rhs, .. }
        | Instruction::IntegerBinary { lhs, rhs, .. }
        | Instruction::SafeIntegerDivRem { lhs, rhs, .. }
        | Instruction::IntegerCompare { lhs, rhs, .. }
        | Instruction::IntegerCompareTo { lhs, rhs, .. } => {
            visit(*lhs);
            visit(*rhs);
        }
        Instruction::IntegerShift {
            value,
            normalized_count,
            ..
        } => {
            visit(*value);
            visit(*normalized_count);
        }
        Instruction::UnaryOp { operand, .. }
        | Instruction::FloatUnary { operand, .. }
        | Instruction::FloatConversion { operand, .. }
        | Instruction::IntegerUnary { operand, .. }
        | Instruction::IntegerConvert { operand, .. }
        | Instruction::ExtractValue {
            aggregate: operand, ..
        }
        | Instruction::HeapLoad {
            object: operand, ..
        }
        | Instruction::MachineHeapLoad {
            object: operand, ..
        }
        | Instruction::MachineAtomicLoad {
            object: operand, ..
        }
        | Instruction::ULongToPtr { value: operand, .. }
        | Instruction::PtrToULong { value: operand, .. }
        | Instruction::RawLoad {
            pointer: operand, ..
        }
        | Instruction::BeginCatch { raw: operand, .. }
        | Instruction::Throw { exception: operand }
        | Instruction::ArrayAllocDynamic { count: operand, .. }
        | Instruction::PushPinFrame {
            object: operand, ..
        }
        | Instruction::PopPinFrame { frame: operand }
        | Instruction::ArrayDataPointer {
            object: operand, ..
        }
        | Instruction::StringDataPointer {
            object: operand, ..
        }
        | Instruction::BorrowDataLength {
            object: operand, ..
        }
        | Instruction::ArrayLen { operand, .. }
        | Instruction::ArrayClone { operand, .. }
        | Instruction::EnumTag { operand, .. }
        | Instruction::EnumField { operand, .. }
        | Instruction::VariantTest { operand, .. }
        | Instruction::VariantPayloadProject { operand, .. }
        | Instruction::ForeignCallbackRegister {
            closure: operand, ..
        } => {
            visit(*operand);
        }
        Instruction::ForeignCallbackOperation(operation) => {
            visit(operation.callback());
        }
        Instruction::MaybeUninit { operation, .. } => operation
            .operand()
            .copied()
            .into_iter()
            .for_each(&mut visit),
        Instruction::MakeAggregate { elements, .. } | Instruction::ArrayAlloc { elements, .. } => {
            elements.iter().copied().for_each(&mut visit)
        }
        Instruction::StringCopy { source, .. } => source.operands().for_each(&mut visit),
        Instruction::ByteCopy {
            source,
            target,
            start,
            ..
        } => source
            .operands()
            .chain([*target, *start])
            .for_each(&mut visit),
        Instruction::StringByteLoad { string, index, .. } => {
            visit(*string);
            visit(*index);
        }
        Instruction::StringByteStore {
            string,
            index,
            value,
        } => {
            visit(*string);
            visit(*index);
            visit(*value);
        }
        Instruction::StringConcat { parts, .. } => parts
            .iter()
            .filter_map(StringConcatPart::value)
            .for_each(&mut visit),
        Instruction::ArrayAssembly { parts, .. } => parts
            .iter()
            .map(|part| match part {
                ArrayAssemblyPart::Element(value) | ArrayAssemblyPart::CopyArray(value) => *value,
            })
            .for_each(&mut visit),
        Instruction::Store { value, .. }
        | Instruction::TaskContextStore { value }
        | Instruction::GlobalStore { value, .. }
        | Instruction::NativeGlobalStore { value, .. } => {
            visit(*value);
        }
        Instruction::Call { site } => {
            call_uses(site.args(), site.destination(targets)).for_each(&mut visit)
        }
        Instruction::HeapStore { object, value, .. }
        | Instruction::MachineHeapStore { object, value, .. }
        | Instruction::MachineAtomicStore { object, value, .. } => {
            visit(*object);
            visit(*value);
        }
        Instruction::MachineAtomicCompareExchange {
            object,
            expected,
            replacement,
            ..
        } => {
            visit(*object);
            visit(*expected);
            visit(*replacement);
        }
        Instruction::RawStore { pointer, value, .. } => {
            visit(*pointer);
            visit(*value);
        }
        Instruction::PtrOffset {
            pointer,
            element_offset,
            ..
        } => {
            visit(*pointer);
            visit(*element_offset);
        }
        Instruction::LocalAddress { local, .. } => {
            visit(Value::Local(*local));
        }
        Instruction::ArrayGet { array, index, .. } => {
            visit(*array);
            visit(*index);
        }
        Instruction::ArraySet {
            array,
            index,
            value,
            ..
        } => {
            visit(*array);
            visit(*index);
            visit(*value);
        }
        Instruction::EnumWrap { fields, .. } => fields.iter().copied().for_each(&mut visit),
        Instruction::ReleaseFieldLoad { .. }
        | Instruction::MakeZstValue { .. }
        | Instruction::TaskContextLoad { .. }
        | Instruction::GlobalLoad { .. }
        | Instruction::GlobalAddress { .. }
        | Instruction::NativeGlobalLoad { .. }
        | Instruction::NativeGlobalAddress { .. }
        | Instruction::FunctionAddress { .. }
        | Instruction::ManagedPoll { .. }
        | Instruction::LandingPad { .. }
        | Instruction::CleanupPad { .. }
        | Instruction::EndCatch => {}
    }
}

/// Logical arguments followed by an indirect callee or dispatch table, if present.
pub fn call_uses(
    args: &[AbiCallArgument],
    destination: CallDestination,
) -> impl Iterator<Item = Value> + '_ {
    let callee = match destination {
        CallDestination::Dispatch { table, .. }
        | CallDestination::NativePointer { callee: table, .. } => Some(table),
        _ => None,
    };
    args.iter()
        .map(|argument| argument.logical_value())
        .chain(callee)
}

pub fn visit_instruction_defs(instruction: &Instruction, mut visit: impl FnMut(ValueSlot)) {
    let out = match instruction {
        Instruction::UnboxValue { result, .. } => {
            return match result {
                UnboxResult::ZeroSized { out, .. } => {
                    visit(ValueSlot::Temp(*out));
                }
                UnboxResult::NonZero(place) => {
                    visit(ValueSlot::Local(place.local()));
                }
            };
        }
        Instruction::BoxValue { out, .. }
        | Instruction::BinOp { out, .. }
        | Instruction::UnaryOp { out, .. }
        | Instruction::FloatUnary { out, .. }
        | Instruction::FloatBinary { out, .. }
        | Instruction::FloatConversion { out, .. }
        | Instruction::IntegerUnary { out, .. }
        | Instruction::IntegerBinary { out, .. }
        | Instruction::SafeIntegerDivRem { out, .. }
        | Instruction::IntegerCompare { out, .. }
        | Instruction::IntegerCompareTo { out, .. }
        | Instruction::IntegerShift { out, .. }
        | Instruction::IntegerConvert { out, .. }
        | Instruction::MakeAggregate { out, .. }
        | Instruction::MakeZstValue { out, .. }
        | Instruction::ExtractValue { out, .. }
        | Instruction::ReleaseFieldLoad { out, .. }
        | Instruction::HeapLoad { out, .. }
        | Instruction::MachineHeapLoad { out, .. }
        | Instruction::MachineAtomicLoad { out, .. }
        | Instruction::AtomicLoad { out, .. }
        | Instruction::AtomicRmw { out, .. }
        | Instruction::AtomicCmpXchg { out, .. }
        | Instruction::MachineAtomicCompareExchange { out, .. }
        | Instruction::GlobalLoad { out, .. }
        | Instruction::GlobalAddress { out, .. }
        | Instruction::TaskContextLoad { out }
        | Instruction::NativeGlobalLoad { out, .. }
        | Instruction::NativeGlobalAddress { out, .. }
        | Instruction::FunctionAddress { out, .. }
        | Instruction::ULongToPtr { out, .. }
        | Instruction::PtrToULong { out, .. }
        | Instruction::RawLoad { out, .. }
        | Instruction::PtrOffset { out, .. }
        | Instruction::LocalAddress { out, .. }
        | Instruction::BeginCatch { out, .. }
        | Instruction::ArrayAllocDynamic { out, .. }
        | Instruction::ArrayAlloc { out, .. }
        | Instruction::ArrayAssembly { out, .. }
        | Instruction::StringConcat { out, .. }
        | Instruction::StringCopy { out, .. }
        | Instruction::StringByteLoad { out, .. }
        | Instruction::PushPinFrame { out, .. }
        | Instruction::ArrayDataPointer { out, .. }
        | Instruction::StringDataPointer { out, .. }
        | Instruction::BorrowDataLength { out, .. }
        | Instruction::ArrayLen { out, .. }
        | Instruction::ArrayGet { out, .. }
        | Instruction::ArrayClone { out, .. }
        | Instruction::EnumWrap { out, .. }
        | Instruction::EnumTag { out, .. }
        | Instruction::EnumField { out, .. }
        | Instruction::VariantTest { out, .. }
        | Instruction::VariantPayloadProject { out, .. }
        | Instruction::ForeignCallbackRegister { out, .. } => Some(*out),
        Instruction::Call { site } => return call_def(site.result()).into_iter().for_each(visit),
        Instruction::ForeignCallbackOperation(operation) => operation.out(),
        Instruction::Store { local, .. } => {
            return visit(ValueSlot::Local(*local));
        }
        Instruction::MaybeUninit { out, .. } => {
            return visit(ValueSlot::Local(*out));
        }
        Instruction::LandingPad { record, raw } | Instruction::CleanupPad { record, raw } => {
            visit(ValueSlot::Temp(*record));
            visit(ValueSlot::Temp(*raw));
            return;
        }
        Instruction::TaskContextStore { .. }
        | Instruction::GlobalStore { .. }
        | Instruction::PublishReleaseReady { .. }
        | Instruction::NativeGlobalStore { .. }
        | Instruction::HeapStore { .. }
        | Instruction::MachineHeapStore { .. }
        | Instruction::MachineAtomicStore { .. }
        | Instruction::AtomicStore { .. }
        | Instruction::RawStore { .. }
        | Instruction::ArraySet { .. }
        | Instruction::ManagedPoll { .. }
        | Instruction::PopPinFrame { .. }
        | Instruction::EndCatch
        | Instruction::Throw { .. }
        | Instruction::ByteCopy { .. }
        | Instruction::StringByteStore { .. } => None,
    };
    out.into_iter().map(ValueSlot::Temp).for_each(visit)
}

pub fn call_def(result: TypedCallResult) -> Option<ValueSlot> {
    match result {
        TypedCallResult::Void => None,
        TypedCallResult::ElidedZst(out) | TypedCallResult::Direct(out) => {
            Some(ValueSlot::Temp(out))
        }
        TypedCallResult::IndirectResult(storage) => Some(ValueSlot::Local(storage)),
    }
}

pub fn visit_terminator_uses(
    terminator: &Terminator,
    targets: &CallTargets,
    mut use_value: impl FnMut(Value),
) {
    match terminator {
        Terminator::Invoke { site } => {
            for value in call_uses(site.args(), site.destination(targets)) {
                use_value(value);
            }
        }
        Terminator::CondBr { cond, .. } => use_value(*cond),
        Terminator::Return { value: Some(value) } => use_value(*value),
        Terminator::Resume { exception } => use_value(*exception),
        Terminator::Br(_) | Terminator::Return { value: None } | Terminator::Unreachable => {}
    }
}

pub fn terminator_def(terminator: &Terminator) -> Option<ValueSlot> {
    match terminator {
        Terminator::Invoke { site } => call_def(site.result()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indirect_result_storage_is_a_call_definition() {
        let storage = LocalId::from_raw(3.into());
        assert_eq!(
            call_def(TypedCallResult::IndirectResult(storage)),
            Some(ValueSlot::Local(storage))
        );
    }
}
