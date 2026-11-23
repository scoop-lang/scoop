use scoop_identity::PersistentInitializationUnitId;

use super::VerifiedStrongSafepointFingerprintSetV1;
use crate::link_object::{
    ImmortalObjectFingerprintSetV1, InitializationFingerprintSetV1, StaticStorageFingerprintSetV1,
    TypeFingerprintSetV1, VerifiedStrongCallableFingerprintSetV1,
};

/// Registration fingerprints derived from the emitted objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistrationObjectSetV1<
    D = scoop_lir::StrongTypeDescriptorRefV1,
    C = scoop_lir::StrongTypeDispatchCallableRefV1,
    I = PersistentInitializationUnitId,
> {
    safepoints: VerifiedStrongSafepointFingerprintSetV1,
    callables: VerifiedStrongCallableFingerprintSetV1,
    types: TypeFingerprintSetV1<D, C>,
    immortal_objects: ImmortalObjectFingerprintSetV1,
    static_storages: StaticStorageFingerprintSetV1,
    initializations: InitializationFingerprintSetV1<I>,
}

pub type RegistrationObjectSetV2 = RegistrationObjectSetV1<
    scoop_lir::StrongTypeDescriptorRefV2,
    scoop_lir::StrongTypeDispatchCallableRefV2,
    scoop_lir::StrongInitializationDependencyRefV2,
>;

impl<D: scoop_lir::StrongDescriptorReference, C: Clone, I> RegistrationObjectSetV1<D, C, I> {
    pub fn new(
        safepoints: VerifiedStrongSafepointFingerprintSetV1,
        callables: VerifiedStrongCallableFingerprintSetV1,
        types: TypeFingerprintSetV1<D, C>,
        immortal_objects: ImmortalObjectFingerprintSetV1,
        static_storages: StaticStorageFingerprintSetV1,
        initializations: InitializationFingerprintSetV1<I>,
    ) -> Self {
        Self {
            safepoints,
            callables,
            types,
            immortal_objects,
            static_storages,
            initializations,
        }
    }

    pub const fn producer(&self) -> scoop_identity::ConeIdentity {
        self.safepoints.producer()
    }

    pub const fn safepoints(&self) -> &VerifiedStrongSafepointFingerprintSetV1 {
        &self.safepoints
    }

    pub const fn callables(&self) -> &VerifiedStrongCallableFingerprintSetV1 {
        &self.callables
    }

    pub const fn types(&self) -> &TypeFingerprintSetV1<D, C> {
        &self.types
    }

    pub const fn immortal_objects(&self) -> &ImmortalObjectFingerprintSetV1 {
        &self.immortal_objects
    }

    pub const fn static_storages(&self) -> &StaticStorageFingerprintSetV1 {
        &self.static_storages
    }

    pub const fn initializations(&self) -> &InitializationFingerprintSetV1<I> {
        &self.initializations
    }
}

#[cfg(test)]
mod tests;
