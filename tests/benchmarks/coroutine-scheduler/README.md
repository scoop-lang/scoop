# 协程调度主路径基准

用于把队列与 worker 调度成本从 TCP、event loop 和 HTTP 中分离。相同源码分别链接改动前后的固定 `scoop.concurrent` 产物，使用 release 和相同 runtime；结果保存在仓库外。

```sh
~/.scoop/bin/scoop build tests/benchmarks/coroutine-scheduler/main.scoop --release \
  --cone-path /path/to/fixed/libraries -o /path/outside/repository/scheduler
taskset -c 0 /path/outside/repository/scheduler 1 64 32768 dispatch
taskset -c 0,2,4,6 /path/outside/repository/scheduler 4 64 32768 yield
```

四个参数依次为 worker 数、任务数、每任务操作数、模式：

- `dispatch`：低层 `CoroutineWork` 每次运行后重新排队，直接测 executor 接纳、队列、worker 领取与唤醒。所有可变负载状态在下一次发布之前更新，发布之后不再访问；允许下一个 worker 在 dispatch 返回前接手。
- `yield`：正常编译的协程反复执行 `yieldCoroutine()`，包括 continuation、execution 与 Context 成本。

每次先以每任务 256 次操作预热，再创建正式 worker。开始门打开后计时，直到全部工作完成、worker 排空并 join；线程创建与初始任务登记不计入该窗口。输出实际核对过的操作数和单调时钟纳秒数。总调度调用还包括固定的启动／退出开销，不能将它视为单次 CAS 的耗时。

用相同 CPU 集合、新旧交错各三次比较；构建、测试和测量不能同时运行。外部 `/usr/bin/time` 的 CPU／RSS 覆盖整个进程（含预热、启动和收尾），其口径不同于程序输出的计时窗口。该基准用于解释调度成本，应用表现仍以共用客户端的裸 TCP echo 等组合负载验证。
