//! Connect the physical members of each initializer before building image tables.

use super::*;
use scoop_identity::{PersistentInitializationUnitId, StrongDefinitionEntity as Entity};
use scoop_lir::StrongInitializationUnitSemanticPlanV1;

fn entities(unit: &StrongInitializationUnitSemanticPlanV1) -> impl Iterator<Item = Entity> {
    [
        Entity::initialization_unit(unit.unit()),
        Entity::static_storage(unit.storage()),
        Entity::static_storage(unit.failure_root()),
        Entity::callable_body(unit.initializer()),
        Entity::callable_body(unit.ensure()),
    ]
    .into_iter()
    .chain(unit.schedule().gateway().map(Entity::callable_body))
}

#[derive(Default)]
pub(super) struct InitializationMembers(BTreeMap<Entity, BTreeSet<Member>>);

impl InitializationMembers {
    pub(super) fn new(closure: &ProgramLinkClosure) -> Self {
        let mut groups = Self::default();
        for (artifact, _) in closure.artifacts() {
            for unit in &artifact.production().initializations {
                for entity in entities(unit) {
                    groups.0.entry(entity).or_default();
                }
            }
        }
        groups
    }

    pub(super) fn record(&mut self, entity: Entity, member: Member) -> bool {
        if let Some(members) = self.0.get_mut(&entity) {
            members.insert(member);
            return true;
        }
        false
    }

    fn anchor(&self, unit: PersistentInitializationUnitId) -> Result<Member, LinkError> {
        self.0
            .get(&Entity::initialization_unit(unit))
            .and_then(|members| members.first())
            .copied()
            .ok_or_else(|| {
                error(
                    scoop_source::DiagnosticCode::E2409,
                    format!("initialization unit {unit:?} has no selected definition"),
                )
            })
    }

    pub(super) fn connect(
        &self,
        closure: &ProgramLinkClosure,
        edges: &mut BTreeMap<Member, Vec<Member>>,
        pending: &mut Vec<Member>,
    ) -> Result<(), LinkError> {
        for (artifact, _) in closure.artifacts() {
            for unit in &artifact.production().initializations {
                let anchor = self.anchor(unit.unit())?;
                if unit.schedule().gateway().is_some() {
                    pending.push(anchor);
                }
                for entity in entities(unit) {
                    for &member in &self.0[&entity] {
                        if member != anchor {
                            edges.entry(member).or_default().push(anchor);
                            edges.entry(anchor).or_default().push(member);
                        }
                    }
                }
                for &dependency in unit.dependencies() {
                    edges
                        .entry(anchor)
                        .or_default()
                        .push(self.anchor(dependency)?);
                }
            }
        }
        Ok(())
    }
}
