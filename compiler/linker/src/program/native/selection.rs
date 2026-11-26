use super::*;
use crate::NativeSymbolBinding;

#[cfg(all(test, unix))]
mod tests;

mod candidates;
pub(super) use candidates::{Candidate, candidate};

pub(super) fn include(
    inputs: &mut ProgramInputs<'_>,
    runtime: &RuntimeObjectSet,
    id: NativeObjectId,
    reason: &str,
) -> Result<(), LinkError> {
    let file = &inputs.native.files[&id.input()];
    let (index, _) = inputs.native.object(id)?;
    let origin = format!("native object {id} selected by {reason}");
    let diagnostic = format!("{origin} ({})", file.locator.display());
    for symbol in index.definitions.keys() {
        if inputs.compiler_owned(symbol) {
            return Err(error(
                scoop_source::DiagnosticCode::E2306,
                format!("{diagnostic} defines compiler-owned symbol {symbol}"),
            ));
        }
        if let Some(previous) = inputs.definitions.get(symbol) {
            if matches!(previous, DefinitionOwner::Runtime(_))
                && inputs.target.native_object_format() == scoop_lir::NativeObjectFormat::Coff64
            {
                let old = runtime.symbols().definitions[symbol];
                let new = index.definitions[symbol];
                if old.binding == NativeSymbolBinding::Weak && old == new {
                    continue;
                }
            }
            if let DefinitionOwner::Native(previous) = previous {
                let (previous_index, _) = inputs.native.object(*previous)?;
                let old = previous_index.definitions[symbol];
                let new = index.definitions[symbol];
                if old.binding != NativeSymbolBinding::Strong
                    || new.binding != NativeSymbolBinding::Strong
                {
                    if new.binding.precedence(inputs.target.native_object_format())
                        > old.binding.precedence(inputs.target.native_object_format())
                    {
                        inputs
                            .definitions
                            .insert(symbol.clone(), DefinitionOwner::Native(id));
                    }
                    continue;
                }
            }
            let previous = match previous {
                DefinitionOwner::Native(previous) => inputs.native.files[&previous.input()]
                    .locator
                    .display()
                    .to_string(),
                other => format!("{other:?}"),
            };
            return Err(error(
                scoop_source::DiagnosticCode::E2306,
                format!("{diagnostic} conflicts with {previous} at {symbol}"),
            ));
        }
        inputs
            .definitions
            .insert(symbol.clone(), DefinitionOwner::Native(id));
    }
    for symbol in &index.requirements {
        inputs.requirements.insert(symbol.clone());
        inputs
            .requirement_origins
            .entry(symbol.clone())
            .or_default()
            .push(origin.clone());
    }
    inputs.native.selected.insert(id, reason.into());

    Ok(())
}

pub(super) fn resolve(
    inputs: &mut ProgramInputs<'_>,
    declarations: &BTreeMap<String, Declarations<'_>>,
    runtime: &RuntimeObjectSet,
    profile: &ValidatedFinalLinkProfile,
    roots: &[std::path::PathBuf],
) -> Result<(), LinkError> {
    let mut processed = std::collections::BTreeSet::new();
    add_autolinks(inputs, &mut processed, profile, roots)?;
    let mut pending = inputs.requirements.clone();
    let mut unresolved = BTreeMap::new();
    let linker_symbols = profile
        .elf_inputs()
        .map_or(&[][..], |inputs| inputs.linker_defined_symbols);
    while let Some(symbol) = pending.pop_first() {
        if linker_symbols.contains(&symbol.as_str()) || inputs.linker_defined(&symbol) {
            continue;
        }
        if matches!(profile, ValidatedFinalLinkProfile::Darwin(_))
            && !declarations.contains_key(&symbol)
            && !inputs.definitions.contains_key(&symbol)
            && symbol
                .strip_prefix("_objc_msgSend$")
                .is_some_and(|selector| !selector.is_empty())
        {
            // Darwin's linker emits the selector stub; only its dispatcher is imported.
            let dispatcher = "_objc_msgSend".to_owned();
            inputs.requirements.insert(dispatcher.clone());
            pending.insert(dispatcher);
            continue;
        }
        let source_symbol = if inputs.target.native_object_format()
            == scoop_lir::NativeObjectFormat::Coff64
            && !declarations.contains_key(&symbol)
            && let Some(native) = symbol.strip_prefix("__imp_")
            && declarations.get(native).is_some_and(|declaration| {
                matches!(
                    declaration.contract,
                    NativeExternalContract::ReadOnlyData { .. }
                        | NativeExternalContract::MutableData { .. }
                )
            }) {
            native
        } else {
            symbol.as_str()
        };
        let binding = declarations
            .get(source_symbol)
            .map_or(NativeLibraryBinding::DefaultNativeNamespace, |decl| {
                decl.contract.library()
            });
        let origin = inputs
            .requirement_origins
            .get(&symbol)
            .map(|origins| origins.join(", "))
            .unwrap_or_else(|| "program startup/linker support".into());
        let candidate = match candidate(source_symbol, binding, inputs, profile) {
            Ok(candidate) => candidate,
            Err(err) => {
                let err = error(
                    err.code(),
                    format!("{err}; reference chain: {symbol} <- {origin}"),
                );
                if err.code() != scoop_source::DiagnosticCode::E2308 {
                    return Err(err);
                }
                // A later selected member may introduce this symbol's library.
                unresolved.insert(symbol, err);
                continue;
            }
        };
        match candidate {
            Candidate::Object(id) => {
                if !inputs.native.selected.contains_key(&id) {
                    include(inputs, runtime, id, &format!("{symbol} <- {origin}"))?;
                    let (index, _) = inputs.native.object(id)?;
                    pending.extend(index.requirements.iter().cloned());
                    add_autolinks(inputs, &mut processed, profile, roots)?;
                    pending.extend(unresolved.keys().cloned());
                    unresolved.clear();
                }
            }
            Candidate::Definition => continue,
            Candidate::Dynamic(binding) => {
                for requirement in inputs.namespace.bind(source_symbol.to_owned(), binding)? {
                    inputs.requirements.insert(requirement.clone());
                    inputs
                        .requirement_origins
                        .entry(requirement.clone())
                        .or_default()
                        .push(format!("native dynamic input selected by {symbol}"));
                    pending.insert(requirement);
                }
            }
        }
    }
    if let Some((_, error)) = unresolved.pop_first() {
        return Err(error);
    }
    // A member selected for another symbol can add a definition after a
    // dynamic binding was chosen. Revisit only these changed resolutions.
    let changed: Vec<_> = inputs
        .namespace
        .bound_symbols()
        .into_iter()
        .filter(|symbol| inputs.definitions.contains_key(*symbol))
        .cloned()
        .collect();
    for symbol in changed {
        let declaration = declarations.get(&symbol);
        let binding = declaration.map_or(NativeLibraryBinding::DefaultNativeNamespace, |decl| {
            decl.contract.library()
        });
        let candidate = candidate(&symbol, binding, inputs, profile).map_err(|err| {
            let origins = declaration
                .map(|decl| decl.origins.join(", "))
                .unwrap_or_default();
            error(err.code(), format!("{err}; origins: {origins}"))
        })?;
        if let (Candidate::Object(id), Some(declaration)) = (candidate, declaration) {
            let (index, _) = inputs.native.object(id)?;
            check_kind(&symbol, declaration.contract, index.definitions[&symbol])?;
        }
        inputs.namespace.unbind(&symbol);
    }
    Ok(())
}

fn add_autolinks(
    inputs: &mut ProgramInputs<'_>,
    processed: &mut std::collections::BTreeSet<NativeObjectId>,
    profile: &ValidatedFinalLinkProfile,
    roots: &[std::path::PathBuf],
) -> Result<(), LinkError> {
    let selected: Vec<_> = inputs
        .native
        .selected
        .keys()
        .filter(|id| !processed.contains(id))
        .copied()
        .collect();
    let mut added = false;
    for id in selected {
        processed.insert(id);
        let file = &inputs.native.files[&id.input()];
        let (_, range) = inputs.native.object(id)?;
        let requirements = crate::native_input::autolink::read(&file.bytes[range], profile)?;
        for key in requirements {
            added |= inputs.native.add(
                key,
                vec![format!("autolink from native object {id}")],
                roots,
                profile,
            )?;
        }
    }
    if added {
        inputs
            .namespace
            .extend(&mut inputs.native, roots, profile)?;
    }
    Ok(())
}
