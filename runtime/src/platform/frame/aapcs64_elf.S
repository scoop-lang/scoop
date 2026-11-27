/* Append the managed caller's LR, SP and FP to the C implementation arguments. */
.text

.macro SCOOP_MANAGED_ENTRY public, implementation, pc, stack, fp
.section .text.\public,"ax",%progbits
.p2align 2
.globl \public
.hidden \public
.type \public,%function
\public:
    .cfi_startproc
    mov \pc, x30
    mov \stack, sp
    mov \fp, x29
    b \implementation
    .cfi_endproc
.size \public, .-\public
.endm

SCOOP_MANAGED_ENTRY scoop_rt_safepoint, scoop_rt_safepoint_impl, x0, x1, x2
SCOOP_MANAGED_ENTRY scoop_rt_gc_collect, scoop_rt_gc_collect_impl, x0, x1, x2
SCOOP_MANAGED_ENTRY scoop_rt_prepare_throwable, scoop_rt_prepare_throwable_impl, x2, x3, x4
SCOOP_MANAGED_ENTRY scoop_rt_throwable_print_stack_trace, scoop_rt_throwable_print_stack_trace_impl, x1, x2, x3
SCOOP_MANAGED_ENTRY scoop_runtime_alloc_slow, scoop_runtime_alloc_slow_impl, x2, x3, x4
SCOOP_MANAGED_ENTRY scoop_rt_string_concat, scoop_rt_string_concat_impl, x2, x3, x4
SCOOP_MANAGED_ENTRY scoop_rt_init_enter, scoop_rt_init_enter_impl, x1, x2, x3
SCOOP_MANAGED_ENTRY scoop_rt_init_succeed, scoop_rt_init_succeed_impl, x1, x2, x3
SCOOP_MANAGED_ENTRY scoop_rt_init_fail, scoop_rt_init_fail_impl, x2, x3, x4
SCOOP_MANAGED_ENTRY scoop_rt_init_failure, scoop_rt_init_failure_impl, x1, x2, x3
SCOOP_MANAGED_ENTRY scoop_rt_init_cycle_message, scoop_rt_init_cycle_message_impl, x1, x2, x3
SCOOP_MANAGED_ENTRY scoop_rt_context_push, scoop_rt_context_push_impl, x3, x4, x5
SCOOP_MANAGED_ENTRY scoop_rt_context_ensure_root, scoop_rt_context_ensure_root_impl, x1, x2, x3
SCOOP_MANAGED_ENTRY scoop_rt_enter_native_safe, scoop_rt_enter_native_safe_impl, x2, x3, x4
SCOOP_MANAGED_ENTRY scoop_rt_enter_native_borrowed, scoop_rt_enter_native_borrowed_impl, x2, x3, x4

.section .note.GNU-stack,"",%progbits
