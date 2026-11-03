//! Closed task-context operations shared by MIR construction and CFG lowering.

pub use scoop_identity::{ContextKey, ContextStorageRole, ContextStorageType};

pub fn context_type_representation(storage: ContextStorageType) -> crate::MirTypeRepresentationV1 {
    use crate::MirTypeRepresentationV1 as Repr;
    let fields = context_fields(storage)
        .into_iter()
        .map(|(field, ty)| crate::MirRepresentationFieldV1 {
            field: field.id(),
            value: ty.exact_record().id(),
        })
        .collect();
    match storage.role {
        ContextStorageRole::Task | ContextStorageRole::Node => Repr::Class {
            kind: crate::MirClassKindV1::Final,
            declared_fields: fields,
            release_policy: Default::default(),
        },
        // Runtime bindings erase a reference to its object, not an interface view.
        ContextStorageRole::Binding => Repr::Intrinsic(crate::MirParamFreeIntrinsicV1::Any),
        ContextStorageRole::Mark | ContextStorageRole::SwitchGuard => Repr::Struct {
            fields,
            c_layout: crate::MirTypeCLayoutPolicyV1::Ordinary,
            interior_mutable: false,
        },
    }
}

pub fn context_fields(
    storage: ContextStorageType,
) -> Vec<(
    scoop_identity::CborIdentityRecord<
        scoop_identity::PersistentFieldId,
        scoop_identity::FieldIdentityKey,
    >,
    ContextStorageType,
)> {
    use ContextStorageRole as Role;
    let roles: &[Role] = match storage.role {
        Role::Task => &[Role::Node, Role::Binding],
        Role::Node => &[Role::Binding, Role::Binding, Role::Binding, Role::Binding],
        Role::Binding => &[],
        Role::Mark => &[Role::Task, Role::Node],
        Role::SwitchGuard => &[Role::Task],
    };
    let owner = storage.nominal_record();
    roles
        .iter()
        .enumerate()
        .map(|(index, role)| {
            let key = scoop_identity::FieldIdentityKey::context_slot(owner.key(), index as u32)
                .expect("context storage has its fixed field roles");
            (
                scoop_identity::CborIdentityRecord::from_key(key)
                    .expect("context field identity is canonical"),
                ContextStorageType::new(storage.core, *role),
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
pub enum ContextOperation<E> {
    TryGet {
        key: ContextKey,
    },
    Push {
        key: ContextKey,
        value: Box<E>,
    },
    Restore {
        mark: Box<E>,
    },
    Current,
    Snapshot,
    CurrentExecution,
    Fork {
        root: Box<E>,
        /// An independent invocation has no execution until a coroutine owns it.
        execution: Option<Box<E>>,
    },
    Enter {
        task: Box<E>,
    },
    Leave {
        guard: Box<E>,
    },
    IsPresent {
        binding: Box<E>,
    },
    UnwrapBinding {
        binding: Box<E>,
    },
    EnsureRoot,
}

impl<E> ContextOperation<E> {
    pub fn operands(&self) -> impl Iterator<Item = &E> {
        let operands = match self {
            Self::TryGet { .. }
            | Self::Current
            | Self::Snapshot
            | Self::CurrentExecution
            | Self::EnsureRoot => [None, None],
            Self::Push { value, .. } => [Some(value.as_ref()), None],
            Self::Restore { mark } => [Some(mark.as_ref()), None],
            Self::Fork { root, execution } => [Some(root.as_ref()), execution.as_deref()],
            Self::Enter { task } => [Some(task.as_ref()), None],
            Self::Leave { guard } => [Some(guard.as_ref()), None],
            Self::IsPresent { binding } | Self::UnwrapBinding { binding } => {
                [Some(binding.as_ref()), None]
            }
        };
        operands.into_iter().flatten()
    }

    pub fn operands_mut(&mut self) -> impl Iterator<Item = &mut E> {
        let operands = match self {
            Self::TryGet { .. }
            | Self::Current
            | Self::Snapshot
            | Self::CurrentExecution
            | Self::EnsureRoot => [None, None],
            Self::Push { value, .. } => [Some(value.as_mut()), None],
            Self::Restore { mark } => [Some(mark.as_mut()), None],
            Self::Fork { root, execution } => [Some(root.as_mut()), execution.as_deref_mut()],
            Self::Enter { task } => [Some(task.as_mut()), None],
            Self::Leave { guard } => [Some(guard.as_mut()), None],
            Self::IsPresent { binding } | Self::UnwrapBinding { binding } => {
                [Some(binding.as_mut()), None]
            }
        };
        operands.into_iter().flatten()
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::TryGet { .. } => "ContextTryGet",
            Self::Push { .. } => "ContextPush",
            Self::Restore { .. } => "ContextRestore",
            Self::Current => "ContextCurrent",
            Self::Snapshot => "ContextSnapshot",
            Self::CurrentExecution => "ContextCurrentExecution",
            Self::Fork { .. } => "ContextFork",
            Self::Enter { .. } => "ContextEnter",
            Self::Leave { .. } => "ContextLeave",
            Self::IsPresent { .. } => "ContextIsPresent",
            Self::UnwrapBinding { .. } => "ContextUnwrapBinding",
            Self::EnsureRoot => "ContextEnsureRoot",
        }
    }

    pub fn map<F>(&self, mut map: impl FnMut(&E) -> F) -> ContextOperation<F> {
        match self {
            Self::TryGet { key } => ContextOperation::TryGet { key: *key },
            Self::Push { key, value } => ContextOperation::Push {
                key: *key,
                value: Box::new(map(value)),
            },
            Self::Restore { mark } => ContextOperation::Restore {
                mark: Box::new(map(mark)),
            },
            Self::Current => ContextOperation::Current,
            Self::Snapshot => ContextOperation::Snapshot,
            Self::CurrentExecution => ContextOperation::CurrentExecution,
            Self::Fork { root, execution } => ContextOperation::Fork {
                root: Box::new(map(root)),
                execution: execution.as_ref().map(|value| Box::new(map(value))),
            },
            Self::Enter { task } => ContextOperation::Enter {
                task: Box::new(map(task)),
            },
            Self::Leave { guard } => ContextOperation::Leave {
                guard: Box::new(map(guard)),
            },
            Self::IsPresent { binding } => ContextOperation::IsPresent {
                binding: Box::new(map(binding)),
            },
            Self::UnwrapBinding { binding } => ContextOperation::UnwrapBinding {
                binding: Box::new(map(binding)),
            },
            Self::EnsureRoot => ContextOperation::EnsureRoot,
        }
    }

    pub const fn may_gc(&self) -> bool {
        matches!(
            self,
            Self::Push { .. } | Self::Fork { .. } | Self::EnsureRoot
        )
    }
}
