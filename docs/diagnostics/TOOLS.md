# 包管理编辑器与 API 工具错误文案

本表共 **226** 条去重错误相关文案。计数规则、范围及限制见 [盘点总表](INVENTORY.md)。序号仅用于本次清单定位，不是诊断编号；“错误类型显示”和“错误构造/上下文”包含内部合同、格式错误和外层包装。

归并后的规则与编号见 [编号方案](NUMBERING.md)，本表每行的处置见 [完整归并对照](RULE-MAPPING.md)。旧序号保留用于追溯。

## analysis（33 条）

| 清单序号 | 领域/实现模块 | 来源形式 | 英文文案模板 | 实现位置 |
| --- | --- | --- | --- | --- |
| <a id="t0001"></a>1 | host | 错误构造/上下文、错误类型显示 | ` analysis revision changed ` | [analysis/src/host/edits.rs:20](../../compiler/analysis/src/host/edits.rs#L20)；[lsp/src/convert/location.rs:38](../../compiler/lsp/src/convert/location.rs#L38)；[lsp/src/diagnostics.rs:85](../../compiler/lsp/src/diagnostics.rs#L85)；[lsp/src/worker.rs:90](../../compiler/lsp/src/worker.rs#L90)；[lsp/src/worker.rs:279](../../compiler/lsp/src/worker.rs#L279) |
| <a id="t0002"></a>2 | host | 错误类型显示 | ` edit source is unavailable ` | [analysis/src/host/edits.rs:21](../../compiler/analysis/src/host/edits.rs#L21) |
| <a id="t0003"></a>3 | host | 错误类型显示 | ` edit source version changed ` | [analysis/src/host/edits.rs:22](../../compiler/analysis/src/host/edits.rs#L22) |
| <a id="t0004"></a>4 | host | 错误类型显示 | ` edit range is not a source character range ` | [analysis/src/host/edits.rs:24](../../compiler/analysis/src/host/edits.rs#L24) |
| <a id="t0005"></a>5 | host | 错误类型显示 | ` source edits overlap ` | [analysis/src/host/edits.rs:26](../../compiler/analysis/src/host/edits.rs#L26) |
| <a id="t0006"></a>6 | semantic | 错误构造/上下文 | ` selected core input is unavailable ` | [analysis/src/semantic/dependencies.rs:181](../../compiler/analysis/src/semantic/dependencies.rs#L181) |
| <a id="t0007"></a>7 | semantic | 错误构造/上下文 | ` selected core has no compiler protocols ` | [analysis/src/semantic/dependencies.rs:185](../../compiler/analysis/src/semantic/dependencies.rs#L185) |
| <a id="t0008"></a>8 | semantic | 错误类型显示 | ` rename was cancelled ` | [analysis/src/semantic/rename.rs:47](../../compiler/analysis/src/semantic/rename.rs#L47) |
| <a id="t0009"></a>9 | semantic | 错误类型显示 | ` rename analysis is unavailable: {error:?} ` | [analysis/src/semantic/rename.rs:48](../../compiler/analysis/src/semantic/rename.rs#L48) |
| <a id="t0010"></a>10 | semantic | 错误类型显示 | ` new name is not a valid source identifier ` | [analysis/src/semantic/rename.rs:50](../../compiler/analysis/src/semantic/rename.rs#L50) |
| <a id="t0011"></a>11 | semantic | 错误类型显示 | ` rename would change a declaration or reference binding ` | [analysis/src/semantic/rename.rs:52](../../compiler/analysis/src/semantic/rename.rs#L52) |
| <a id="t0012"></a>12 | semantic | 错误构造/上下文 | ` local binding has no source declaration origin ` | [analysis/src/semantic/rename/anchors.rs:69](../../compiler/analysis/src/semantic/rename/anchors.rs#L69) |
| <a id="t0013"></a>13 | semantic | 错误构造/上下文 | ` source declaration origin is ambiguous ` | [analysis/src/semantic/rename/anchors.rs:96](../../compiler/analysis/src/semantic/rename/anchors.rs#L96) |
| <a id="t0014"></a>14 | semantic | 错误构造/上下文 | ` renamed declaration has no editable project source ` | [analysis/src/semantic/rename/plan.rs:63](../../compiler/analysis/src/semantic/rename/plan.rs#L63)；[analysis/src/semantic/rename/plan.rs:75](../../compiler/analysis/src/semantic/rename/plan.rs#L75) |
| <a id="t0015"></a>15 | semantic | 错误构造/上下文 | ` related declarations do not share an editable source name ` | [analysis/src/semantic/rename/plan.rs:68](../../compiler/analysis/src/semantic/rename/plan.rs#L68) |
| <a id="t0016"></a>16 | semantic | 错误构造/上下文 | ` one source name also refers to a different declaration ` | [analysis/src/semantic/rename/plan.rs:86](../../compiler/analysis/src/semantic/rename/plan.rs#L86) |
| <a id="t0017"></a>17 | semantic | 错误构造/上下文 | ` symbol has an implicit or operator use without an editable identifier ` | [analysis/src/semantic/rename/plan.rs:91](../../compiler/analysis/src/semantic/rename/plan.rs#L91) |
| <a id="t0018"></a>18 | semantic | 错误构造/上下文 | ` rename source is unavailable ` | [analysis/src/semantic/rename/world.rs:32](../../compiler/analysis/src/semantic/rename/world.rs#L32)；[lsp/src/queries/rename.rs:73](../../compiler/lsp/src/queries/rename.rs#L73) |
| <a id="t0019"></a>19 | semantic | 错误构造/上下文 | ` rename source cannot be parsed ` | [analysis/src/semantic/rename/world.rs:35](../../compiler/analysis/src/semantic/rename/world.rs#L35) |
| <a id="t0020"></a>20 | semantic | 错误构造/上下文 | ` rename requires complete source syntax ` | [analysis/src/semantic/rename/world.rs:40](../../compiler/analysis/src/semantic/rename/world.rs#L40) |
| <a id="t0021"></a>21 | semantic | 错误构造/上下文 | ` rename requires complete declaration and body references ` | [analysis/src/semantic/rename/world.rs:55](../../compiler/analysis/src/semantic/rename/world.rs#L55) |
| <a id="t0022"></a>22 | semantic | 错误构造/上下文 | ` rename requires successful source analysis ` | [analysis/src/semantic/rename/world.rs:64](../../compiler/analysis/src/semantic/rename/world.rs#L64) |
| <a id="t0023"></a>23 | semantic | 错误构造/上下文 | ` rename requires complete member relationships ` | [analysis/src/semantic/rename/world.rs:73](../../compiler/analysis/src/semantic/rename/world.rs#L73)；[analysis/src/semantic/rename/world.rs:78](../../compiler/analysis/src/semantic/rename/world.rs#L78) |
| <a id="t0024"></a>24 | semantic | 错误构造/上下文 | ` declaration syntax is unavailable ` | [analysis/src/semantic/rename/world.rs:118](../../compiler/analysis/src/semantic/rename/world.rs#L118) |
| <a id="t0025"></a>25 | workspace | 错误构造/上下文 | ` dependency coordinate mismatch: expected {coordinate:?}, found {:?} ` | [analysis/src/workspace/artifacts.rs:53](../../compiler/analysis/src/workspace/artifacts.rs#L53) |
| <a id="t0026"></a>26 | workspace | 错误构造/上下文 | ` dependency must be a manifest library ` | [analysis/src/workspace/artifacts.rs:64](../../compiler/analysis/src/workspace/artifacts.rs#L64) |
| <a id="t0027"></a>27 | workspace | 错误构造/上下文 | ` prepared target or purpose differs from the request ` | [analysis/src/workspace/graph.rs:34](../../compiler/analysis/src/workspace/graph.rs#L34) |
| <a id="t0028"></a>28 | workspace | 错误构造/上下文 | ` dependency input {next} is unavailable ` | [analysis/src/workspace/graph/order.rs:20](../../compiler/analysis/src/workspace/graph/order.rs#L20) |
| <a id="t0029"></a>29 | workspace | 错误构造/上下文 | ` dependency cycle reaches {:?} ` | [analysis/src/workspace/graph/order.rs:30](../../compiler/analysis/src/workspace/graph/order.rs#L30) |
| <a id="t0030"></a>30 | workspace | 错误构造/上下文 | ` package manager returned a different member selection ` | [analysis/src/workspace/loader.rs:100](../../compiler/analysis/src/workspace/loader.rs#L100) |
| <a id="t0031"></a>31 | workspace | 错误构造/上下文 | ` cannot run package manager {}: {error} ` | [analysis/src/workspace/package.rs:147](../../compiler/analysis/src/workspace/package.rs#L147)；[scoop/src/cli/package_manager.rs:66](../../compiler/scoop/src/cli/package_manager.rs#L66)；[scoop/src/cli/package_manager/members.rs:66](../../compiler/scoop/src/cli/package_manager/members.rs#L66)；[scoop/src/cli/package_manager/prepare.rs:135](../../compiler/scoop/src/cli/package_manager/prepare.rs#L135) |
| <a id="t0032"></a>32 | workspace | 错误构造/上下文 | ` package manager failed ({}): {} ` | [analysis/src/workspace/package.rs:179](../../compiler/analysis/src/workspace/package.rs#L179) |
| <a id="t0033"></a>33 | workspace | 错误构造/上下文 | ` invalid package manager response: {error} ` | [analysis/src/workspace/package.rs:187](../../compiler/analysis/src/workspace/package.rs#L187) |

## api（13 条）

| 清单序号 | 领域/实现模块 | 来源形式 | 英文文案模板 | 实现位置 |
| --- | --- | --- | --- | --- |
| <a id="t0034"></a>34 | main | 错误构造/上下文 | ` cannot locate installation root ` | [api/src/main.rs:60](../../compiler/api/src/main.rs#L60) |
| <a id="t0035"></a>35 | main | 错误构造/上下文 | ` provide --sysroot or --docs, or install scoop-api in <SYSROOT>/commands ` | [api/src/main.rs:65](../../compiler/api/src/main.rs#L65) |
| <a id="t0036"></a>36 | main | 错误构造/上下文 | ` query must be nonempty and limit must be positive ` | [api/src/main.rs:81](../../compiler/api/src/main.rs#L81) |
| <a id="t0037"></a>37 | main | 错误构造/上下文 | ` unknown host default; provide --target or --docs ` | [api/src/main.rs:96](../../compiler/api/src/main.rs#L96) |
| <a id="t0038"></a>38 | main | 错误构造/上下文 | ` unknown complete target {target:?} ` | [api/src/main.rs:104](../../compiler/api/src/main.rs#L104) |
| <a id="t0039"></a>39 | query | 错误构造/上下文 | ` cannot read {}: {e} ` | [api/src/query.rs:59](../../compiler/api/src/query.rs#L59)；[toolchain/src/c_bridge.rs:67](../../compiler/toolchain/src/c_bridge.rs#L67) |
| <a id="t0040"></a>40 | query | 错误构造/上下文 | ` invalid API data {}: {e} ` | [api/src/query.rs:60](../../compiler/api/src/query.rs#L60) |
| <a id="t0041"></a>41 | query | 错误构造/上下文 | ` unsupported API schema {actual}; expected {SCHEMA} ` | [api/src/query.rs:68](../../compiler/api/src/query.rs#L68) |
| <a id="t0042"></a>42 | query | 错误构造/上下文 | ` {error}; generate documentation explicitly with scoop doc, or update installed documentation with scoop-install docs ` | [api/src/query.rs:76](../../compiler/api/src/query.rs#L76) |
| <a id="t0043"></a>43 | query | 错误构造/上下文 | ` documentation target is {}, requested {target} ` | [api/src/query.rs:82](../../compiler/api/src/query.rs#L82) |
| <a id="t0044"></a>44 | query | 错误构造/上下文 | ` API coordinate mismatch in {} ` | [api/src/query.rs:93](../../compiler/api/src/query.rs#L93) |
| <a id="t0045"></a>45 | query | 错误构造/上下文 | ` duplicate API declaration {key} ` | [api/src/query.rs:108](../../compiler/api/src/query.rs#L108) |
| <a id="t0046"></a>46 | query | 错误构造/上下文 | ` cyclic API parent at {key} ` | [api/src/query.rs:163](../../compiler/api/src/query.rs#L163) |

## lsp（45 条）

| 清单序号 | 领域/实现模块 | 来源形式 | 英文文案模板 | 实现位置 |
| --- | --- | --- | --- | --- |
| <a id="t0047"></a>47 | cli | 错误构造/上下文 | ` unexpected response id {} ` | [lsp/src/cli/client.rs:61](../../compiler/lsp/src/cli/client.rs#L61) |
| <a id="t0048"></a>48 | cli | 错误构造/上下文 | ` unexpected response while waiting for diagnostics ` | [lsp/src/cli/client.rs:80](../../compiler/lsp/src/cli/client.rs#L80) |
| <a id="t0049"></a>49 | cli | 错误构造/上下文 | ` language server closed before completing the query{details} ` | [lsp/src/cli/client.rs:122](../../compiler/lsp/src/cli/client.rs#L122) |
| <a id="t0050"></a>50 | cli | 错误构造/上下文 | ` language server exited with {status}{details} ` | [lsp/src/cli/client.rs:144](../../compiler/lsp/src/cli/client.rs#L144) |
| <a id="t0051"></a>51 | cli | 错误构造/上下文 | ` server stderr reader stopped ` | [lsp/src/cli/client.rs:155](../../compiler/lsp/src/cli/client.rs#L155) |
| <a id="t0052"></a>52 | cli | 错误构造/上下文 | ` server selected {negotiated} instead of {} ` | [lsp/src/cli/query.rs:72](../../compiler/lsp/src/cli/query.rs#L72) |
| <a id="t0053"></a>53 | cli | 错误构造/上下文 | ` query requires a document ` | [lsp/src/cli/query.rs:100](../../compiler/lsp/src/cli/query.rs#L100) |
| <a id="t0054"></a>54 | cli | 错误构造/上下文 | ` Cone.toml has no parent directory ` | [lsp/src/cli/query.rs:155](../../compiler/lsp/src/cli/query.rs#L155) |
| <a id="t0055"></a>55 | cli | 错误构造/上下文 | ` {} has no Cone.toml; select a Cone project with --project ` | [lsp/src/cli/query.rs:162](../../compiler/lsp/src/cli/query.rs#L162) |
| <a id="t0056"></a>56 | cli | 错误构造/上下文 | ` document has no parent directory ` | [lsp/src/cli/query.rs:180](../../compiler/lsp/src/cli/query.rs#L180) |
| <a id="t0057"></a>57 | cli | 错误构造/上下文 | ` document requires a file name ` | [lsp/src/cli/query.rs:183](../../compiler/lsp/src/cli/query.rs#L183) |
| <a id="t0058"></a>58 | cli | 错误构造/上下文 | ` cannot represent {} as a file URI ` | [lsp/src/cli/query.rs:200](../../compiler/lsp/src/cli/query.rs#L200) |
| <a id="t0059"></a>59 | convert | 错误构造/上下文 | ` source path cannot be represented as a URI ` | [lsp/src/convert/location.rs:45](../../compiler/lsp/src/convert/location.rs#L45) |
| <a id="t0060"></a>60 | documents | 错误构造/上下文 | ` document is already open ` | [lsp/src/documents.rs:70](../../compiler/lsp/src/documents.rs#L70) |
| <a id="t0061"></a>61 | documents | 错误构造/上下文 | ` document is not open ` | [lsp/src/documents.rs:102](../../compiler/lsp/src/documents.rs#L102)；[lsp/src/documents.rs:105](../../compiler/lsp/src/documents.rs#L105) |
| <a id="t0062"></a>62 | documents | 错误构造/上下文 | ` document version must increase ` | [lsp/src/documents.rs:109](../../compiler/lsp/src/documents.rs#L109) |
| <a id="t0063"></a>63 | documents | 错误构造/上下文 | ` edit range is reversed ` | [lsp/src/documents.rs:125](../../compiler/lsp/src/documents.rs#L125) |
| <a id="t0064"></a>64 | documents | 错误构造/上下文 | ` source file is unavailable ` | [lsp/src/documents.rs:180](../../compiler/lsp/src/documents.rs#L180) |
| <a id="t0065"></a>65 | documents | 错误构造/上下文 | ` invalid file URI ` | [lsp/src/documents.rs:231](../../compiler/lsp/src/documents.rs#L231) |
| <a id="t0066"></a>66 | queries | 错误构造/上下文 | ` code action data is required ` | [lsp/src/queries.rs:144](../../compiler/lsp/src/queries.rs#L144) |
| <a id="t0067"></a>67 | queries | 错误构造/上下文 | ` unknown request {} ` | [lsp/src/queries.rs:216](../../compiler/lsp/src/queries.rs#L216) |
| <a id="t0068"></a>68 | queries | 错误构造/上下文 | ` request cancelled ` | [lsp/src/queries.rs:235](../../compiler/lsp/src/queries.rs#L235)；[lsp/src/queries.rs:276](../../compiler/lsp/src/queries.rs#L276)；[lsp/src/queries/semantic.rs:284](../../compiler/lsp/src/queries/semantic.rs#L284)；[lsp/src/server/notifications.rs:35](../../compiler/lsp/src/server/notifications.rs#L35)；[lsp/src/worker.rs:103](../../compiler/lsp/src/worker.rs#L103)；[lsp/src/worker.rs:215](../../compiler/lsp/src/worker.rs#L215) |
| <a id="t0069"></a>69 | queries | 错误构造/上下文 | ` reversed code action range ` | [lsp/src/queries/code_actions.rs:85](../../compiler/lsp/src/queries/code_actions.rs#L85) |
| <a id="t0070"></a>70 | queries | 错误构造/上下文 | ` code action source changed ` | [lsp/src/queries/code_actions.rs:119](../../compiler/lsp/src/queries/code_actions.rs#L119) |
| <a id="t0071"></a>71 | queries | 错误构造/上下文 | ` syntax repair is unavailable ` | [lsp/src/queries/code_actions.rs:129](../../compiler/lsp/src/queries/code_actions.rs#L129) |
| <a id="t0072"></a>72 | queries | 错误构造/上下文 | ` import context changed ` | [lsp/src/queries/code_actions.rs:142](../../compiler/lsp/src/queries/code_actions.rs#L142) |
| <a id="t0073"></a>73 | queries | 错误构造/上下文 | ` import repair is unavailable ` | [lsp/src/queries/code_actions.rs:149](../../compiler/lsp/src/queries/code_actions.rs#L149) |
| <a id="t0074"></a>74 | queries | 错误构造/上下文 | ` client does not support versioned code actions ` | [lsp/src/queries/code_actions.rs:207](../../compiler/lsp/src/queries/code_actions.rs#L207) |
| <a id="t0075"></a>75 | queries | 错误构造/上下文 | ` declaration type is unavailable ` | [lsp/src/queries/hover.rs:234](../../compiler/lsp/src/queries/hover.rs#L234) |
| <a id="t0076"></a>76 | queries | 错误构造/上下文 | ` documentation inputs are unavailable ` | [lsp/src/queries/hover/documentation.rs:32](../../compiler/lsp/src/queries/hover/documentation.rs#L32) |
| <a id="t0077"></a>77 | queries | 错误构造/上下文 | ` implementation analysis is unavailable for this project ` | [lsp/src/queries/implementation.rs:83](../../compiler/lsp/src/queries/implementation.rs#L83) |
| <a id="t0078"></a>78 | queries | 错误构造/上下文 | ` reference analysis is unavailable for this project ` | [lsp/src/queries/references.rs:32](../../compiler/lsp/src/queries/references.rs#L32) |
| <a id="t0079"></a>79 | queries | 错误构造/上下文 | ` rename source location is unavailable ` | [lsp/src/queries/rename.rs:83](../../compiler/lsp/src/queries/rename.rs#L83) |
| <a id="t0080"></a>80 | queries | 错误构造/上下文 | ` client does not support versioned rename edits ` | [lsp/src/queries/rename.rs:112](../../compiler/lsp/src/queries/rename.rs#L112) |
| <a id="t0081"></a>81 | queries | 错误构造/上下文 | ` source is unavailable ` | [lsp/src/queries/semantic.rs:98](../../compiler/lsp/src/queries/semantic.rs#L98)；[lsp/src/queries/workspace_symbols.rs:37](../../compiler/lsp/src/queries/workspace_symbols.rs#L37)；[lsp/src/worker.rs:281](../../compiler/lsp/src/worker.rs#L281) |
| <a id="t0082"></a>82 | queries | 错误构造/上下文 | ` local type is unavailable ` | [lsp/src/queries/semantic.rs:276](../../compiler/lsp/src/queries/semantic.rs#L276) |
| <a id="t0083"></a>83 | queries | 错误构造/上下文 | ` call type is unavailable ` | [lsp/src/queries/signature_help.rs:139](../../compiler/lsp/src/queries/signature_help.rs#L139) |
| <a id="t0084"></a>84 | queries | 错误构造/上下文 | ` source {} is unavailable ` | [lsp/src/queries/workspace_symbols.rs:21](../../compiler/lsp/src/queries/workspace_symbols.rs#L21) |
| <a id="t0085"></a>85 | queries | 错误构造/上下文 | ` source path has no file URI ` | [lsp/src/queries/workspace_symbols.rs:39](../../compiler/lsp/src/queries/workspace_symbols.rs#L39) |
| <a id="t0086"></a>86 | server | 错误构造/上下文 | ` initialize must be the first request ` | [lsp/src/server.rs:148](../../compiler/lsp/src/server.rs#L148) |
| <a id="t0087"></a>87 | server | 错误构造/上下文 | ` server has shut down ` | [lsp/src/server.rs:153](../../compiler/lsp/src/server.rs#L153) |
| <a id="t0088"></a>88 | server | 错误构造/上下文 | ` server is already initialized ` | [lsp/src/server.rs:158](../../compiler/lsp/src/server.rs#L158) |
| <a id="t0089"></a>89 | server | 错误构造/上下文 | ` request id is already active ` | [lsp/src/server.rs:185](../../compiler/lsp/src/server.rs#L185) |
| <a id="t0090"></a>90 | server | 错误构造/上下文 | ` analysis worker stopped ` | [lsp/src/server.rs:202](../../compiler/lsp/src/server.rs#L202) |
| <a id="t0091"></a>91 | server | 错误构造/上下文 | ` document is unavailable ` | [lsp/src/server.rs:218](../../compiler/lsp/src/server.rs#L218) |

## pm（135 条）

| 清单序号 | 领域/实现模块 | 来源形式 | 英文文案模板 | 实现位置 |
| --- | --- | --- | --- | --- |
| <a id="t0092"></a>92 | cache | 错误构造/上下文 | ` output path has no parent ` | [pm/src/cache.rs:71](../../compiler/pm/src/cache.rs#L71) |
| <a id="t0093"></a>93 | cache | 错误构造/上下文 | ` cannot locate the home directory; use --package-cache-dir ` | [pm/src/cache.rs:90](../../compiler/pm/src/cache.rs#L90) |
| <a id="t0094"></a>94 | cache | 错误构造/上下文 | ` input changed while preparing packages: {}; retry the command ` | [pm/src/cache/inputs.rs:35](../../compiler/pm/src/cache/inputs.rs#L35) |
| <a id="t0095"></a>95 | catalog | 错误构造/上下文 | ` Cone.lock does not include {}; resolve without --locked ` | [pm/src/catalog.rs:151](../../compiler/pm/src/catalog.rs#L151) |
| <a id="t0096"></a>96 | catalog | 错误构造/上下文 | ` sources for {} are not cached; run scoop pm fetch ` | [pm/src/catalog.rs:222](../../compiler/pm/src/catalog.rs#L222) |
| <a id="t0097"></a>97 | catalog | 错误构造/上下文 | ` {} requires a different semantic fingerprint of {}; rebuild the upstream artifact from its source ` | [pm/src/catalog.rs:251](../../compiler/pm/src/catalog.rs#L251) |
| <a id="t0098"></a>98 | catalog | 错误构造/上下文 | ` {} is a virtual workspace; select a Cone subdirectory ` | [pm/src/catalog.rs:299](../../compiler/pm/src/catalog.rs#L299) |
| <a id="t0099"></a>99 | catalog | 错误构造/上下文 | ` portable Cone.lock source paths require UTF-8 ` | [pm/src/catalog.rs:350](../../compiler/pm/src/catalog.rs#L350) |
| <a id="t0100"></a>100 | catalog | 错误构造/上下文 | ` artifact at {} declares {}, expected {} ` | [pm/src/catalog/artifacts.rs:155](../../compiler/pm/src/catalog/artifacts.rs#L155) |
| <a id="t0101"></a>101 | catalog | 错误构造/上下文 | ` ambiguous artifacts for {} in search roots ` | [pm/src/catalog/artifacts.rs:168](../../compiler/pm/src/catalog/artifacts.rs#L168) |
| <a id="t0102"></a>102 | catalog | 错误构造/上下文 | ` no artifact found for {} in the selected artifact search roots ` | [pm/src/catalog/artifacts.rs:191](../../compiler/pm/src/catalog/artifacts.rs#L191) |
| <a id="t0103"></a>103 | catalog | 错误构造/上下文 | ` Cone.lock does not record {reference} for {}; resolve without --locked ` | [pm/src/catalog/remote.rs:32](../../compiler/pm/src/catalog/remote.rs#L32) |
| <a id="t0104"></a>104 | catalog | 错误构造/上下文 | ` Git --precise requires a full lowercase commit ID ` | [pm/src/catalog/remote.rs:49](../../compiler/pm/src/catalog/remote.rs#L49) |
| <a id="t0105"></a>105 | catalog | 错误构造/上下文 | ` metadata requires a locked Git commit; run scoop pm resolve ` | [pm/src/catalog/remote.rs:71](../../compiler/pm/src/catalog/remote.rs#L71) |
| <a id="t0106"></a>106 | catalog | 错误构造/上下文 | ` Git checkout {repository}#{commit} is not cached; run scoop pm fetch ` | [pm/src/catalog/remote.rs:75](../../compiler/pm/src/catalog/remote.rs#L75) |
| <a id="t0107"></a>107 | catalog | 错误构造/上下文 | ` Cone.lock does not record registry {registry} for {}; resolve without --locked ` | [pm/src/catalog/remote.rs:107](../../compiler/pm/src/catalog/remote.rs#L107) |
| <a id="t0108"></a>108 | catalog | 错误构造/上下文 | ` registry metadata for {} is not cached; run scoop pm fetch ` | [pm/src/catalog/remote.rs:124](../../compiler/pm/src/catalog/remote.rs#L124) |
| <a id="t0109"></a>109 | catalog | 错误构造/上下文 | ` Git dependency {key} has a path outside its checkout ` | [pm/src/catalog/requirements.rs:95](../../compiler/pm/src/catalog/requirements.rs#L95) |
| <a id="t0110"></a>110 | catalog | 错误构造/上下文 | ` remote packages cannot contain local artifact locators ` | [pm/src/catalog/requirements.rs:104](../../compiler/pm/src/catalog/requirements.rs#L104) |
| <a id="t0111"></a>111 | catalog | 错误构造/上下文 | ` no default registry for {key}; declare registry.default or an explicit registry URL ` | [pm/src/catalog/requirements.rs:110](../../compiler/pm/src/catalog/requirements.rs#L110) |
| <a id="t0112"></a>112 | cli | 错误构造/上下文 | ` invalid package preparation request: {error} ` | [pm/src/cli.rs:109](../../compiler/pm/src/cli.rs#L109) |
| <a id="t0113"></a>113 | cli | 错误构造/上下文 | ` cannot write package preparation response: {error} ` | [pm/src/cli.rs:119](../../compiler/pm/src/cli.rs#L119) |
| <a id="t0114"></a>114 | cli | 错误构造/上下文 | ` refusing to overwrite {} ` | [pm/src/cli.rs:132](../../compiler/pm/src/cli.rs#L132) |
| <a id="t0115"></a>115 | cli | 错误构造/上下文 | ` {} does not exist; resolve dependencies first ` | [pm/src/cli/deps.rs:40](../../compiler/pm/src/cli/deps.rs#L40) |
| <a id="t0116"></a>116 | cli | 错误构造/上下文 | ` no locked resolution for {} / {} / {target}; resolve dependencies first ` | [pm/src/cli/deps.rs:65](../../compiler/pm/src/cli/deps.rs#L65) |
| <a id="t0117"></a>117 | cli | 错误构造/上下文 | ` {key} is not in any selected locked graph ` | [pm/src/cli/deps/why.rs:80](../../compiler/pm/src/cli/deps/why.rs#L80) |
| <a id="t0118"></a>118 | cli | 错误构造/上下文 | ` add/remove requires exactly one selected Cone; use -p group:name ` | [pm/src/cli/edit.rs:99](../../compiler/pm/src/cli/edit.rs#L99) |
| <a id="t0119"></a>119 | cli | 错误构造/上下文 | ` dependency path must not be empty ` | [pm/src/cli/edit.rs:146](../../compiler/pm/src/cli/edit.rs#L146) |
| <a id="t0120"></a>120 | cli | 错误构造/上下文 | ` manifest dependency path must be UTF-8 ` | [pm/src/cli/edit.rs:154](../../compiler/pm/src/cli/edit.rs#L154) |
| <a id="t0121"></a>121 | cli | 错误构造/上下文 | ` invalid manifest overlays: {error} ` | [pm/src/cli/metadata.rs:101](../../compiler/pm/src/cli/metadata.rs#L101) |
| <a id="t0122"></a>122 | cli | 错误构造/上下文 | ` --precise requires exactly one --package ` | [pm/src/cli/resolve.rs:50](../../compiler/pm/src/cli/resolve.rs#L50) |
| <a id="t0123"></a>123 | edit | 错误构造/上下文 | ` {section} must be a table ` | [pm/src/edit.rs:30](../../compiler/pm/src/edit.rs#L30) |
| <a id="t0124"></a>124 | edit | 错误构造/上下文 | ` {key} is not declared in [{section}] of {} ` | [pm/src/edit.rs:62](../../compiler/pm/src/edit.rs#L62) |
| <a id="t0125"></a>125 | error | 错误类型显示 | ` \n  at {} (bytes {}..{}) ` | [pm/src/error.rs:95](../../compiler/pm/src/error.rs#L95) |
| <a id="t0126"></a>126 | lockfile | 错误构造/上下文 | ` purpose must be production or test ` | [pm/src/lockfile.rs:35](../../compiler/pm/src/lockfile.rs#L35) |
| <a id="t0127"></a>127 | lockfile | 错误构造/上下文 | ` lockfile has no parent directory ` | [pm/src/lockfile/store.rs:51](../../compiler/pm/src/lockfile/store.rs#L51) |
| <a id="t0128"></a>128 | lockfile | 错误构造/上下文 | ` lockfile has no filename ` | [pm/src/lockfile/store.rs:57](../../compiler/pm/src/lockfile/store.rs#L57) |
| <a id="t0129"></a>129 | lockfile | 错误构造/上下文 | ` unsupported Cone.lock format {} ` | [pm/src/lockfile/validate.rs:12](../../compiler/pm/src/lockfile/validate.rs#L12) |
| <a id="t0130"></a>130 | lockfile | 错误构造/上下文 | ` duplicate lock resolution for {} / {} / {} ` | [pm/src/lockfile/validate.rs:20](../../compiler/pm/src/lockfile/validate.rs#L20) |
| <a id="t0131"></a>131 | lockfile | 错误构造/上下文 | ` locked root must be a normalized workspace-relative path ` | [pm/src/lockfile/validate.rs:38](../../compiler/pm/src/lockfile/validate.rs#L38) |
| <a id="t0132"></a>132 | lockfile | 错误构造/上下文 | ` unknown lock target {:?} ` | [pm/src/lockfile/validate.rs:46](../../compiler/pm/src/lockfile/validate.rs#L46) |
| <a id="t0133"></a>133 | lockfile | 错误构造/上下文 | ` lock resolution has more than one node for {key} ` | [pm/src/lockfile/validate.rs:56](../../compiler/pm/src/lockfile/validate.rs#L56) |
| <a id="t0134"></a>134 | lockfile | 错误构造/上下文 | ` locked dependency {} must be a library ` | [pm/src/lockfile/validate.rs:61](../../compiler/pm/src/lockfile/validate.rs#L61) |
| <a id="t0135"></a>135 | lockfile | 错误构造/上下文 | ` test dependencies are only allowed on a Test root: {} ` | [pm/src/lockfile/validate.rs:69](../../compiler/pm/src/lockfile/validate.rs#L69) |
| <a id="t0136"></a>136 | lockfile | 错误构造/上下文 | ` lock resolution is missing root {root} ` | [pm/src/lockfile/validate.rs:79](../../compiler/pm/src/lockfile/validate.rs#L79) |
| <a id="t0137"></a>137 | lockfile | 错误构造/上下文 | ` {} has missing or duplicate locked dependency {edge} ` | [pm/src/lockfile/validate.rs:88](../../compiler/pm/src/lockfile/validate.rs#L88) |
| <a id="t0138"></a>138 | lockfile | 错误构造/上下文 | ` {} is missing the implicit core edge ` | [pm/src/lockfile/validate.rs:96](../../compiler/pm/src/lockfile/validate.rs#L96) |
| <a id="t0139"></a>139 | lockfile | 错误构造/上下文 | ` cycle in locked dependency graph at {coordinate} ` | [pm/src/lockfile/validate.rs:112](../../compiler/pm/src/lockfile/validate.rs#L112) |
| <a id="t0140"></a>140 | lockfile | 错误构造/上下文 | ` lock resolution contains unreachable nodes ` | [pm/src/lockfile/validate.rs:127](../../compiler/pm/src/lockfile/validate.rs#L127) |
| <a id="t0141"></a>141 | lockfile | 错误构造/上下文 | ` invalid locked source for {coordinate}: {message} ` | [pm/src/lockfile/validate.rs:149](../../compiler/pm/src/lockfile/validate.rs#L149) |
| <a id="t0142"></a>142 | lockfile | 错误构造/上下文 | ` empty path or NUL in path ` | [pm/src/lockfile/validate.rs:154](../../compiler/pm/src/lockfile/validate.rs#L154) |
| <a id="t0143"></a>143 | lockfile | 错误构造/上下文 | ` Git repository and full hexadecimal commit are required ` | [pm/src/lockfile/validate.rs:171](../../compiler/pm/src/lockfile/validate.rs#L171) |
| <a id="t0144"></a>144 | lockfile | 错误构造/上下文 | ` Git subdir must be normalized ` | [pm/src/lockfile/validate.rs:179](../../compiler/pm/src/lockfile/validate.rs#L179) |
| <a id="t0145"></a>145 | lockfile | 错误构造/上下文 | ` Git references must contain distinct declared selectors ` | [pm/src/lockfile/validate.rs:191](../../compiler/pm/src/lockfile/validate.rs#L191) |
| <a id="t0146"></a>146 | lockfile | 错误构造/上下文 | ` registry archive and lowercase SHA-256 are required ` | [pm/src/lockfile/validate.rs:208](../../compiler/pm/src/lockfile/validate.rs#L208) |
| <a id="t0147"></a>147 | lockfile | 错误构造/上下文 | ` archive must be an HTTP(S) URL without embedded credentials or fragment ` | [pm/src/lockfile/validate.rs:222](../../compiler/pm/src/lockfile/validate.rs#L222)；[pm/src/sources/registry.rs:101](../../compiler/pm/src/sources/registry.rs#L101) |
| <a id="t0148"></a>148 | lockfile | 错误构造/上下文 | ` sysroot name must match a scoop group coordinate ` | [pm/src/lockfile/validate.rs:228](../../compiler/pm/src/lockfile/validate.rs#L228) |
| <a id="t0149"></a>149 | main | 错误构造/上下文 | ` error: {error}\ncannot encode diagnostic: {encoding} ` | [pm/src/main.rs:15](../../compiler/pm/src/main.rs#L15) |
| <a id="t0150"></a>150 | prepare | 错误构造/上下文 | ` input changed while preparing packages; retry the command ` | [pm/src/prepare/record.rs:104](../../compiler/pm/src/prepare/record.rs#L104) |
| <a id="t0151"></a>151 | resolve | 错误构造/上下文 | ` update cannot be combined with --locked/--frozen ` | [pm/src/resolve.rs:84](../../compiler/pm/src/resolve.rs#L84) |
| <a id="t0152"></a>152 | resolve | 错误构造/上下文 | ` no workspace members selected ` | [pm/src/resolve.rs:107](../../compiler/pm/src/resolve.rs#L107) |
| <a id="t0153"></a>153 | resolve | 错误构造/上下文 | ` no locked resolution for {} / {} / {target_name}; run scoop pm resolve ` | [pm/src/resolve.rs:144](../../compiler/pm/src/resolve.rs#L144) |
| <a id="t0154"></a>154 | resolve | 错误构造/上下文 | ` \nCone.lock cannot satisfy the current requirements; run scoop pm resolve ` | [pm/src/resolve.rs:162](../../compiler/pm/src/resolve.rs#L162) |
| <a id="t0155"></a>155 | resolve | 错误构造/上下文 | ` \n--offline used only local metadata and Git objects; remote versions were not checked ` | [pm/src/resolve.rs:165](../../compiler/pm/src/resolve.rs#L165) |
| <a id="t0156"></a>156 | resolve | 错误构造/上下文 | ` Cone.lock no longer matches {}; run scoop pm resolve without --locked ` | [pm/src/resolve.rs:176](../../compiler/pm/src/resolve.rs#L176) |
| <a id="t0157"></a>157 | resolve | 错误构造/上下文 | ` package {key} is not in any selected dependency graph ` | [pm/src/resolve.rs:206](../../compiler/pm/src/resolve.rs#L206) |
| <a id="t0158"></a>158 | resolve | 错误构造/上下文 | ` input appeared while preparing packages: {}; retry the command ` | [pm/src/resolve.rs:216](../../compiler/pm/src/resolve.rs#L216) |
| <a id="t0159"></a>159 | resolver | 错误构造/上下文 | ` expected {} from {}, source declares {} ` | [pm/src/resolver/provider.rs:76](../../compiler/pm/src/resolver/provider.rs#L76) |
| <a id="t0160"></a>160 | resolver | 错误构造/上下文 | ` dependency resolution failed:\n{} ` | [pm/src/resolver/report.rs:19](../../compiler/pm/src/resolver/report.rs#L19) |
| <a id="t0161"></a>161 | resolver | 错误构造/上下文 | ` \n  {path}: {} requires {version} from {} ` | [pm/src/resolver/report.rs:51](../../compiler/pm/src/resolver/report.rs#L51) |
| <a id="t0162"></a>162 | resolver | 错误构造/上下文 | ` \n    {} (bytes {}..{}) ` | [pm/src/resolver/report.rs:56](../../compiler/pm/src/resolver/report.rs#L56) |
| <a id="t0163"></a>163 | resolver | 错误构造/上下文 | ` {kind}; dependencies must be libraries ` | [pm/src/resolver/report.rs:73](../../compiler/pm/src/resolver/report.rs#L73) |
| <a id="t0164"></a>164 | resolver | 错误构造/上下文 | ` does not satisfy {version} ` | [pm/src/resolver/report.rs:77](../../compiler/pm/src/resolver/report.rs#L77) |
| <a id="t0165"></a>165 | resolver | 错误构造/上下文 | ` excluded by lock, update, or yank policy ` | [pm/src/resolver/report.rs:81](../../compiler/pm/src/resolver/report.rs#L81) |
| <a id="t0166"></a>166 | resolver | 错误构造/上下文 | ` other version or source constraints conflict ` | [pm/src/resolver/report.rs:84](../../compiler/pm/src/resolver/report.rs#L84) |
| <a id="t0167"></a>167 | resolver | 错误构造/上下文 | ` \n    candidate {choice}: {} ` | [pm/src/resolver/report.rs:87](../../compiler/pm/src/resolver/report.rs#L87) |
| <a id="t0168"></a>168 | sources | 错误构造/上下文 | ` Git objects for {repository} are not cached; fetch without --offline ` | [pm/src/sources/git.rs:76](../../compiler/pm/src/sources/git.rs#L76) |
| <a id="t0169"></a>169 | sources | 错误构造/上下文 | ` Git commit {commit} for {repository} is not cached; fetch without --offline ` | [pm/src/sources/git.rs:102](../../compiler/pm/src/sources/git.rs#L102) |
| <a id="t0170"></a>170 | sources | 错误构造/上下文 | ` Cone.lock must contain the full Git commit ID ` | [pm/src/sources/git.rs:113](../../compiler/pm/src/sources/git.rs#L113) |
| <a id="t0171"></a>171 | sources | 错误构造/上下文 | ` checkout {} needs uncached submodules; fetch without --offline ` | [pm/src/sources/git.rs:148](../../compiler/pm/src/sources/git.rs#L148) |
| <a id="t0172"></a>172 | sources | 错误构造/上下文 | ` invalid Git repository URL: {error} ` | [pm/src/sources/git.rs:192](../../compiler/pm/src/sources/git.rs#L192) |
| <a id="t0173"></a>173 | sources | 错误构造/上下文 | ` cannot represent Git repository {} as a file URL ` | [pm/src/sources/git.rs:203](../../compiler/pm/src/sources/git.rs#L203) |
| <a id="t0174"></a>174 | sources | 错误构造/上下文 | ` locked Git commit must be a full lowercase hexadecimal object ID ` | [pm/src/sources/git.rs:216](../../compiler/pm/src/sources/git.rs#L216) |
| <a id="t0175"></a>175 | sources | 错误构造/上下文 | ` cannot run Git: {error} ` | [pm/src/sources/git/command.rs:6](../../compiler/pm/src/sources/git/command.rs#L6) |
| <a id="t0176"></a>176 | sources | 错误构造/上下文 | ` Git exited with {}: {} ` | [pm/src/sources/git/command.rs:9](../../compiler/pm/src/sources/git/command.rs#L9) |
| <a id="t0177"></a>177 | sources | 错误构造/上下文 | ` Git returned invalid UTF-8: {error} ` | [pm/src/sources/git/command.rs:15](../../compiler/pm/src/sources/git/command.rs#L15) |
| <a id="t0178"></a>178 | sources | 错误构造/上下文 | ` cannot inspect Git object {commit}: {error} ` | [pm/src/sources/git/command.rs:36](../../compiler/pm/src/sources/git/command.rs#L36) |
| <a id="t0179"></a>179 | sources | 错误构造/上下文 | ` Git remote has no default HEAD commit ` | [pm/src/sources/git/command.rs:63](../../compiler/pm/src/sources/git/command.rs#L63) |
| <a id="t0180"></a>180 | sources | 错误构造/上下文 | ` invalid registry archive URL: {error} ` | [pm/src/sources/registry.rs:93](../../compiler/pm/src/sources/registry.rs#L93) |
| <a id="t0181"></a>181 | sources | 错误构造/上下文 | ` archive path escapes the package: {} ` | [pm/src/sources/registry/archive.rs:29](../../compiler/pm/src/sources/registry/archive.rs#L29) |
| <a id="t0182"></a>182 | sources | 错误构造/上下文 | ` unsupported archive entry type: {} ` | [pm/src/sources/registry/archive.rs:43](../../compiler/pm/src/sources/registry/archive.rs#L43) |
| <a id="t0183"></a>183 | sources | 错误构造/上下文 | ` duplicate or empty archive path: {} ` | [pm/src/sources/registry/archive.rs:52](../../compiler/pm/src/sources/registry/archive.rs#L52) |
| <a id="t0184"></a>184 | sources | 错误构造/上下文 | ` GET {url}: {error} ` | [pm/src/sources/registry/archive.rs:90](../../compiler/pm/src/sources/registry/archive.rs#L90)；[pm/src/sources/registry/http.rs:23](../../compiler/pm/src/sources/registry/http.rs#L23) |
| <a id="t0185"></a>185 | sources | 错误构造/上下文 | ` SHA-256 mismatch downloading {url} ` | [pm/src/sources/registry/archive.rs:100](../../compiler/pm/src/sources/registry/archive.rs#L100) |
| <a id="t0186"></a>186 | sources | 错误构造/上下文 | ` missing package record {} ` | [pm/src/sources/registry/fetch.rs:51](../../compiler/pm/src/sources/registry/fetch.rs#L51) |
| <a id="t0187"></a>187 | sources | 错误构造/上下文 | ` locked package {coordinate} is missing from {registry} ` | [pm/src/sources/registry/fetch.rs:108](../../compiler/pm/src/sources/registry/fetch.rs#L108) |
| <a id="t0188"></a>188 | sources | 错误构造/上下文 | ` archive for {coordinate} is not cached; fetch without --offline ` | [pm/src/sources/registry/fetch.rs:152](../../compiler/pm/src/sources/registry/fetch.rs#L152) |
| <a id="t0189"></a>189 | sources | 错误构造/上下文 | ` archive manifest does not match registry metadata for {coordinate} ` | [pm/src/sources/registry/fetch.rs:181](../../compiler/pm/src/sources/registry/fetch.rs#L181) |
| <a id="t0190"></a>190 | sources | 错误构造/上下文 | ` immutable registry version metadata changed for {}; cannot overwrite the cached package ` | [pm/src/sources/registry/fetch.rs:216](../../compiler/pm/src/sources/registry/fetch.rs#L216) |
| <a id="t0191"></a>191 | sources | 错误构造/上下文 | ` registry index for {key} at {registry} is not cached; resolve without --offline ` | [pm/src/sources/registry/index.rs:34](../../compiler/pm/src/sources/registry/index.rs#L34) |
| <a id="t0192"></a>192 | sources | 错误构造/上下文 | ` unsupported registry format {} ` | [pm/src/sources/registry/index.rs:48](../../compiler/pm/src/sources/registry/index.rs#L48)；[pm/src/sources/registry/index.rs:58](../../compiler/pm/src/sources/registry/index.rs#L58) |
| <a id="t0193"></a>193 | sources | 错误构造/上下文 | ` invalid registry index identity or format for {key} at {registry} ` | [pm/src/sources/registry/index.rs:79](../../compiler/pm/src/sources/registry/index.rs#L79) |
| <a id="t0194"></a>194 | sources | 错误构造/上下文 | ` duplicate registry version {coordinate} ` | [pm/src/sources/registry/index.rs:88](../../compiler/pm/src/sources/registry/index.rs#L88) |
| <a id="t0195"></a>195 | sources | 错误构造/上下文 | ` invalid registry version or SHA-256 for {coordinate} ` | [pm/src/sources/registry/index.rs:108](../../compiler/pm/src/sources/registry/index.rs#L108) |
| <a id="t0196"></a>196 | sources | 错误构造/上下文 | ` duplicate registry dependency {} in {coordinate} ` | [pm/src/sources/registry/index.rs:115](../../compiler/pm/src/sources/registry/index.rs#L115) |
| <a id="t0197"></a>197 | sources | 错误构造/上下文 | ` sysroot dependency must use the scoop group ` | [pm/src/sources/registry/index.rs:121](../../compiler/pm/src/sources/registry/index.rs#L121) |
| <a id="t0198"></a>198 | sources | 错误构造/上下文 | ` registry Git dependency requires repository and full commit ` | [pm/src/sources/registry/index.rs:135](../../compiler/pm/src/sources/registry/index.rs#L135) |
| <a id="t0199"></a>199 | sources | 错误构造/上下文 | ` registry metadata for {coordinate} is missing the exact implicit core edge ` | [pm/src/sources/registry/index.rs:154](../../compiler/pm/src/sources/registry/index.rs#L154) |
| <a id="t0200"></a>200 | sources | 错误构造/上下文 | ` invalid packaged Cone.toml: {error} ` | [pm/src/sources/registry/manifest.rs:18](../../compiler/pm/src/sources/registry/manifest.rs#L18) |
| <a id="t0201"></a>201 | sources | 错误构造/上下文 | ` published manifest must not contain workspace, overrides, or publish configuration ` | [pm/src/sources/registry/manifest.rs:24](../../compiler/pm/src/sources/registry/manifest.rs#L24) |
| <a id="t0202"></a>202 | sources | 错误构造/上下文 | ` published manifest must define a Cone ` | [pm/src/sources/registry/manifest.rs:30](../../compiler/pm/src/sources/registry/manifest.rs#L30) |
| <a id="t0203"></a>203 | sources | 错误构造/上下文 | ` published manifest must expand workspace inheritance ` | [pm/src/sources/registry/manifest.rs:34](../../compiler/pm/src/sources/registry/manifest.rs#L34) |
| <a id="t0204"></a>204 | sources | 错误构造/上下文 | ` invalid reserved package coordinate or core kind ` | [pm/src/sources/registry/manifest.rs:48](../../compiler/pm/src/sources/registry/manifest.rs#L48) |
| <a id="t0205"></a>205 | sources | 错误构造/上下文 | ` explicit core requirement does not accept scoop:scoop.core:0.1.0 ` | [pm/src/sources/registry/manifest.rs:76](../../compiler/pm/src/sources/registry/manifest.rs#L76) |
| <a id="t0206"></a>206 | sources | 错误构造/上下文 | ` published dependency {key} must expand workspace inheritance ` | [pm/src/sources/registry/manifest.rs:102](../../compiler/pm/src/sources/registry/manifest.rs#L102) |
| <a id="t0207"></a>207 | sources | 错误构造/上下文 | ` published dependency {key} requires a version ` | [pm/src/sources/registry/manifest.rs:108](../../compiler/pm/src/sources/registry/manifest.rs#L108) |
| <a id="t0208"></a>208 | sources | 错误构造/上下文 | ` published Git dependency {key} must select a full commit ` | [pm/src/sources/registry/manifest.rs:117](../../compiler/pm/src/sources/registry/manifest.rs#L117)；[pm/src/sources/registry/manifest.rs:126](../../compiler/pm/src/sources/registry/manifest.rs#L126) |
| <a id="t0209"></a>209 | sources | 错误构造/上下文 | ` published dependency {key} needs an explicit registry, pinned Git, or sysroot source ` | [pm/src/sources/registry/manifest.rs:137](../../compiler/pm/src/sources/registry/manifest.rs#L137) |
| <a id="t0210"></a>210 | workspace | 错误构造/上下文 | ` {key} is not a member of {} ` | [pm/src/workspace.rs:173](../../compiler/pm/src/workspace.rs#L173) |
| <a id="t0211"></a>211 | workspace | 错误构造/上下文 | ` a nested workspace cannot be an outer workspace member ` | [pm/src/workspace/discover.rs:28](../../compiler/pm/src/workspace/discover.rs#L28) |
| <a id="t0212"></a>212 | workspace | 错误构造/上下文 | ` duplicate workspace member ` | [pm/src/workspace/discover.rs:36](../../compiler/pm/src/workspace/discover.rs#L36)；[protocol/src/package/members.rs:38](../../compiler/protocol/src/package/members.rs#L38) |
| <a id="t0213"></a>213 | workspace | 错误构造/上下文 | ` duplicate workspace coordinate {key}: {} and {} ` | [pm/src/workspace/discover.rs:49](../../compiler/pm/src/workspace/discover.rs#L49) |
| <a id="t0214"></a>214 | workspace | 错误构造/上下文 | ` default member {:?} is not a workspace member ` | [pm/src/workspace/discover.rs:81](../../compiler/pm/src/workspace/discover.rs#L81) |
| <a id="t0215"></a>215 | workspace | 错误构造/上下文 | ` duplicate default workspace member ` | [pm/src/workspace/discover.rs:87](../../compiler/pm/src/workspace/discover.rs#L87) |
| <a id="t0216"></a>216 | workspace | 错误构造/上下文 | ` input must be a Cone directory or Cone.toml ` | [pm/src/workspace/discover.rs:177](../../compiler/pm/src/workspace/discover.rs#L177) |
| <a id="t0217"></a>217 | workspace | 错误构造/上下文 | ` no Cone.toml found from {} or its parents ` | [pm/src/workspace/discover.rs:187](../../compiler/pm/src/workspace/discover.rs#L187) |
| <a id="t0218"></a>218 | workspace | 错误构造/上下文 | ` {} does not define a Cone ` | [pm/src/workspace/expand.rs:12](../../compiler/pm/src/workspace/expand.rs#L12) |
| <a id="t0219"></a>219 | workspace | 错误构造/上下文 | ` user manifest cannot declare the reserved single-file coordinate ` | [pm/src/workspace/expand.rs:40](../../compiler/pm/src/workspace/expand.rs#L40) |
| <a id="t0220"></a>220 | workspace | 错误构造/上下文 | ` core must be a library ` | [pm/src/workspace/expand.rs:48](../../compiler/pm/src/workspace/expand.rs#L48) |
| <a id="t0221"></a>221 | workspace | 错误构造/上下文 | ` cannot inherit {field}: Cone is not a workspace member ` | [pm/src/workspace/expand.rs:95](../../compiler/pm/src/workspace/expand.rs#L95)；[pm/src/workspace/expand.rs:126](../../compiler/pm/src/workspace/expand.rs#L126) |
| <a id="t0222"></a>222 | workspace | 错误构造/上下文 | ` workspace {} does not define cone.{field} ` | [pm/src/workspace/expand.rs:101](../../compiler/pm/src/workspace/expand.rs#L101) |
| <a id="t0223"></a>223 | workspace | 错误构造/上下文 | ` workspace {} does not define dependency {key} ` | [pm/src/workspace/expand.rs:132](../../compiler/pm/src/workspace/expand.rs#L132) |
| <a id="t0224"></a>224 | workspace | 错误构造/上下文 | ` duplicate manifest overlay path ` | [pm/src/workspace/overlays.rs:26](../../compiler/pm/src/workspace/overlays.rs#L26)；[protocol/src/package/overlays.rs:67](../../compiler/protocol/src/package/overlays.rs#L67) |
| <a id="t0225"></a>225 | workspace | 错误构造/上下文 | ` manifest has no parent ` | [pm/src/workspace/overlays.rs:59](../../compiler/pm/src/workspace/overlays.rs#L59) |
| <a id="t0226"></a>226 | workspace | 错误构造/上下文 | ` manifest has no name ` | [pm/src/workspace/overlays.rs:62](../../compiler/pm/src/workspace/overlays.rs#L62) |
