# Registry 首版发布与管理设计草案

状态：讨论稿，2026-11-10。本文细化 M63 之后的发布工作，不表示命令已经实现，也不替代现行规范。当前合同见[包管理规范](../specs/SCOOP-PACKAGE-SPEC.md)，已实现范围见[实施记录](PROGRESS.md)，现有手工操作见[registry 指南](https://scoop-lang.org/guides/registry.html)。

## 1. 建议采用的首版形态

首版采用 **本地源码打包与索引维护工具、独立 Git 维护仓库、CI 部署静态文件**。公共 registry 通过 PR 接收版本，维护者审核后由 CI 发布；自托管者可以直接使用相同工具生成目录，再用自己的部署方式上传。

HTTP 站点继续只提供 GET。发布管理使用 Git 托管平台现有的账号、评审和部署功能；这些信息不进入包格式、求解、语言身份或构建过程。

| 部分 | 首版职责 |
| --- | --- |
| 库的源码仓库 | 源码、测试、Cone.toml、正常版本管理；产生最终源码归档 |
| `scoop pm package` | 展开清单、确定随包文件、生成归档；显式检查归档中的独立 Cone |
| `scoop pm registry …` | 维护本地静态目录，生成索引，添加版本与撤回／恢复版本 |
| registry 的 Git 仓库 | 保存版本变更、维护说明和部署配置，承载 PR 审核 |
| CI | 检查本次变更，部署已确定的文件，报告实际部署结果 |
| 静态站点 | 提供 config、每包索引和源码归档；可附带浏览页面 |
| 消费者 | 沿用现有 resolve／fetch／update、Cone.lock 和共享源码缓存 |

建议先提供一套小规模 Git＋静态站点的参考部署。源码归档也可以纳入该独立仓库，避免第一版就需要归档上传服务。对象存储是同一静态目录的另一种部署选择，不在 PM 内增加存储后端框架。

## 2. 已有基础与需要补齐的内容

已实现的消费者包括：

- `config.json`、`packages/<group>/<name>.json` 和按索引 URL 获取 tar.zst；
- 根据 metadata 求解，仅下载选中的版本；
- 首次下载摘要校验、归档读取、展开清单与索引匹配；
- lock、撤回版本的选择规则、离线使用和跨项目共享缓存。

对应实现主要位于 `compiler/pm/src/sources/registry/`。`RegistryIndex`／`RegistryVersion` 已有完整数据结构，`published_manifest` 已能检查独立发布清单并导出生产依赖。`compiler/manifest/src/package.rs` 已定义 `PublishConfig`，解析器已经接受 include、exclude 和文件映射。

尚未实现的是 package 和 registry 管理命令，以及使用这些命令的正式发布流程。现有规范第 9 节已给出大部分打包规则，本轮应把它变成可操作的工具与验收闭环。

另外有两个现有约束必须反映在设计中：

1. 客户端比较已发布 metadata 时，只有 `yanked` 可以不同；archive URL 也必须保持。新环境使用旧 lock 时仍可能读取旧版本的索引，因此必须同时保留索引记录和归档。
2. 格式 1 对未知字段报错。作者、维护者、简介、下载统计等信息不能直接追加到消费者索引。需要展示的信息放在网页数据或维护说明中。

## 3. 作者如何准备一个版本

### 3.1 打包与检查

下列命令均为待实现接口；根选择继续采用现有 workspace／`-p` 规则：

```sh
scoop pm package -p org.example:json --list
scoop pm package -p org.example:json --check
```

`--list` 展示最终文件清单和改写后的 Cone.toml，不写归档、不调用编译器。为了固定 Git 依赖，它可能需要获取原声明的 commit；支持复用来源匹配的 lock 和已有 Git 缓存，不能把它描述为始终无需获取来源的操作。

普通 package 只做源码分发所需的清单转换、输入检查和归档生成。`--check` 对本次最终归档解包后的独立 Cone 创建临时消费者 lock，然后调用普通 driver 做一次 release、locked 构建。检查使用包内声明的来源，不带入作者 workspace 的 path 绑定或根 overrides，不修改作者的 Cone.lock。

检查的 target 必须明确记录；默认本机，也可显式指定一个 target。跨平台项目在其支持的目标上运行相应检查，打包器不推导“支持所有平台”的结论。库行为测试仍通过普通 `scoop test` 完成，默认 package 不隐式运行测试。

输出路径沿用第 9 节的 `target/package/<group>/<name>/<version>.tar.zst`。若保留 `<version>.json`，它只是从归档信息派生的预览；registry 导入的唯一必要交付物是 tar.zst，正式索引从实际归档清单生成。

归档使用稳定文件顺序、固定 tar 时间／owner／mode 和明确的压缩设置。同一工具与输入应得到确定结果；不同工具版本之间不承诺压缩字节相同。检查通过后提交和部署同一份归档，不能在 CI 部署阶段重新打包。

### 3.2 发布清单的转换

| 开发时的声明 | 归档中的形式 |
| --- | --- |
| workspace 继承 | 展开为独立 Cone 的实际字段与依赖 |
| 普通 registry 依赖 | 每条都固定完整 registry URL，保留版本要求 |
| path 依赖 | 必须有发布版本要求与可确定的默认 registry，转换为该 registry 的依赖 |
| Git／GitHub 依赖 | 固定原来源的完整 commit；缺少版本要求时补入该 commit 清单的精确版本 |
| 无显式来源的 scoop 系统库 | 保留 sysroot 来源；隐式 core 在索引展开为精确边 |
| artifact 依赖 | 报错，由作者改为明确的源码来源 |
| workspace、overrides、publish 和本地 lock | 不作为消费者配置进入归档 |

生产与测试依赖都完成转换；索引只导出生产依赖。测试依赖不会变成消费者的传递生产依赖。

发布前需要的依赖版本应已可从其正式来源取得。首版按依赖顺序发布互相依赖的新版本：先发布底层库并部署成功，再检查和发布上层库。`package --workspace` 可以生成多个归档，但不隐式递归发布，也不承诺多个包同时生效。

### 3.3 文件与 native 输入

沿用当前草案的默认文件集、include／exclude 和 `[[publish.files]]` 映射。归档根直接包含独立 Cone.toml；源码、测试和 native 条件分支完整保留，不按打包主机截取文件集。

实施前应补全第 9 节的路径清单，使其覆盖目前实际存在的 manifest 能力：

- 生产与测试源码选择、native.sources、路径形式的 native.include、readme，以及 Windows 的 rc 输入；
- 现有 native argv 规则已识别的 include／forced-include 路径参数，包括相应的 Windows 参数；
- sysroot 相对 include 和逻辑 native libraries 保持原有含义，不复制宿主 SDK 或系统库；
- 源码内 includeStr／includeBytes／includeDir、C 头文件和 RC 中的相对资源关系，由作者保留相应文件布局。打包器不扫描并改写源码字符串或实现另一套预处理器；
- bundle 是外部工具的 metadata。PM 保留表，作者显式纳入其配置及资源；PM 不解析整套 bundle 配置，也不把源码包检查描述为应用安装包检查。

已识别路径不能转换为可独立使用的包内路径或现有逻辑系统输入时，定位到具体声明报错。对任意源码文本、工具 metadata 或宏参数不作“已证明全部路径可移植”的承诺，实际编译与应用打包仍按各自职责报告输入错误。

文件映射保留相对目录关系；symlink 按实际目标读取，归档只写普通文件／目录。必要的路径边界、映射冲突和遍历循环检查沿用普通 I/O 职责，不检查 owner、权限或文件系统来源。

## 4. 本地 registry 管理接口

建议补齐以下接口。前三组构成现有草案的核心，初始化和变更检查使参考部署不必再手写格式操作。

| 命令 | 行为 |
| --- | --- |
| `registry init --root <dir>` | 创建格式 1 的静态目录；已有合法配置可直接报告已初始化，冲突配置报错 |
| `registry add --root <dir> <archive>` | 读取最终归档，导出 metadata，先写归档，再更新对应包索引 |
| `registry yank/unyank --root <dir> <coordinate>` | 只更新指定完整版本的撤回状态；相同状态直接成功，缺少版本报错 |
| `registry check --root <dir> --against <base-dir>` | 供 PR 检查候选目录相对于已接纳目录的变化；只读取本次新增归档来完成归档／manifest 检查 |

统一通过 `scoop pm registry …` 或 `scoop-pm registry …` 调用。这里不增加远端上传命令或 PM 内登录功能。最小日常浏览可先直接使用网页和索引文件，独立 list／info 命令不作为首版交付前置条件。

例如：

```sh
scoop pm registry init --root ../scoop-registry/public
scoop pm registry add --root ../scoop-registry/public \
  target/package/org.example/json/1.4.3.tar.zst
scoop pm registry yank --root ../scoop-registry/public org.example:json:1.4.3
```

本地目录约定为：

```text
public/
  config.json
  packages/org.example/json.json
  archives/org.example/json/1.4.3.tar.zst
  index.html
```

add 默认生成相对 registry base 的 `archives/<group>/<name>/<version>.tar.zst`。PM 不把上传凭据、Git 维护仓库地址或网页域名写进 config.json。归档位于其他静态站点的既有消费能力保留；首套参考部署使用同一 base 下的文件，不先增加多套上传适配器。

### 4.1 更新、冲突与失败

add 在普通文件锁内更新对应包索引，保留全部旧版本，避免并发添加不同版本时丢失记录。已存在的完整版本报“已存在”，不提供覆盖选项，撤回后也不能重用版本。部署重试使用已提交的文件，不重新执行 package／add。

归档先写临时文件并完成正常读取，然后放到最终位置；索引通过同目录临时文件替换。失败不能让索引指向不完整归档。若归档已落盘而索引更新失败，可以留下未引用归档；重试仅针对该未发布候选确认相同字节后继续，不覆盖索引中已存在的版本。

check 比较两个目录的 config／包索引和文件变更：旧版本不得删除或改变固定字段，允许 yanked 变化；新增版本必须有匹配归档。CI 同时拒绝直接修改或删除旧归档路径。该检查复用已有格式和 published_manifest 规则，不建立第二套依赖语义检查，不重新下载或解包所有未变化的旧版本。

## 5. 公共 registry 的发布工作流

建议采用一个独立的 registry 仓库：

```text
scoop-registry/
  README.md
  MAINTAINERS.md
  .github/CODEOWNERS
  .github/workflows/
  public/
```

源码归档和消费者索引是提交中的实际发布内容，CI 不根据移动中的 Git tag 重新取得源码并打包。没有另一份必须手工维护、再转换成索引的版本数据库。

```mermaid
flowchart LR
    Source["库源码与正常测试"] --> Package["package：最终 tar.zst"]
    Package --> Add["registry add：本地静态目录变更"]
    Add --> PR["PR 检查与维护者审核"]
    PR --> Main["合入 registry 主分支"]
    Main --> Deploy["CI 部署同一批静态文件"]
    Deploy --> HTTP["HTTP GET 站点"]
    HTTP --> Client["现有 PM 解析、锁定与构建"]
    Yank["registry yank / unyank"] --> PR
```

具体流程：

1. 作者在源码仓库完成正常测试，检查最终归档；发布前先完成它依赖的新版本。
2. 作者在 registry checkout 中运行 add，提交归档和索引的实际差异，创建 PR。首次加入包时附上源码仓库、维护联系人和使用说明。
3. CI 检查本次新增归档及 metadata，比较既有版本是否被修改；在约定 target 对本次归档中的独立 Cone 做普通构建。作者本机检查和 registry 接收检查是两个实际输入边界；CI 内不再逐层重复同一事实。
4. registry 维护者审阅新增版本、清单转换、随包文件与必要测试结果。PR 的检查任务与部署任务分开，检查任务不需要部署凭据。
5. 合并前，以最新主分支检查冲突和变更。两条 PR 同时给一个包增加版本时，需要合并索引中的新增记录，不能让后合并者覆盖先前版本。
6. 部署任务串行运行，基于轮到它时的最新主分支生成完整静态目录，并部署已提交的归档。合并成功不等于发布成功；实际部署成功才表示消费者可以取得该版本。

新包接纳、后续版本、维护者变更和命名争议都通过普通 PR 处理。首版由少量 registry 维护者负责最终合并；包作者与联系信息记录在 MAINTAINERS.md，供人工协作，不参与 PM 判断一个包是否合法。

CODEOWNERS 用于请求 registry 维护团队审阅，不能假设所有外部包作者都可直接作为有效 code owner：GitHub 要求 code owner 对该仓库具有写权限。无需为自动分配评审而给每位作者整个 registry 的写权限，外部作者仍通过 fork／PR 参与。见 [GitHub CODEOWNERS 文档](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)。

group 是现有坐标的一部分，不因看起来像反向域名就增加域名验证。变更维护联系人不会改变 group:name:version、来源 URL 或源码内容。

## 6. 部署与版本管理规则

### 6.1 稳定地址与发布顺序

正式开放前确定自己控制的 registry base 和归档地址。推荐在同一稳定域名下使用相对 archive 路径，底层托管方式变化时继续保持原 URL 和原字节。

支持完整站点部署的托管方式，可部署包含所有旧记录、旧归档和新版本的完整目录。直接向对象存储／Web 目录更新时，先完成新增归档，再替换相应索引；不能用先上传索引再慢慢补文件的同步顺序。

两个版本之间不需要全 registry 事务。多个互不依赖的版本可以共同提交；相互依赖的新版本按第 3 节逐步发布，不增加预发布 registry 映射或跨 registry 事务。

config 和包索引采用适合更新的 HTTP 缓存策略；不可变归档可长期缓存。文件缺失返回正常 404，不使用 SPA fallback 返回 HTML。托管设置不改变客户端格式，也不要求客户端增加 HEAD 或条件请求。

GitHub Pages 可作为小规模参考部署；其目前的已发布站点上限为 1 GB，不能据此承诺容纳不断增长的大型 native 归档。规模超过所选托管能力时，可迁移稳定域名背后的存储；若既有归档使用另一域名，则必须继续服务原 URL，后续新版本才选用新 URL。参见 [Pages 限制](https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits)与[自定义部署工作流](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)。

### 6.2 生命周期

| 操作 | 对已发布数据和消费者的影响 |
| --- | --- |
| 发布新版本 | 新增一个完整版本；普通有效 lock 不自动更新 |
| 修复代码、依赖、遗漏文件或打包错误 | 发布新版本；不能覆盖原版本或重写摘要 |
| yank | 只改变 yanked；新选择排除，旧 lock 继续可用，`--precise` 不绕过撤回 |
| unyank | 恢复可被新解析选中；归档与其他 metadata 保持 |
| 修改介绍或维护联系人 | 只更新网页／维护说明；不改归档内已经发布的内容 |
| 更换代码托管位置 | 更新可变说明及后续版本，保留既有来源与归档地址 |
| 废弃一个包 | 页面说明替代方案，可撤回相关版本；保留全部记录和归档 |
| 更名或改变坐标 | 按新 key 发布，由作者提供迁移说明；不自动别名或换绑依赖 |

不把物理删除版本作为常规管理命令。撤回状态只有在客户端进行相关新解析并取得更新索引时才影响选择，不主动使已有缓存或 lock 失效，也不承诺离线客户端立即得知撤回。

### 6.3 失败与回退

PR 检查失败只影响候选变更。部署失败时报告实际失败步骤，重试同一份已确定文件；旧站点继续服务。逐对象上传产生未引用归档可以接受，索引不能提前暴露它。

成功发布新版本后，不能通过部署旧 Git 快照来“回滚包”，因为这会移除新 lock 所需的记录或归档，也可能撤销后来的 yank。错误版本使用 yank／新版本修复；回退网页或部署脚本时，继续携带当前完整包目录。

串行部署在取得执行机会后选取最新主分支，防止一个旧提交的慢任务最后覆盖新目录。这使用 CI 现有的部署并发控制，不设计独立发布序号、收据或事务服务。

## 7. 浏览页面与客户端协议分开

首版提供静态首页、包／版本列表、撤回标记、依赖声明示例和发布指南即可。列表从当前索引生成，网页生成失败不修改已存在的包内容。网页生成由参考仓库的小型构建脚本负责，不增加 PM 内的通用站点引擎。

后续需要 README、许可证、源码链接或更丰富的搜索时，可以从新归档中的既有 Cone 元数据生成独立页面数据。它们不加入格式 1 的求解索引。修改页面说明也不意味着可以更改已发布归档中的 README 或 license。

跨 target 的 API 文档构建、安装 executable、下载统计、评分、浏览器管理后台不作为此次发布工具交付的前置条件。现有 `scoop doc` 可以由作者正常使用，但不因此增加 registry 的文档编译服务。

## 8. 实施顺序与验收

| 阶段 | 交付内容 | 代表性验收 |
| --- | --- | --- |
| R1 打包 | 文件计划、清单转换、归档写入、list／check | 一个真实库从归档独立构建；workspace/path/Git 转换；native vendor 与平台文件完整 |
| R2 目录维护 | init、add、yank／unyank、相对基线的变更检查 | 输出被现有客户端直接消费；重复版本、失败不更新索引、普通并发不丢记录 |
| R3 参考发布 | 独立 registry 模板、PR 检查、串行静态部署、简洁页面和操作说明 | 从作者提交最终归档到新消费者下载、编译和运行的完整流程 |
| R4 正式启用 | 选定长期 URL、维护团队和托管位置，发布少量真实包 | 新环境使用旧 lock、显式 update、yank／unyank；部署重试保留全部已发布内容 |

工具实现集中在 `compiler/pm`，CLI 增加 package 和 registry 模块，复用 manifest/workspace、现有 registry 类型、published_manifest、归档读取与普通文件锁／原子写入。文件计划与清单转换可按实际代码规模拆模块，不另建 registry stage、通用插件或第二套格式 reader。`package --check` 调用既有 driver，PM 本体仍不依赖 LLVM。

测试重点：

- 打包文件集、映射冲突、漏掉明确声明的输入、稳定归档输出与两张依赖表的转换；
- 不把根 override 当作原 Git 来源，不发布浮动 ref 或本地 path／artifact；
- 实际 native vendor 编译，至少一个在非打包主机分支中声明的文件纳入归档；
- 并发添加同包不同版本、重复版本拒绝、yank 状态幂等、失败后索引仍可读；
- 使用真实发布命令改造现有静态 HTTP fixture，覆盖新版本消费和旧 lock 获取，不再仅手写 tar／JSON；
- 保留已有下载完整性、共享缓存与离线测试，不为每个库复制同一轮冷／热下载验收。

普通验证使用 release 和已有预编译 sysroot；源码、工具与原产物保持存在。归档格式、命令行、HTTP 发布和失败场景使用 PM 测试／必要 CLI fixture，普通库行为留在 Cone tests。修改代码后按仓库要求先格式化与 lint，再测试；验收日志放在仓库外。

## 9. 确认方案后需要同步的文档

首先细化包管理规范第 9 节：命令输入输出、全部已支持路径字段、检查 target、依赖的发布顺序、索引更新、重复版本与中断处理。第 8.2 节明确 archive URL 同其他固定 metadata 一起不可变，并明确旧 lock 同时依赖旧记录和归档。

第 10 节补入实际交付命令；package-manager 设计／进度与 ROADMAP 将这轮发布工作单独记录，不能把 M63 的已完成消费范围改写为发布已经完成。语言／编译器规范中的“源码打包留至下一阶段”只在实际交付时更新状态引用；本方案没有语言、IR、链接或 runtime ABI 变更。

当前建议按“小规模公开 registry 接受第三方 PR，同时允许自托管”实施。需要运营侧确定的是长期域名、维护仓库和初始维护团队；这些选择不阻塞 R1／R2 的本地工具实现。首版可以完整完成作者打包、审核、发布、撤回与消费者构建，而无需账户网站或上传服务。
