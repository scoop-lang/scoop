//! Follow saved physical references without making image tables liveness roots.

use super::*;
use scoop_identity::StrongDefinitionEntity;
use scoop_slib::ProgramLinkMember;

type Member = (ConeIdentity, SlibMemberId);

impl SelectedMembers {
    pub(super) fn retain_reachable<'a>(
        &mut self,
        closure: &ProgramLinkClosure,
        native_references: impl Iterator<Item = &'a str>,
    ) -> Result<(), LinkError> {
        let mut owners = BTreeMap::<&[u8], Member>::new();
        let mut references = BTreeMap::<Member, Vec<&[u8]>>::new();
        let mut registered_definitions = Vec::new();
        let mut registrations = BTreeMap::<StrongDefinitionEntity, Member>::new();
        let mut pending = Vec::new();
        for (artifact, symbols) in closure.artifacts() {
            let cone = artifact.identity();
            let plans = &artifact.production().definitions;
            for member in symbols.members() {
                if !self.contains(cone, member.member()) {
                    continue;
                }
                let root = match member {
                    ProgramLinkMember::Scoop { definitions, .. } => {
                        for id in definitions {
                            let plan = &plans[id];
                            match plan.role {
                                StrongDefinitionRole::TypeDescriptor
                                | StrongDefinitionRole::ImmortalObject => {
                                    registered_definitions.push((
                                        (cone, member.member()),
                                        plan.entity,
                                        plan.role,
                                    ));
                                }
                                StrongDefinitionRole::TypeRegistration
                                | StrongDefinitionRole::ImmortalRegistration => {
                                    registrations.insert(plan.entity, (cone, member.member()));
                                }
                                _ => {}
                            }
                        }
                        definitions.iter().any(|id| is_root(plans[id].role))
                    }
                    ProgramLinkMember::Generated { .. } => true,
                };
                if root {
                    // Keep observable storage, initialization and native bridges.
                    pending.push((cone, member.member()));
                }
            }
            for definition in symbols.defined_symbols().owners() {
                let member = (cone, definition.member());
                if !self.members.contains(&member) {
                    continue;
                }
                if let Some(previous) = owners.insert(definition.symbol(), member) {
                    return Err(error(format!(
                        "multiple selected definitions for {}: {previous:?}, {member:?}",
                        String::from_utf8_lossy(definition.symbol())
                    )));
                }
                if symbols
                    .link_support()
                    .runtime_data_aliases()
                    .iter()
                    .any(|alias| {
                        definition.owner() == LinkDefinitionOwnerV1::StrongDefinition(alias.owner())
                    })
                {
                    // The startup glue creates this alias after member selection.
                    pending.push(member);
                }
            }
            for requirement in symbols.requirements() {
                let member = (cone, requirement.member());
                if self.members.contains(&member) {
                    references
                        .entry(member)
                        .or_default()
                        .push(requirement.symbol());
                }
            }
        }
        let mut companions = BTreeMap::<Member, Vec<Member>>::new();
        for (member, entity, role) in registered_definitions {
            let registration = registrations.get(&entity).ok_or_else(|| {
                error(format!("{role:?} {entity:?} has no selected registration"))
            })?;
            if member != *registration {
                companions.entry(member).or_default().push(*registration);
            }
        }
        pending
            .extend(native_references.filter_map(|symbol| owners.get(symbol.as_bytes()).copied()));
        let mut reached = BTreeSet::new();
        while let Some(member) = pending.pop() {
            if !reached.insert(member) {
                continue;
            }
            if let Some(registrations) = companions.get(&member) {
                pending.extend(registrations);
            }
            if let Some(symbols) = references.get(&member) {
                pending.extend(
                    symbols
                        .iter()
                        .filter_map(|symbol| owners.get(symbol).copied()),
                );
            }
        }
        self.members.retain(|member| reached.contains(member));
        for (artifact, symbols) in closure.artifacts() {
            let cone = artifact.identity();
            let definitions = self.definitions.get_mut(&cone).expect("selected Cone");
            definitions.clear();
            for member in symbols.members() {
                if self.members.contains(&(cone, member.member()))
                    && let ProgramLinkMember::Scoop {
                        definitions: retained,
                        ..
                    } = member
                {
                    // Only selected registrations enter the final image tables.
                    definitions.extend(retained);
                }
            }
        }
        Ok(())
    }
}

fn is_root(role: StrongDefinitionRole) -> bool {
    use StrongDefinitionRole::*;
    match role {
        StaticStorage
        | RootRegistration
        | InitializationCell
        | InitializationRegistration
        | GeneratedBridge
        | RootEntryDescriptor
        | NativeAppEntryDescriptor => true,
        CallableBody | CallableRegistration | TypeDescriptor | TypeRegistration
        | ImmortalObject | ImmortalRegistration | Layout | ScanProgram | DispatchTable
        | DispatchSlot | ImageDescriptor => false,
    }
}
