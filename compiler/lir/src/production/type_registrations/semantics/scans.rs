//! Reuse borrowed inline scans while projecting each type descriptor.

use scoop_identity::PersistentScanId;

use super::StrongTypeDescriptorSemanticPlanBuildError;
use crate::{ArrayType, Layout, LayoutKind, RefScan, TypeDescriptor, TypeDescriptorInlineScanV1};

pub(super) struct InlineScans<'a> {
    records: Vec<(PersistentScanId, &'a RefScan)>,
}

impl<'a> InlineScans<'a> {
    pub(super) fn new(
        layouts: &'a la_arena::Arena<Layout>,
        arrays: &'a la_arena::Arena<ArrayType>,
    ) -> Self {
        let mut records = layouts
            .iter()
            .map(|(_, layout)| {
                let payload = match &layout.kind {
                    LayoutKind::Plain { scan } | LayoutKind::Enum { scan } => scan,
                    LayoutKind::Intrinsic(_) => &RefScan::None,
                };
                (layout.identity.scan_record().id(), payload)
            })
            .chain(arrays.iter().map(|(_, array)| {
                (
                    array.identity.scan_record().id(),
                    array.layout.instance().inline_scan(),
                )
            }))
            .collect::<Vec<_>>();
        records.sort_unstable_by_key(|(id, _)| *id);
        Self { records }
    }

    pub(super) fn validate(
        &self,
        descriptor: &TypeDescriptor,
    ) -> Result<(), StrongTypeDescriptorSemanticPlanBuildError> {
        let exact_type = descriptor.identity.exact_type();
        let contains_reference = descriptor.instance_shape.inline_scan().contains_reference();
        let TypeDescriptorInlineScanV1::Defined(scan) = descriptor.inline_scan else {
            return if contains_reference {
                Err(StrongTypeDescriptorSemanticPlanBuildError::MissingInlineScan(exact_type))
            } else {
                Ok(())
            };
        };
        if !contains_reference {
            return Err(
                StrongTypeDescriptorSemanticPlanBuildError::UnexpectedInlineScan {
                    exact_type,
                    scan,
                },
            );
        }
        let start = self.records.partition_point(|(id, _)| *id < scan);
        let mut payloads = self.records[start..]
            .iter()
            .take_while(|(id, _)| *id == scan)
            .map(|(_, payload)| *payload);
        let Some(payload) = payloads.next() else {
            return Err(
                StrongTypeDescriptorSemanticPlanBuildError::UnknownInlineScan { exact_type, scan },
            );
        };
        if payloads.next().is_some() {
            return Err(
                StrongTypeDescriptorSemanticPlanBuildError::DuplicateInlineScanDefinition {
                    exact_type,
                    scan,
                },
            );
        }
        if payload != descriptor.instance_shape.inline_scan() {
            return Err(
                StrongTypeDescriptorSemanticPlanBuildError::InlineScanPayloadMismatch {
                    exact_type,
                    scan,
                },
            );
        }
        Ok(())
    }
}
