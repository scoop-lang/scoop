//! Process-wide LLVM options for Scoop's GC, arithmetic, and machine layout.

use std::ffi::CStr;
use std::sync::Once;

const TARGET_INDEPENDENT_OPTIONS: &[&CStr] = &[
    c"--disable-tail-duplicate",
    c"--tail-dup-placement=false",
    c"--hints-allow-reordering=false",
];

// LLVM exposes these X86 policies only as process command-line options. They
// are installed unconditionally with the common options, so the first target
// machine created in a compiler process cannot choose the policy of later
// target machines.
const X86_BACKEND_OPTIONS: &[&CStr] = &[
    c"--no-x86-call-frame-opt",
    c"--x86-experimental-pref-innermost-loop-alignment=5",
];

pub(super) fn initialize_process_options() {
    static INITIALIZE: Once = Once::new();
    INITIALIZE.call_once(|| {
        // X86 call-frame optimization can replace the reserved argument area
        // with pushes. Stack maps then report the fixed frame size while the
        // call-site SP includes extra arguments, violating the runtime profile.
        // Both the post-register-allocation tail-duplication pass and block
        // placement can copy a STATEPOINT after the site plan is finalized.
        // LLVM 22.1 does not preserve noduplicate on that machine instruction,
        // so keep the shared tail and its single return PC in both passes.
        // A vectorization hint must not implicitly authorize FP reassociation.
        // LLVM otherwise reorders reductions even without fast-math flags.
        // Prefer 32-byte innermost X86 loop entries, keeping small kernels in
        // one fetch block even when preceding code grows. Pad before the loop.
        let arguments = std::iter::once(c"scoop-codegen")
            .chain(TARGET_INDEPENDENT_OPTIONS.iter().copied())
            .chain(X86_BACKEND_OPTIONS.iter().copied())
            .map(CStr::as_ptr)
            .collect::<Vec<_>>();
        // SAFETY: the constant argument strings and pointer array remain valid
        // for the call. Initialization runs once before any target machine.
        unsafe {
            inkwell::llvm_sys::support::LLVMParseCommandLineOptions(
                arguments.len() as i32,
                arguments.as_ptr(),
                c"".as_ptr(),
            );
        }
    });
}

#[cfg(test)]
pub(super) fn process_option_spellings() -> Vec<&'static str> {
    TARGET_INDEPENDENT_OPTIONS
        .iter()
        .chain(X86_BACKEND_OPTIONS)
        .map(|option| option.to_str().expect("static LLVM option is UTF-8"))
        .collect()
}
