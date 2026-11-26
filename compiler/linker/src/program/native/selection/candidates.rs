use super::*;
use scoop_lir::NativeObjectFormat;

pub(in crate::program::native) enum Candidate {
    Definition,
    Object(NativeObjectId),
    Dynamic(crate::namespace::NativeBinding),
}

pub(in crate::program::native) fn candidate(
    symbol: &str,
    binding: NativeLibraryBinding,
    inputs: &mut ProgramInputs<'_>,
    profile: &ValidatedFinalLinkProfile,
) -> Result<Candidate, LinkError> {
    let explicit = match binding {
        NativeLibraryBinding::DefaultNativeNamespace => None,
        NativeLibraryBinding::Requirement(id) => {
            Some(inputs.native.libraries[&id].inputs.as_slice())
        }
    };
    let system_alias = match binding {
        NativeLibraryBinding::DefaultNativeNamespace => false,
        NativeLibraryBinding::Requirement(id) => inputs.native.libraries[&id].system_alias,
    };
    if let Some(owner) = inputs.definitions.get(symbol) {
        return match owner {
            DefinitionOwner::Native(id)
                if explicit.is_none_or(|input| input.contains(&id.input())) =>
            {
                Ok(Candidate::Object(*id))
            }
            _ if explicit.is_none() => Ok(Candidate::Definition),
            _ => Err(error(
                scoop_source::DiagnosticCode::E2306,
                format!(
                    "native library {explicit:?} cannot bind {symbol}, already defined by {owner:?}"
                ),
            )),
        };
    }
    let mut candidates: Vec<_> = inputs
        .namespace
        .candidates(&inputs.native, symbol, explicit, system_alias)?
        .into_iter()
        .map(|binding| {
            let strength = binding.symbol_binding();
            (Candidate::Dynamic(binding), strength)
        })
        .collect();
    for file in inputs
        .native
        .declared_files()
        .into_iter()
        .filter(|file| explicit.is_none_or(|ids| ids.contains(&file.id)))
    {
        if let Some((id, index)) = file.candidate(symbol) {
            candidates.push((Candidate::Object(id), index.definitions[symbol].binding));
        }
    }
    if explicit.is_none()
        && matches!(inputs.namespace, crate::namespace::NativeNamespace::Elf(_))
        && candidates.iter().any(|(candidate, binding)| {
            *binding == NativeSymbolBinding::Strong && matches!(candidate, Candidate::Object(_))
        })
    {
        // Object definitions precede toolchain archive fallbacks and interpose
        // shared runtime exports, including Rust staticlib arithmetic builtins.
        candidates.retain(|(candidate, _)| !system_candidate(candidate));
    }
    if let Some(candidate) = choose(symbol, candidates, inputs.target.native_object_format())? {
        return Ok(candidate);
    }
    match binding {
        NativeLibraryBinding::Requirement(id) => Err(error(
            scoop_source::DiagnosticCode::E2308,
            format!(
                "native library {} does not provide {symbol}",
                inputs.native.libraries[&id].key.library().as_str()
            ),
        )),
        NativeLibraryBinding::DefaultNativeNamespace => {
            if matches!(inputs.namespace, crate::namespace::NativeNamespace::Coff(_)) {
                return Ok(Candidate::Dynamic(crate::namespace::NativeBinding::Coff(
                    crate::namespace::coff::CoffBinding::Linker,
                )));
            }
            inputs.native.compiler_builtin(symbol, profile)?.map(Candidate::Object).ok_or_else(||
                error(scoop_source::DiagnosticCode::E2308,
                    format!("unresolved native symbol {symbol} in default namespace (unresolved symbol {symbol})")))
        }
    }
}

fn system_candidate(candidate: &Candidate) -> bool {
    matches!(
        candidate,
        Candidate::Dynamic(crate::namespace::NativeBinding::Elf(
            crate::namespace::elf::ElfBinding::System(_)
        ))
    )
}

fn choose(
    symbol: &str,
    candidates: Vec<(Candidate, NativeSymbolBinding)>,
    format: NativeObjectFormat,
) -> Result<Option<Candidate>, LinkError> {
    let Some(priority) = candidates
        .iter()
        .map(|(_, binding)| binding.precedence(format))
        .max()
    else {
        return Ok(None);
    };
    let best = candidates
        .into_iter()
        .filter(|(_, binding)| binding.precedence(format) == priority)
        .collect::<Vec<_>>();
    let lazy_archives = format == NativeObjectFormat::MachO64
        && best.iter().all(|(candidate, _)| {
            matches!(
                candidate,
                Candidate::Object(NativeObjectId::ArchiveMember(_))
            )
        });
    if best[0].1 == NativeSymbolBinding::Strong && best.len() > 1 && !lazy_archives {
        return Err(error(
            scoop_source::DiagnosticCode::E2306,
            format!("ambiguous native providers for {symbol}: multiple strong definitions"),
        ));
    }
    Ok(best.into_iter().next().map(|(candidate, _)| candidate))
}
