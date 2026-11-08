# 静态常量与图标资源按需裁剪设计

状态：实现与七目标定向验收完成；发行打包按本次用户要求取消。日期：2026-11-09。源码调研基线：`f3fd7f9f8`。

本设计补齐最终链接对 immortal 静态对象的按需选择，以 `scoop.icons.lucide` 为实际消费者，接续[不可达 callable 裁剪](CALLABLE-PRUNING.md)和[类型／shape 元数据裁剪](TYPE-PRUNING.md)。第 2 节保留实施前的调研基线；目标规则已先同步到规范，完成情况以验收记录为准。

## 1. 需求与范围

应用仅使用 `search` 时，最终可执行程序只保留实际需要的图标资源；未使用图标的 SVG 数据、独立 getter 机器码、callable 登记和 immortal 登记均不进入成品。在库中新增未被该应用引用的图标，不应增加这些内容。库 `.slib` 仍保存供其他消费者使用的完整候选。

“使用”由最终保留的代码和数据的实际引用闭包决定。应用经组件、默认参数、泛型、闭包或其他 Cone 间接引用的资源同样需要保留。声明依赖、import、public 可见性和资源的源码所属函数本身不构成保留理由。

本项处理已有的三类 immortal 静态对象：普通字符串及 `includeStr`／`env` 产生的 String、`includeBytes` 的 `Array<Byte>`、`includeDir` 的文件表及其路径／字节对象。按对象的既有表示处理，不按 intrinsic 名称或 icon 库名称设置特例。

Lucide 继续采用[现有独立入口](../../sysroot/src/scoop.icons.lucide/src/generated/search.scoop)：

```scoop
public val search: SvgAsset get() {
    return SvgAsset(includeStr("../../assets/search.svg"))
}
```

`SvgAsset` 是共享的普通 struct，每个图标没有独立 class 或持久包装对象。getter 由已有 callable 选择处理；使用的 getter 可以保留为调用，也可以内联，不能将必须内联作为资源裁剪的前提。

本项不扩展全局存储、singleton、eager／lazy 初始化和 generated-C bridge 的裁剪，不改变它们的保留规则。常量求值、常量内容合并、native dead stripping、LTO、资源压缩和动态资源加载不在本次范围内。无需新增注解、CLI 开关、图标专用构建步骤或运行时资源索引。

## 2. 当前行为与证据

[GUI 扩展实施记录](../desktop-ui/GUI-EXTENSIONS-PROGRESS.md)中，真实离屏消费者分别使用一个和全部 31 个 Lucide 图标，两者均保留 31 份 SVG，原始资源合计 12,159 字节。整个程序还包含 Skia 和 runtime，不能将其完整文件大小当作图标开销。

2026-11-09 的最小复现复制这 31 份 SVG 和 getter，并在独立 Cone 中复现相同的 `SvgAsset` struct，隔离资源与包装的链接行为。实验使用 macOS AArch64、现有 `bd056f602` 工具；其 codegen 和 linker 与源码调研基线相同。此实验不代替真实 graphics.svg／UI 集成测试。

| 消费方式 | profile | 最终 SVG 数 | 保留的独立图标 getter |
| --- | --- | ---: | --- |
| 不声明资源库依赖 | release | 0 | 无 |
| 声明依赖但不引用图标 | release | 31 | 无 |
| 只引用 search | release | 31 | 无，使用的 getter 已内联 |
| 引用全部 31 个图标 | release | 31 | 无，使用的 getter 已内联 |
| 只引用 search | debug | 31 | 仅 search |
| 新增一个未引用 SVG，仍只引用 search | release | 32 | 无 |

六个程序均编译、运行并核对输出。检查使用 `--keep-symbols`，对照 `symbols.json` 中的 getter 符号和 body ID，同时查找完整资源字节；显式 debug 实验关闭源码调试信息。原始日志、二进制和 JSON 报告保存在仓库外，不作为提交内容。

两个独立机制导致资源无法删除：

1. [物理对象分组](../../compiler/codegen/src/object_partition.rs)将普通 Strong 的 immortal 对象和登记放入共享 `non_callable` member，可能同时包含该 Cone 的其他静态存储和初始化定义。源码文件独立不等于物理 member 独立。
2. [可达性选择](../../compiler/linker/src/program/selection/reachable.rs)将 `ImmortalObject` 和 `ImmortalRegistration` 都列为无条件根。即使没有 getter 或其他数据引用，资源也会被选中。

分组与保留根必须一起调整。只细分对象仍会全部保留；只移除根会使同组的其他根或一个被使用的常量带入其余资源。

实现规范 2.10.1 已描述按引用选择常量的目标，但 2.19 及当前实现仍保留全部 immortal。本次文档澄清该目标尚未实现，避免将既有文字当作已经具备的能力。

## 3. 物理分组

### 3.1 普通 Strong 对象

每个 `PersistentImmortalObjectId` 的 `ImmortalObject` 与 `ImmortalRegistration` 合成一个独立物理 member。分组直接使用已有 typed entity 和 definition role；[登记生产计划](../../compiler/lir/src/production/immortal_registrations.rs)已经保存同一对象的这两个 definition。

不同对象不能仅因同属一个 Cone、同一源码文件或同一 getter 而合组。String 的公共 TD／TypeRegistration 不并入资源 member，继续通过原有符号引用选择。`Array<EmbeddedFile>` 的文件表与其路径、字节对象也分别分组，父子关系由已有 relocation 表达。

此分组复用 codegen 的 metadata 模块投影和对象发射路径，不为每个常量重新生成一份完整模块。数据与登记同组，避免为必须一起保留的两项分别承担对象容器开销。

### 3.2 ODR 与 callable 私有数据

ODR 对象和登记保留原有 ODR member key、shared ABI 与独立分组。先沿现有规则选定 ODR 定义，再在已选定义之间按同一 immortal ID 建立必须同选的关联；不把不同 ODR member 强行合并成新的 group。

callable 的代码、EH、GC 表、callable registration、函数私有机器常量和 Context cells 仍在原 member 内。共享 immortal 常量不因源码归属于某个 callable 就并入该 callable；源码所有权用于身份，实际符号引用用于存活选择。

## 4. 最终链接选择

### 4.1 根与关联

在 `SelectedMembers::new` 完成现有 Strong／ODR 选择后，扩展 `retain_reachable` 的既有遍历。

| 定义或入口 | 选择规则 |
| --- | --- |
| RootEntryDescriptor、NativeAppEntryDescriptor | 保持无条件根 |
| StaticStorage、RootRegistration、InitializationCell、InitializationRegistration | 保持无条件根 |
| GeneratedBridge、generated-C member | 保持无条件根 |
| runtime 引用、typed runtime data alias 目标、已声明 native 输入的 Scoop 引用 | 保持现有入口规则；archive 候选引用仍保守处理 |
| ImmortalObject、ImmortalRegistration | 移出无条件根，按引用和同选关联到达 |
| callable、TD／type registration、layout／scan／dispatch | 沿现有引用及同选规则处理 |
| 候选 image 的完整登记表 | 不作为遍历根，也不作为重新保活候选的引用来源 |

使用现有完整 typed entity 中的 `PersistentImmortalObjectId` 配对对象与登记，不能按符号显示名、路径、SVG 内容或布局配对。新增的关联只存在于此次链接的选择数据中，不新增 wire 字段或持久可达性表。

### 4.2 遍历步骤

1. 复用已选 member 的定义、符号 owner、requirements 和现有类型登记关联。
2. 从已选定义收集 immortal 对象和对应登记的 member，以 `PersistentImmortalObjectId` 关联。只关联经过 ODR 选择的定义，不从被淘汰候选中补登记。
3. 按 4.1 初始化待访问集合，移除两种 immortal role 的无条件入口。
4. 访问 member 时，沿现有 `(member, symbol)` requirements 到达被引用 member；对象到达时，同时到达它的登记 member。普通 Strong 配对已在同一 member，ODR 的分离定义通过该关联闭合。
5. 登记中的实际 relocation 保留对象与其 TypeRegistration；对象头继续引用真实 TD。现有 TD 与 TypeRegistration 的同选关系、父类型、scan 和其他引用继续处理。
6. 复用原有 visited 集合直到没有新 member。未被根到达的对象／登记即使互相引用，也不因此存活；不另建 SCC 分析。
7. 从保留 member 重建所选 definition 集合，再由现有 `SelectedImage` 过滤五类登记表。被删除资源的登记不能重新出现在最终 image 中。

登记到对象的机器引用只解决一个方向。ODR 对象被代码引用时，仍需显式的“对象必须带登记”关联；不能只删除根判定而漏掉这一关系。缺失必要关联沿现有链接格式／引用错误路径报告，不在 runtime 新建补登记或恢复流程。

不重新扫描机器对象、读取 HIR、恢复源码调用图或重放常量语义。完整输入已经完成的格式、类型和引用检查继续复用。

### 4.3 内联与跨 Cone

未内联的 getter 通过其机器引用保留资源。getter 被内联后，consumer 中的代码继续通过现有 imported immortal 引用保留 provider 的数据；getter 本体及其 callable 登记可以删除。不能从对象 ID 的源码 owner 反向保活 getter，也不能因为 getter 不再保留而删除仍被 consumer 引用的数据。

同一 provider 资源被多个 Cone 使用时，引用仍指向既有对象身份。泛型、默认参数和闭包只消费当前管线实际物化的对象和引用；本项不新增强制去重，也不根据相同字节合并不同声明或泛型 application。

### 4.4 静态对象图

`includeDir` 的目录表被保留时，其实际引用的全部路径和字节数组及各自登记必须保留；目录表和子对象没有其他使用者时可以一起删除。对子对象的独立实际引用仍可保留该子对象。

本项按对象粒度选择，不拆分目录数组内部条目。使用完整目录表后，仅在运行时读取其中一个文件，不意味着其他条目可删除。同理，运行时图标名称索引或包含全部资产的数组会保留其真实引用的全部图标；Lucide 应继续使用独立 getter。

## 5. 语言与运行时约束

选择对 debug／release 默认生效，使用同一规则；不同优化结果可以具有不同的实际引用集合。`--keep-symbols` 只影响符号保留，不控制代码或资源的存活。

编译期输入仍在所属源码的语义检查中展开，包括未使用声明和运行期不可达分支。源码／资源错误、实际输入变化和失败不发布的行为保持；最终产物裁剪不用于跳过必要的读取和诊断。普通 const 的身份与可合并存储规则继续遵守语言规范 9.1.2、11.19。

保留的 immortal 对象继续满足运行时规范 2.4、2.8、3.3：对象起点、大小、对齐、TD、登记和静态引用闭合；对象只读且没有指向可移动对象的槽。collector 对这些对象的处理、managed root、pin／handle 及 native 借用协议保持。数据和登记同步退出 image，不增加运行期 liveness 查询或资源表。

保留的静态存储初值、初始化函数、native 入口和 runtime data alias 对资源的引用必须继续生效。即使应用源码没有直接读取该资源，只要这些现有根仍引用它，就不能删除。本项不推断初始化副作用，也不借裁剪改变一次性初始化时机。

最终 Cone 的依赖身份和 image 组织仍遵守现有合同。本项不以整 Cone 消失为验收条件；只要求该 Cone 内未使用的资源及其包装、登记缺席。

## 6. 实现位置与产物更新

| 模块 | 必要改动 |
| --- | --- |
| [codegen/object_partition.rs](../../compiler/codegen/src/object_partition.rs) | 普通 Strong immortal 对象与登记按 ID 合组，并从共享静态 member 中分离 |
| [linker/program/selection/reachable.rs](../../compiler/linker/src/program/selection/reachable.rs) | 调整根，建立已选 immortal 对象到登记的关联，复用现有遍历 |
| [linker/program/image.rs](../../compiler/linker/src/program/image.rs) | 复用所选 definition 过滤登记表，验证空表与移除登记的结果 |
| LIR／slib 数据通道 | 消费已有 entity、role、物理 member 和完整符号引用；不另建语义检查或可达性编码 |
| driver／sysroot 准备 | 按普通构建流程重新生产受影响库，消费新的物理分组 |
| Lucide／graphics.svg | 保持公开 API 和资源组织，补产物验收并在通过后更新裁剪说明 |

本设计不改变 `.slib` 字段、实体 ID 编码、对象表示或 runtime／metadata ABI。物理分组变化产生新的对象 member、内容和正常产物指纹；新的分组不改变语言实体身份。

实施后需使用更新的编译器重建 core 和实际依赖库，包含 Lucide。仅重链接旧的粗分组库不能保证逐图标删除。项目未发布，不为旧分组新增兼容分支、reader 或迁移机制；已有 reader 仍按输入实际 member 选择，验收使用重新生产的库。

编译器与输入变化沿现有更新记录使缓存失效。输入未变化且结果存在时继续直接复用，不为确认资源裁剪读取命中产物、重跑符号选择或重新准备链接。产物检查只放在下述专测和显式测量中。

## 7. 规范与文档同步

实现批次先将以下目标规则落实到对应规范，再修改代码；库的用户说明随实际验收结果更新：

| 文档 | 同步内容 |
| --- | --- |
| 实现规范 2.10.1 | 常量按真实引用选择，数据与 immortal 登记必须同选，注明静态对象图的闭合关系 |
| 实现规范 2.19 | 根集合移除 immortal 两种 role；规定 Strong 对象／登记分组、ODR 关联及最终 image 过滤；静态存储与初始化仍保留 |
| 运行时规范 2.4、2.8、3.3、7 | 最终只登记保留的 immortal，对象与登记仍一一闭合；移除第 7 章对全部 immortal 无条件保留的现状描述 |
| SVG 规范第 5 节、Lucide README、GUI 扩展进度 | 真实资源产物验收通过后声明支持范围；历史测量保留为当时结果 |
| 语言规范 | 保持现有 const、include、初始化和对象身份语义，无新增语言 API |
| ROADMAP／性能进度 | 按核心实现、真实图标及各平台实际验收进度更新，不提前声明完成 |

## 8. 验收

### 8.1 编译器正确性专测

在 `tests/fixtures/compiler-correctness/immortal-pruning` 组织一组代表性源码／产物用例，复用既有符号 dump 和链接检查方式。普通资源行为仍放所属 Cone 的 `scoop.test`；fixture 只承担最终产物、跨 Cone、链接和外部进程检查。

| 场景 | 必须观察到的结果 |
| --- | --- |
| 同库多个独立 String／字节数组，零引用、单引用、多引用 | 只保留实际引用对象及登记；未使用 getter 和 callable 登记缺席 |
| 新增唯一内容的未引用资源及 getter | 原消费者输出正确；新增 payload、定义和两类登记均未进入成品 |
| 跨 Cone 的直接调用与实际发生的内联 | getter 保留与否由原优化决定；活资源始终存在，内联后不能因删除 getter 而丢资源 |
| 泛型／默认参数物化与现有 ODR 重复候选 | 在选中定义间配对登记，实际使用的对象有效；不从被淘汰候选补记录 |
| includeDir 文件表、路径与字节子对象 | 活文件表的全部实际子引用闭合；未使用对象图可删除；独立子引用有效 |
| 静态初值、eager／lazy 初始化、runtime alias／必要 native 引用 | 原有入口及副作用顺序保持，所引用资源及登记有效 |
| String、Array<Byte>、目录表存入 managed 对象并经过定向 GC | 数据、类型和读取结果正确，显式 GC 验证不出现缺失 immortal 登记 |

codegen 单测检查真实发射对象中的 Strong 配对、不同对象分离、ODR 原有 member 划分，以及 callable 私有数据分组保持。链接专测检查对象到登记的关联、资源登记全部移除后的空 image 表，并复用现有 callable／type pruning、初始化和 native 引用专测。只对被本项影响的合同增加断言。

二进制检查保留符号，对照当前产物的 typed 身份、实际定义符号和 image 登记；独特 payload 的完整字节用于辅助确认。仅有符号消失不足以证明数据已删除，文件大小相同也不足以证明没有新增资源。内容相同的对象以身份与实际选择关系区分，不用字符串搜索推断是否发生合法共享。需要检查身份字节的用例关闭源码调试信息，避免把调试描述误判为运行登记。

Microsoft linker 产生的 PE 不带 COFF 符号表时，使用实际登记中的 immortal semantic ID、callable body ID、资源字节及运行／GC 结果检查存活关系。有符号表的产物同时检查对象、登记和 getter 名称；不能把缺少符号表误判为没有保留对象。

编译器专测覆盖 debug／release 和当前支持目标的相应对象／链接路径，包含 Darwin 双架构、Linux GNU／musl 和 Windows 三种 native 环境。先完成本机，再记录各平台实际运行与 GC 结果；未执行的组合明确保留为未验收。对一组代表产物复核普通 build 与独立链接入口，不为每个资源重复冷／热构建。GC stress 仅用于上述 GC 正确性场景。

### 8.2 真实 Lucide 验收

使用更新后的真实预编译 `scoop.icons.lucide` 和 graphics.svg，在库支持的 GUI 目标上运行 release 消费者：

| 消费者 | Lucide 资源预期 |
| --- | --- |
| 仅声明依赖，不读取任何图标 | 0 份 SVG，0 个图标 getter／相关登记 |
| 仅使用 search 并实际解析／绘制 | 仅 search 资源及必要登记；其余 30 份资源与包装缺席 |
| 使用全部清单图标 | 全部资源有效，解析与绘制结果正确 |
| 扩充库清单后重建，消费者仍只使用 search | 新增图标数据、getter 和登记缺席，search 继续可用 |

消费者不用含有其他默认图标的 UI 组件来断言“仅一份资源”。组件集成另按实际引用闭包检查，组件需要的默认图标属于有效使用。全部图标的普通解析／像素测试复用原 Cone 测试，不复制一套行为 fixture。

普通消费者复用目标预编译库和共享 Cone cache。只有扩充清单这一项隔离被修改的图标库输入并重新生产该库；源码、工具和原产物保持存在。不得通过隐藏输入来验证预编译消费或缓存命中。

### 8.3 空间与构建成本

首先确认未使用资源和登记确实缺席，再分别记录资源 payload、登记、可装载只读数据、代码段及完整文件大小。完整文件受对齐、符号和签名影响，不要求扩充未用图标后的 executable 逐字节相同，也不以固定文件大小差值作为正确性门槛。

比较 Lucide 库本身及代表性 core 常量的 `.slib` 大小、物理 member 数、库生产耗时与峰值内存，并记录预编译消费者的编译和最终链接成本。独立分组可能增加库容器和构建开销，必须与最终资源减少一并报告；不另设通用资源预算。

需要登记计数时使用现有 `--enable-startup-stats` 单独测量；普通产物和普通运行不加入统计代码。原始日志和逐次 JSON 放仓库外，文档只保留范围、结果、失败闭合和限制。

## 9. 实施顺序与完成条件

1. 按第 7 节先同步目标规范；实现 Strong 分组和链接登记关联，并完成相关单测。两个改动共同进入功能验收。
2. 正常重建所需系统库，完成编译器专测，确认 inline、ODR、静态初值和 GC 后的资源访问闭合。
3. 运行真实 Lucide 零／单／全引用及扩充实验，完成目标平台验证与空间／构建成本测量，再更新库声明和进度。

交付要求是使用中的图标继续正确运行，未使用图标的资源、独立包装代码和登记都不进入最终可执行程序，且保留数据满足原有 GC／ABI 合同。README 随真实图标验收更新，并链接各平台的实际完成范围。

## 10. 实施记录

### 2026-11-09：核心选择与本地编译器验收

`codegen/object_partition` 将每个 Strong immortal 与登记独立合组；ODR 两种 definition 继续各自分组。`program/selection/reachable` 使用既有 typed entity 同时关联类型与 immortal 的登记，移除 immortal 两种 role 的无条件根，复用现有符号遍历和五表投影。没有新增 wire 字段、runtime ABI、用户开关或库特例。

新增 `immortal-pruning` 专测覆盖零／单／多引用、增加未使用资源、release 实际内联后删除 getter、默认参数、三个 Cone 的相同泛型候选、静态初值与初始化、目录对象图、managed 保存及 MOVE／MINOR GC。检查实际 payload、对象与登记符号、semantic ID 和 callable body ID；零引用 provider 的全部 immortal 登记缺席。codegen 发射测试验证七个目标的两种 profile，Strong 配对与 ODR 独立成员均在真实对象中成立。

Darwin AArch64：格式化与 Clippy 通过；codegen 397 项及 linker 16 项单测闭合。codegen 首次全量有一项既有 Windows return-PC 专测失败：默认 Homebrew LLVM 缺少仓库要求的 WinEH 补丁；改用已准备的配套 LLVM 22.1.8 动态库后，该项定向复验通过。不是裁剪逻辑回退。正式 CLI 的 immortal／callable／type pruning、三个 include 产物、native-app lifecycle 与 startup-stats 共 8 个 fixture、15 个变体、92 个过程通过；fixture runner 82 项规则测试通过，含 4 项平台条件跳过。

本节不将对象交叉发射等同于目标平台实际运行。

### 2026-11-09：真实 Lucide 与构建成本

重建 Darwin AArch64 系统库及带 SVG／Expat 的 Skia 后，`lucide-pruning` 正式 CLI 专测通过。零／单／全引用消费预编译库，扩库只复制和修改图标 Cone；保留原单引用程序，再检查扩库程序，避免相同输出路径覆盖基线。检查全部实际 SVG、对象／登记符号、semantic ID 和未用 getter body ID。单引用程序实际解析并在 RasterSurface 绘制 search；普通解析／绘制行为复用 Lucide 的全部 31 图标像素测试和 SVG 的 14 项测试，均通过，包含 Metal。

| 消费者 | SVG 数／原始字节 | 含头及对齐的 SVG 对象字节 | SVG immortal 登记字节 | 完整 executable 字节 |
| --- | ---: | ---: | ---: | ---: |
| 零引用 | 0／0 | 0 | 0 | 22,732,552 |
| 仅 search，解析并绘制 | 1／352 | 376 | 152 | 22,871,512 |
| 全部 31 图标，读取资源 | 31／12,159 | 13,008 | 4,712 | 22,790,632 |
| 增加第 32 个未用图标，仍仅 search | 1／352 | 376 | 152 | 22,871,512 |

单引用相对全部资源少 11,807 字节 SVG，计入对象头／对齐和 30 条登记后少 17,192 字节静态内容，不含符号表和 image 表项。完整文件的三个消费者执行不同代码；单引用的绘图路径使其整体比纯资源读取程序更大，不能反推裁剪效果。单引用与扩库程序的 `__text` 均为 6,853,852 字节，`__TEXT,__const`／`__DATA_CONST,__const` 均为 11,515,048／593,608 字节；这些段包括 Skia 等共享内容。两次文件大小相等只是本次观察，不是逐字节相同的合同。

同一 Lucide 源码和预编译依赖的库构建对照如下。基线工具为已安装的 `4a7cec6e`，其 codegen、linker、LIR、driver 和 slib 与调研基线无差异；本次工具包含核心裁剪实现。使用 4 workers、各自空编译缓存；单次样本没有控制操作系统页缓存，不据此声称构建加速。

| 项目 | 裁剪前 | 裁剪后 |
| --- | ---: | ---: |
| Lucide `.slib` 字节 | 156,085 | 176,234 |
| 物理对象 member | 33 | 63 |
| 容器全部 member | 36 | 66 |
| 冷构建墙钟 | 1.94 s | 1.77 s |
| 最大 RSS | 1,403,142,144 B | 1,400,242,176 B |

增加 30 个对象分组使库容器增加 20,149 字节（12.9%）；完整候选仍供其他消费者使用。当前 core `.slib` 为 12,260,284 字节、15,626 个物理对象；单次冷构建 5.18 s，最大 RSS 1,767,194,624 B。已安装旧 core 的源码版本不同，不将其大小差异作为纯分组成本。预编译单图标消费者单独编译为 root `.slib` 为 1.94 s／1,506,541,568 B RSS，独立最终链接为 2.44 s／900,808,704 B RSS；运行输出正确。RSS 为 `/usr/bin/time -l` 的最大 resident set 指标，不是并行进程组内存之和。

### 2026-11-09：七目标运行验收与收尾

使用本次编译器重建被测系统库及依赖，完成以下实际编译、链接和运行。表中只统计最终通过的有效用例，失败尝试与定向复验不重复计数；这是本设计的定向验收。

| 目标／执行环境 | 通过 fixture／变体／过程 | Lucide／SVG 普通测试 |
| --- | ---: | ---: |
| `aarch64-apple-darwin` | 9／16／97 | 1／14 |
| `x86_64-apple-darwin`，Rosetta | 9／16／97 | 1／14 |
| `x86_64-unknown-linux-gnu`，nuc12 | 9／16／97 | 1／13 |
| `x86_64-unknown-linux-musl`，nuc12 | 8／15／92 | 不适用，未提供 GUI |
| `x86_64-pc-windows-msvc-ucrt` | 6／10／65 | 1／13 |
| `x86_64-w64-windows-llvm-mingw-ucrt` | 6／10／65 | 1／13 |
| `x86_64-w64-windows-llvm-mingw-msvcrt` | 6／10／65 | 1／13 |

七个目标均通过 `immortal-pruning` 的 debug／release、独立链接、对象图闭合及 MOVE／MINOR GC 检查。六个 GUI 目标均通过真实 Lucide 零／单／全引用及扩库专测：分别保留 0／1／31 份 SVG，新增未用图标后仍只有 352 字节的 search。每个 GUI 目标的 1 项 Lucide Cone 测试覆盖全部 31 个图标的解析和实际像素；Darwin 的 14 项 SVG 测试包含真实 Metal，其余 GUI 目标为 13 项。Darwin x86_64 在 Apple Silicon 上通过 Rosetta 运行，不记作 Intel 硬件验证。

既有回归包括三个 include 产物用例和 native-app lifecycle；Darwin／Linux 另含 callable／type pruning 与 startup-stats。Windows 的这三项既有专测按声明不适用，musl 的 Lucide 用例不适用，均不计为通过。全平台合计 53 个有效 fixture 执行、93 个变体、578 个过程。

跨平台阶段只调整验证脚本，裁剪实现无需追加平台分支。发现的问题已分别闭合：

- GNU 的 fixture runner 将显式 `clang` 符号链接展开为 `clang-22`，导致误找不存在的 `clang++-22`。保留调用路径后真实 C++／SVG 链接通过；新增调用名回归，Ruff 及 83 项 runner 规则测试通过（79 通过、4 项平台跳过）。
- Microsoft linker 的 PE 没有 COFF 符号表。检查实际登记 ID 的唯一性与缺席、资源字节、callable body ID 和 GC；有符号表的产物继续同时核对符号。三个 Windows 目标及 Darwin／GNU 定向复验通过。
- 检查脚本的成功提示改为写出明确的 LF 字节，保持 runner 的原始字节断言，消除 Windows Python CRLF 带来的误报。

每批变更提交并推送后同步 `origin/main`。安装包制作已按用户要求取消；原始日志、JSON 报告与库体积对照保存在仓库外，临时构建与验证目录按本次清理约定回收。

