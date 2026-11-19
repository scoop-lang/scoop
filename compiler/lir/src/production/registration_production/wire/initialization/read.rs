use super::*;
use crate::link_data::link_error;
use crate::*;
use scoop_identity::{ConeIdentity, PersistentIdResolver, ValidatedIdentityGraph};

impl DecodedStrongInitializationUnitRegistrationPlanV1 {
    pub(in crate::production::registration_production) fn read_definition(
        &self,
        provider: ConeIdentity,
        identities: &mut ValidatedIdentityGraph,
    ) -> Result<StrongInitializationUnitDefinitionRefV2, LinkDataError> {
        Ok(StrongInitializationUnitDefinitionRefV2 {
            provider,
            unit: identities.resolve(self.semantic.unit).map_err(link_error)?,
            cell: StrongInitializationArtifactRefV2 {
                symbol: self.cell_symbol.resolve(identities).map_err(link_error)?,
                plan: identities
                    .resolve(self.cell_definition_plan)
                    .map_err(link_error)?,
                primary: identities
                    .resolve(self.cell_primary_atom)
                    .map_err(link_error)?,
            },
            registration: StrongInitializationArtifactRefV2 {
                symbol: self
                    .registration_symbol
                    .resolve(identities)
                    .map_err(link_error)?,
                plan: identities
                    .resolve(self.registration_definition_plan)
                    .map_err(link_error)?,
                primary: identities
                    .resolve(self.registration_primary_atom)
                    .map_err(link_error)?,
            },
        })
    }

    pub(in crate::production::registration_production) fn read(
        self,
        definition: StrongInitializationUnitDefinitionRefV2,
        identities: &mut ValidatedIdentityGraph,
        definitions: &StrongInitializationDefinitionCatalogV2,
    ) -> Result<CompileInitializationUnit, LinkDataError> {
        let data = self.semantic;
        let unit = identities.resolve(data.unit).map_err(link_error)?;
        let dependencies = definitions
            .resolve(unit, &data.dependencies)
            .map_err(link_error)?
            .into_references();
        let semantic = data.resolve(identities, unit, dependencies)?;
        Ok(CompileInitializationUnit {
            semantic,
            definition,
        })
    }
}

impl DecodedStrongInitializationUnitSemanticProjectionV1 {
    pub(in crate::production::registration_production) fn read_link(
        self,
        identities: &mut ValidatedIdentityGraph,
    ) -> Result<StrongInitializationUnitSemanticPlanV1, LinkDataError> {
        let unit = identities.resolve(self.unit).map_err(link_error)?;
        let dependencies = self
            .dependencies
            .iter()
            .map(|id| identities.resolve(*id).map_err(link_error))
            .collect::<Result<_, _>>()?;
        self.resolve(identities, unit, dependencies)
    }

    fn resolve<D>(
        self,
        identities: &mut ValidatedIdentityGraph,
        unit: PersistentInitializationUnitId,
        dependencies: Vec<D>,
    ) -> Result<StrongInitializationUnitSemanticPlan<D>, LinkDataError> {
        let schedule = match self.semantic_schedule {
            DecodedStrongInitializationSchedulePlanV1::LazyAccess => {
                StrongInitializationSchedulePlanV1::LazyAccess
            }
            DecodedStrongInitializationSchedulePlanV1::EagerStartup(gateway) => {
                StrongInitializationSchedulePlanV1::EagerStartup {
                    gateway: identities.resolve(gateway).map_err(link_error)?,
                }
            }
        };
        Ok(StrongInitializationUnitSemanticPlan::from_artifact(
            unit,
            self.diagnostic_path,
            schedule,
            identities.resolve(self.storage_id).map_err(link_error)?,
            identities
                .resolve(self.failure_root_id)
                .map_err(link_error)?,
            identities
                .resolve(self.initializer_id)
                .map_err(link_error)?,
            identities.resolve(self.ensure_id).map_err(link_error)?,
            dependencies,
        ))
    }
}
