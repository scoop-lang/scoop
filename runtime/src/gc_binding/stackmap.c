#include "stackmap.h"
#include <stdlib.h>

static int compare_record_pc(const void *left, const void *right) {
    uintptr_t a = ((const ScoopGcPcEntry *)left)->return_pc;
    uintptr_t b = ((const ScoopGcPcEntry *)right)->return_pc;
    return (a > b) - (a < b);
}

bool scoop_stackmap_finish_index(ScoopStackMapIndex *index, uintptr_t *duplicate_pc) {
    qsort(index->records, index->record_count, sizeof *index->records, compare_record_pc);
    for (size_t record = 1; record < index->record_count; record++) {
        if (index->records[record - 1].return_pc == index->records[record].return_pc) {
            *duplicate_pc = index->records[record].return_pc;
            return false;
        }
    }
    return true;
}

void scoop_stackmap_dispose_index(ScoopStackMapIndex *index) {
    if (index != NULL) {
        free(index->records);
        *index = (ScoopStackMapIndex){0};
    }
}

const ScoopGcPcEntry *scoop_stackmap_lookup(const ScoopStackMapIndex *index,
                                            uintptr_t return_pc) {
    if (index == NULL)
        return NULL;
    size_t low = 0, high = index->record_count;
    while (low < high) {
        size_t middle = low + (high - low) / 2;
        uintptr_t candidate = index->records[middle].return_pc;
        if (candidate < return_pc)
            low = middle + 1;
        else if (candidate > return_pc)
            high = middle;
        else
            return &index->records[middle];
    }
    return NULL;
}
