#define _POSIX_C_SOURCE 200809L
#include "internal.h"
#include "stream.h"

#include <inttypes.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

static void write_text(ScoopTraceWriter writer, const char *text) {
    writer.write(writer.context, text, strlen(text));
}

static void write_format(ScoopTraceWriter writer, const char *format, ...) {
    char local[256];
    va_list args;
    va_start(args, format);
    int length = vsnprintf(local, sizeof local, format, args);
    va_end(args);
    if (length < 0)
        abort();
    if ((size_t)length < sizeof local) {
        writer.write(writer.context, local, (size_t)length);
        return;
    }
    char *bytes = scoop_trace_allocate((size_t)length + 1);
    va_start(args, format);
    vsnprintf(bytes, (size_t)length + 1, format, args);
    va_end(args);
    writer.write(writer.context, bytes, (size_t)length);
    free(bytes);
}

static void render_frame(const void *bytes, const ScoopTraceFrame *frame,
                         const ScoopTraceModule *modules, const ScoopTraceSymbol *symbol,
                         ScoopTraceWriter writer) {
    write_text(writer, "  at ");
    if (symbol != NULL) {
        if (symbol->function != NULL)
            write_format(writer, "%s ", symbol->function);
        if (symbol->filename != NULL)
            write_format(writer, "(%s:%d) ", symbol->filename, symbol->line);
    }
    if (frame->module != SCOOP_TRACE_NO_MODULE) {
        const ScoopTraceModule *module = &modules[frame->module];
        const char *path = (const char *)bytes + module->path_offset;
        write_format(writer, "%s [", path);
        if (module->id_kind != 0) {
            write_text(writer, module->id_kind == 1   ? "uuid="
                               : module->id_kind == 3 ? "codeview="
                                                      : "build-id=");
            for (uint32_t byte = 0; byte < module->id_length; byte++) {
                write_format(writer, "%02x", module->id[byte]);
            }
        } else {
            write_text(writer, "module-id unavailable");
        }
        write_format(writer, ", link=0x%" PRIxPTR "]", frame->link_pc);
    } else {
        write_text(writer, "<module unavailable>");
    }
    write_format(writer, " (pc=0x%" PRIxPTR ", query=0x%" PRIxPTR ")\n", frame->raw_pc,
                 frame->query_pc);
}

void scoop_trace_write(const void *bytes, ScoopTraceWriter writer) {
    const ScoopTraceHeader *header = bytes;
    const ScoopTraceFrame *frames = (const void *)(header + 1);
    const ScoopTraceModule *modules = (const void *)(frames + header->frame_count);
    bool changed = false, unavailable = false;
    for (uint32_t index = 0; index < header->frame_count; index++) {
        const ScoopTraceFrame *frame = &frames[index];
        ScoopTraceSymbols symbols = {0};
        if (frame->module != SCOOP_TRACE_NO_MODULE)
            scoop_trace_symbolize(header->generation, frame->query_pc, &symbols);
        changed |= symbols.changed;
        unavailable |= symbols.unavailable;
        if (symbols.first == NULL) {
            render_frame(bytes, frame, modules, NULL, writer);
        } else {
            /* A symbolizer may supply several inline records for one PC. */
            for (ScoopTraceSymbol *symbol = symbols.first; symbol != NULL; symbol = symbol->next)
                render_frame(bytes, frame, modules, symbol, writer);
        }
        scoop_trace_symbols_free(&symbols);
    }
    if (header->frame_count == 0) {
        write_text(writer, "  <no stack frames available>\n");
    }
    if (header->flags & SCOOP_TRACE_TRUNCATED) {
        write_text(writer, "  <stack trace truncated>\n");
    }
    if (header->flags & SCOOP_TRACE_PARTIAL_UNWIND) {
        write_text(writer, "  <unwind stopped with a partial stack>\n");
    }
    if (header->flags & SCOOP_TRACE_BOUNDARY_MISSING) {
        write_text(writer, "  <caller boundary unavailable; runtime frames retained>\n");
    }
    if (changed) {
        write_text(writer, "  <loaded modules changed; preserved historical module addresses>\n");
    } else if (unavailable) {
        write_text(writer, "  <symbols unavailable for some frames; preserved module addresses>\n");
    }
}

static void write_stream(void *context, const void *bytes, size_t length) {
    fwrite(bytes, 1, length, context);
}
void scoop_trace_render(const void *bytes, FILE *stream) {
    scoop_trace_lock_stream(stream);
    scoop_trace_write(bytes, (ScoopTraceWriter){stream, write_stream});
    scoop_trace_unlock_stream(stream);
}
