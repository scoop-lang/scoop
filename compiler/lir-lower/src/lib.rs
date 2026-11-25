//! Lower typed MIR control flow and values into target-specific LIR.
//!
//! The lowering assigns physical layouts to values, plans callable ABIs and
//! makes loads, stores, calls, normal returns and unwind edges explicit.
//! Runtime calls carry the GC and exception effects consumed by codegen.
//! LLVM statepoints and native object emission belong to codegen.
//!
//! `lowering` prepares the module. `callable_abi`, `exact_layouts` and
//! `layout_exports` project the ABI and layout records consumed across Cones.
//! See `docs/specs/SCOOP-IMPL-SPEC.md` section 2.4.

use std::collections::HashMap;

use la_arena::Arena;
use scoop_lir as lir;

use scoop_mir as mir;

mod error;
pub use error::*;

mod closure_support;
mod identity_roots;
use identity_roots::IdentityRoots;

mod capability;
use capability::validate_strong_materialization;
pub use capability::{StrongLirCapabilityError, StrongLirMaterializationRequirement};

mod callable_abi;
pub use callable_abi::CallableAbiProjectionError;

mod cross_cone_bridge;
pub use cross_cone_bridge::{CrossConeLirBridgeLoweringError, lower_cross_cone_bridge_section};

mod exact_layouts;
pub use exact_layouts::{ExactLayoutLoweringError, lower_exact_layout_exports};

mod exact_callable_abi;
pub use exact_callable_abi::{ExactCallableAbiLoweringError, lower_exact_callable_abi_export};

mod layout_exports;
pub use layout_exports::{
    LayoutAbiDependencyLoweringError, LayoutAbiExportDependenciesV1, LayoutAbiExportInputV1,
    LayoutAbiExportLoweringError, lower_layout_abi_dependencies, lower_layout_abi_exports,
};

mod external_callables;
use external_callables::lower_external_callables;
mod dependency_types;

mod strong_production_v2;
pub use strong_production_v2::{
    StrongProductionV2ProjectionError, project_external_initialization_uses_v2,
};

mod debug;
mod lower_profile;
mod lowering;
pub use lowering::{PreparedLir, prepare};
#[cfg(test)]
mod test_pipeline;
#[cfg(test)]
use test_pipeline::lower;

mod abi;
mod callbacks;
mod externs;
mod globals;
mod initialization;
mod locals;
mod metadata;
mod native_abi;
mod production;
mod runtime;
mod safepoints;
mod target;

use callbacks::*;
use externs::*;
use globals::*;
use initialization::*;
use metadata::*;
use runtime::*;
use target::*;

pub use production::lower_entry_production_source;
use production::{lower_initialization_startup_gateway, lower_root_artifacts};

mod function;
use function::lower_function;

#[cfg(test)]
mod tests;
