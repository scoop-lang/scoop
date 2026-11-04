#ifndef SCOOP_RT_TASK_CONTEXT_H
#define SCOOP_RT_TASK_CONTEXT_H

#include "scoop_rt.h"

/* Only these two compiler-owned shapes allocate Context heap objects. */
typedef struct ScoopContextNode {
    ScoopObjectHeader header;
    void *slots[4];
} ScoopContextNode;

typedef struct ScoopTaskContext {
    ScoopObjectHeader header;
    ScoopContextNode *root;
    void *execution;
} ScoopTaskContext;

_Static_assert(sizeof(ScoopContextNode) == 48, "Context node ABI");
_Static_assert(sizeof(ScoopTaskContext) == 32, "Task Context ABI");
_Static_assert(offsetof(ScoopTaskContext, root) == 16, "Task Context root ABI");
_Static_assert(offsetof(ScoopTaskContext, execution) == 24, "Task Context execution ABI");

#endif
