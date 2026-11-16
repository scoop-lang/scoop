//! A function and its identity origin are allocated together in each snapshot.

use std::cell::OnceCell;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

use la_arena::{Arena, Idx};
use scoop_hir as hir;

use crate::{SourceFunctionDeclaration, snapshot_arena::SnapshotArena};

#[derive(Clone)]
pub(crate) enum FunctionOrigin {
    Source(SourceFunctionDeclaration),
    Getter(hir::PropertyId),
    Setter(hir::PropertyId),
    Lexical {
        root: hir::LexicalDefinitionRoot,
        path: scoop_identity::StructuralDefinitionPath,
        role: scoop_identity::LexicalCallableRole,
    },
    Initialization {
        unit: hir::InitializationUnitId,
        role: scoop_identity::InitializationCallableRole,
    },
    TestEntry,
    DerivedEquality,
}

#[derive(Clone)]
struct Declaration {
    function: hir::Function,
    origin: FunctionOrigin,
}

#[derive(Clone, Default)]
pub(crate) struct FunctionDeclarations {
    entries: SnapshotArena<Declaration>,
    view: Rc<OnceCell<Arena<hir::Function>>>,
}

impl FunctionDeclarations {
    pub(crate) fn alloc(
        &mut self,
        function: hir::Function,
        origin: FunctionOrigin,
    ) -> hir::FunctionId {
        self.view = Rc::default();
        let id = self.entries.alloc(Declaration { function, origin });
        hir::FunctionId::from_raw(id.into_raw())
    }

    pub(crate) fn origin(&self, id: hir::FunctionId) -> &FunctionOrigin {
        &self.entries[Idx::from_raw(id.into_raw())].origin
    }

    pub(crate) fn source(&self, id: hir::FunctionId) -> Option<&SourceFunctionDeclaration> {
        match self.origin(id) {
            FunctionOrigin::Source(source) => Some(source),
            _ => None,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (hir::FunctionId, &hir::Function)> + ExactSizeIterator
    {
        self.entries.iter().map(|(id, declaration)| {
            (
                hir::FunctionId::from_raw(id.into_raw()),
                &declaration.function,
            )
        })
    }

    pub(crate) fn as_arena(&self) -> &Arena<hir::Function> {
        self.view
            .get_or_init(|| self.iter().map(|(_, function)| function.clone()).collect())
    }

    pub(crate) fn into_arena(self) -> Arena<hir::Function> {
        self.entries
            .into_arena()
            .into_iter()
            .map(|(_, declaration)| declaration.function)
            .collect()
    }
}

impl Index<hir::FunctionId> for FunctionDeclarations {
    type Output = hir::Function;
    fn index(&self, id: hir::FunctionId) -> &Self::Output {
        &self.entries[Idx::from_raw(id.into_raw())].function
    }
}

impl IndexMut<hir::FunctionId> for FunctionDeclarations {
    fn index_mut(&mut self, id: hir::FunctionId) -> &mut Self::Output {
        self.view = Rc::default();
        &mut self.entries[Idx::from_raw(id.into_raw())].function
    }
}
