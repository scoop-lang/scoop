#include "loop_test.h"

typedef struct {
    ScoopIoWork work;
    unsigned *executed;
} Work;

static void execute(ScoopIoWork *native) {
    Work *work = (void *)native;
    ++*work->executed;
}

static void finish(ScoopIoWork *work, int status) { scoop_io_work_finished(work, status); }

static int submit(ScoopIoLoop *loop, int64_t id, unsigned *count) {
    Work *work = calloc(1, sizeof(*work));
    if (!work)
        return -ENOMEM;
    work->executed = count;
    int status = scoop_io_work_submit(&work->work, loop, id, execute, finish);
    if (status)
        free(work);
    return status;
}

int scoop_test_iocp_work(void) {
    ScoopIoCreated created = scoop_io_loop_create();
    CHECK(!created.status.code);
    ScoopIoLoop *loop = created.pointer;
    unsigned executed = 0;
    CHECK(!submit(loop, 1, &executed));
    scoop_io_work_cancel(loop, 1);
    ScoopIoEvent event;
    CHECK(!test_result(loop, &event));
    CHECK(event.id == 1 && event.kind == SCOOP_IO_POSIX_ERROR && event.code == ECANCELED);
    CHECK(!executed && !scoop_io_work_take(loop));

    CHECK(!submit(loop, 2, &executed));
    ScoopIoWorkTask *task = scoop_io_work_take(loop);
    CHECK(task);
    scoop_io_work_cancel(loop, 2);
    CHECK(!test_result(loop, &event));
    CHECK(event.id == 2 && event.kind == SCOOP_IO_POSIX_ERROR && event.code == ECANCELED);
    CHECK(!executed && !scoop_io_loop_idle(loop));
    // The payload has already been freed by loop_next; this is an empty task.
    scoop_io_work_run(task);
    CHECK(!executed);

    CHECK(!submit(loop, 3, &executed));
    task = scoop_io_work_take(loop);
    CHECK(task);
    scoop_io_work_run(task);
    scoop_io_work_cancel(loop, 3);
    CHECK(!test_result(loop, &event) && event.id == 3 && event.kind == SCOOP_IO_DONE);
    CHECK(executed == 1);
    CHECK(!test_drain(loop));
    return 0;
}
