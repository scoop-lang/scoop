# HashMap 分组探测设计与实现

2026-11-03。前期基线、局部控制字探针和 Hash 分布见[基础容器调研](CORE-CONTAINERS-RESEARCH.md#4-hashmap-与-swiss-table)。本文记录 `scoop.std` 内部 `HashMap`／`HashSet` 的存储与探测改造设计，以及同日完成的首版实现和性能对照。

## 1. 结论

`HashMap` 改为八槽一组的 Swiss table 式开放寻址：一个 `ULong` 保存八个 control byte，普通整数 SWAR 一次筛选整组 H2 指纹，entry 继续平坦存放并缓存完整的混合后 hash。查找同时返回命中槽或本次可提交的空槽，消除未命中插入的第二次探测。容量保持二次幂组数，组间使用三角步长探测。

首版保持当前 75% 的 `occupied` 上限，保留完整 hash 缓存、全表扩容和现有修改检测语义。首版不引入 SIMD、runtime 入口、编译器 intrinsic、Go 式多表目录、Robin Hood entry 搬移、F14 overflow counter、随机 hash seed 或新的公开 API。87.5% 负载率和其他删除策略只有在完整实现对照后才分别决定。

该设计保持[标准库规范](../specs/SCOOP-STDLIB-SPEC.md#23-无序插入顺序与排序顺序)已有合同：查找、插入和移除平均 O(1)，碰撞最坏 O(n)，迭代顺序不承诺；存在等价 key 时替换 value 但保留最早存入的 key 对象。

## 2. 目标与非目标

### 2.1 目标

- 将状态元数据从每槽一个 `Int` 的 4 B 降为每槽一个 control byte。
- 一次检查八个槽的 H2，只对候选槽读取 entry 和执行 key equality。
- 未命中插入在没有 rehash 时复用查找得到的 vacant 槽。
- 保持 rehash 不调用用户 `hash`／`equals`，也不重新计算用户 hash。
- 保持用户回调抛出、回调重入修改、原 key 身份、迭代器失效及 live view 语义。
- 保持值类型内联、`MaybeUninit`、typed GC scan、写屏障和 moving GC 合同。
- 对恒定 hash 等合法但低质量的输入保持有界终止；允许按规范退化为 O(n)，不因探测距离溢出而拒绝合法 key。

### 2.2 非目标

- 不把 `HashMap` 移到 C／Rust runtime，也不让原生容器持有 managed ref。
- 不增加语言可见的向量类型、SSE2／NEON API 或 HashMap 专用 intrinsic。
- 不改变 `Hash.hash()` 的公开结果，不缓存 `String` hash，不增加 hash 能力标记。
- 不提供 capacity、reserve、load factor、iterator.remove 或稳定 entry 地址 API。
- 不改变 `LinkedHashMap` 的插入顺序，也不把普通 `HashMap` 改为密集 value vector。
- 不为大表增加目录、分片、增量 rehash 或延迟迁移；扩容尾延迟留待真实负载独立评估。
- 不在生产实现加入探测计数器、性能开关或多套长期并存的表实现。

## 3. 当前基线与必须保留的行为

改造前的 [HashStorage](../../sysroot/src/scoop.std/src/collections/hash_storage.scoop) 使用线性开放寻址，`states` 的 0／1／2 分别表示 empty／full／deleted。`HashEntry<K,V>` 平坦保存 key、value 和完整 `Long hash`；`occupied` 统计 full 与 deleted，在 75% 时同容量清理或倍增。

现有实现已经具有以下正确设计，改造时保留：

- entry 不是逐项分配的链表节点，K／V 和缓存 hash 内联在数组槽中。
- rehash 搬运缓存 hash，不回调用户 `hash()`。
- 替换 value 写回原 key，保持最早 key 的对象身份。
- 用户回调完成后用 `ModificationCounter` 检查同一集合是否已被重入修改。
- 查找完成后到提交之间不再调用用户方法。
- 删除把整个 `MaybeUninit<HashEntry<K,V>>` 写回 uninit，不能只改状态而继续保活 K／V 引用。
- value 替换只推进 `changes`；插入、移除和非空 clear 同时推进 `changes` 与 `structure`。
- iterator 在永久 exhausted 后不再因后续结构修改恢复或报错。

改造前的明确成本为：4 B/槽状态、逐槽控制流、动态余数定位、tombstone 拉长未命中，以及未命中插入在 `locate` 后由 `vacant` 从头再次探测。M62 已经单独验证“只把 `% capacity` 改为 mask”没有稳定收益；容量 mask 必须与分组布局和单次探测一起实施，不能作为独立收益宣称。

## 4. 方案选择

### 4.1 选择八槽 control word

首版以一个普通 `ULong` 表示八个 control byte，而不是 `MutableArray<UInt8>` 或平台向量：

- 全部现有 64 位目标都能高效执行 64 位 xor、减法、取反、乘法和位移。
- control 读取只有一次数组访问和一次边界检查，不逐槽读取小数组。
- 数值位移明确规定槽与逻辑字节的关系，不依赖目标机器端序或裸指针重解释。
- 已有普通 Scoop 探针中，ULong SWAR 的局部分组筛选明显快于八次小数组循环；完整收益仍由本次真实实现决定。
- 八槽与初始容量 8 一致，不需要首版处理跨 control word 的未对齐组读取或额外 clone 尾部。

Abseil 的 16 槽、F14 的 14 槽以及目标 SIMD 属于后续机器级优化。首版先证明完整查找、删除、rehash、回调和 GC 路径，不以新增通用 SIMD 体系作为 HashMap 的前置条件。

### 4.2 不选择 Robin Hood 作为默认布局

Robin Hood 可以降低探测距离方差，并以 backward-shift deletion 消除 tombstone，但插入和删除都可能搬动多个完整 entry。Scoop 的 K／V 可以是大值类型、含 managed ref 的聚合或双字 interface；逐槽交换会放大复制与写屏障成本。旧 `robin_hood::unordered_map` 还允许低质量 hash 导致探测距离溢出失败，这不符合 Scoop 已规定的最坏 O(n) 行为。

本设计只在 rehash 时批量搬运 entry；普通插入写一个 vacant 槽，普通删除清一个槽。若完整 Swiss 实现仍在特定 churn 负载下受 tombstone 清理主导，再独立比较 backward shift 或 overflow counter，不先为假想负载增加平行布局。

### 4.3 不采用 Go 的多表目录

Go 在 Swiss table 外增加多表目录，以限制单次扩容迁移量，并处理迭代中允许增删的语言语义。Scoop 的结构修改直接使既有 iterator 失效，不需要保留旧表决定迭代次序或在新表复查 value。当前标准库也没有单次插入延迟上界合同。

因此首版继续一次构造新表并原子替换 `storage`。只有真实大表负载显示扩容尾延迟是主要问题时，才单独设计增量 rehash；不能把 Go 的目录复杂度无条件带入普通集合。

## 5. 数据布局与不变量

建议的内部表示为：

```text
HashMap<K, V>
  storage: HashStorage<K, V>
  elementCount: Long
  occupied: Long
  changes: ModificationCounter
  structure: ModificationCounter

HashStorage<K, V>
  controls: MutableArray<ULong>
  elements: MutableArray<MaybeUninit<HashEntry<K, V>>>

HashEntry<K, V>
  key: K
  value: V
  mixedHash: ULong
```

固定不变量：

1. 每组恰有 8 个 entry 槽和一个 control word。
2. `groupCount == controls.size`，`capacity == groupCount * 8`。
3. `groupCount` 始终是正的二次幂；初始为 1，容量初始为 8。
4. `elements.size == capacity`，槽 `group * 8 + offset` 对应 control word 的逻辑 byte `offset`。
5. `elementCount` 等于 full 槽数；`occupied` 等于 full 加 deleted 槽数。
6. empty 和 deleted 槽的 `elements` 必须为 uninit；full 槽必须为完整有效的 `HashEntry`。
7. `elementCount <= occupied <= maximumOccupied < capacity`，表中始终存在 empty，探测一定终止。
8. full control byte 精确等于该 entry 的 H2；entry 的 `mixedHash` 在存储期间不变。
9. 控制字只用于集合算法，不参与 GC 扫描；GC 继续按 typed `MaybeUninit<HashEntry<K,V>>` 数组扫描实际引用布局。

### 5.1 Control byte 编码

首版使用与常见 Swiss table 相同的高位区分：

| 状态 | control byte |
| --- | ---: |
| full | `0x00..0x7f`，值为 H2 |
| empty | `0x80` |
| deleted | `0xfe` |

空组常量为 `0x8080808080808080uL`。槽 0 使用 ULong 的低 8 位，槽 7 使用高 8 位；所有读写通过普通位移和 mask 完成，因此不随机器端序变化。

首版不为利用分配器清零而另造 zero-is-empty 编码。初始化 controls 只需按组写入，次数是 capacity 的八分之一；采用非标准编码会同时影响 H2 空间、状态判断和 SWAR 证明，收益必须先有实际证据。

### 5.2 容量与增长

初始 capacity 为 8。首版保留当前阈值：

```text
maximumOccupied = capacity - capacity / 4
```

也就是 75% occupied，而不是直接采用 7/8。这样首轮对照只改变探测与布局，不同时改变空间／探测取舍。

增长只把 groupCount 和 capacity 倍增。倍增前使用 checked arithmetic，不能在极端分支改用 `Long.MAX_VALUE`，因为非二次幂容量会破坏 group mask 与探测遍历不变量。实际不可分配的容量沿现有分配失败路径结束，不增加容器专用配额或恢复协议。

首版不缩容。same-capacity rehash 只清理 deleted；growth rehash 同时扩大容量。

## 6. Hash 派生

### 6.1 用户 hash 只调用一次

一次公开操作先调用 `key.hash()`，返回后立即检查 `changes`。将 Long 的 64 位位型转换为 ULong 后，执行容器私有 avalanche mixer，结果称为 `mixedHash`。entry 缓存 `mixedHash`，而不是同时保存 raw hash 和 mixed hash。

私有 mixer 复用已有 word hash 的可逆 64 位混合形状：xor-shift、两个奇数乘法、xor-shift。它不改变 `Hash.hash()` 的公开返回值，只决定本表内部布局。该映射为固定、确定性、无回调的普通整数运算；相等 key 的 raw hash 相同，mixed hash 也相同。

始终执行私有 mixing，不新增“已 avalanche”接口或 primitive 特例。原因是现有 `Hash` 合同只要求相等值具有相同 hash，不保证任一位段均匀。若直接把某七位作为 H2、另一段作为二次幂组索引，连续整数、FNV 字符串和普通用户组合 hash 都可能聚集。已有样本也确认短 String 的原始高七位分布不足。

双重 mixing 对已经均匀的 Long hash 有固定成本，必须计入完整查找对照；不能以此为由删除对普通用户 hash 必需的分布修复。若它成为主要成本，应先比较更小但通过分布测试的统一 mixer，不增加类型特化或公开 hash 品质状态。

### 6.2 H1 与 H2

```text
H2 = mixedHash & 0x7f
H1 = mixedHash >> 7
startGroup = H1 & (groupCount - 1)
```

H2 保存在 control byte 中；H1 决定首组。完整 `mixedHash` 仍在读取 key 前比较，以进一步减少 equality 回调。恒定 hash 无法由 mixer 修复，仍可退化为全表探测，但访问组数有显式上界，不失败或无限循环。

## 7. 分组筛选与探测

### 7.1 SWAR byte match

对 control word 与重复八次的目标 byte 做 xor，再使用 zero-byte 检测公式形成候选 mask：

```text
compared = control xor repeatedByte
matches = (compared - 0x0101010101010101) and
          compared.inv() and
          0x8080808080808080
```

mask 的每个置位位于对应逻辑 byte 的最高位。该技巧可能在相邻 byte 上产生额外候选，但不能漏掉真实匹配。因此：

- H2 候选在读取 entry 前必须再次精确检查 control byte 等于 H2；不能对 false positive 或特殊状态调用 `assumeInit()`。
- empty／deleted 选择也必须确认精确 control byte，不能把候选 mask 直接当成槽号。
- false positive 只允许增加少量普通整数检查，不能改变命中、终止或 vacant 选择。

候选位枚举首版使用普通 ULong 操作。最低候选 byte 可用 32／16／8 位三段判断取得，随后以 `mask & (mask - 1)` 清除最低置位。不为此预先新增 `countTrailingZeroBits` intrinsic；只有实际机器码和 profile 显示枚举成为热点时再按 spec 先行单独设计。

### 7.2 组间序列

组间使用三角步长：

```text
group = startGroup
step = 0

repeat at most groupCount times:
    inspect group
    step += 1
    group = (group + step) & groupMask
```

二次幂 groupCount 下，该序列在回到起点前覆盖全部组。实现仍显式限制最多访问 `groupCount` 组；容量或 control 损坏不能让普通查找形成无界循环。生产数据由封闭的 `HashStorage` 写入，不增加第二套重型验证器。

## 8. 操作算法

### 8.1 有类型的查找结果

内部查找返回命中或缺失两种结果，而不是以无效 index 混用：

```text
HashProbe
  Found(index, mixedHash)
  Absent(vacantIndex, mixedHash)
```

`vacantIndex` 精确指向本次探测遇到的第一个 deleted，若此前没有 deleted，则指向终止组中的第一个 empty。结果只在本次同步集合操作中消费，不保存到对象、不跨用户回调，也不增加 generation、token 或资格状态。

`LinkedHashMap` 当前依赖 `HashMap` 的内部 locate／slot 接口，应同步改为消费 `HashProbe`；不保留新旧两套内部 API。

### 8.2 查找

查找步骤：

1. 取得 `changes` stamp。
2. 调用一次 `key.hash()`；返回后检查 stamp。
3. 计算 mixedHash、H1、H2 和首组。
4. 每组先枚举 H2 候选：
   - 精确确认 control byte 为 H2；
   - 读取已初始化 entry；
   - 比较 entry.mixedHash；
   - 完整 hash 相同时调用 `entry.key == key`；
   - equality 返回后立即检查 stamp，随后才使用结果。
5. 当前组没有命中时，记录最早 deleted。
6. 当前组存在精确 empty 时，返回 Absent；不能在检查本组 H2 候选前因 empty 提前终止，因为同组其他槽仍可能命中。
7. 否则进入下一组。

这保持当前 equality 方向 `storedKey == lookupKey`。用户 hash／equality 抛出时不提交外层操作；若回调先修改集合再正常返回，则修改检查抛 `ConcurrentModificationException`。

### 8.3 插入与替换

`put`／`putIfAbsent` 共用一次查找。

Found：

- 读取原 entry。
- `put` 写入 `(old.key, newValue, old.mixedHash)`；`putIfAbsent` 不写。
- 返回原 value。
- 实际替换只推进 `changes`，不推进 `structure`。

Absent：

1. 查找和所有用户回调已经完成。
2. 若 vacant 是 deleted，直接复用；`occupied` 不增加，不因已到阈值而无条件 rehash。
3. 若 vacant 是 empty 且 `occupied < maximumOccupied`，直接使用查找结果。
4. 若 vacant 是 empty 且已到阈值：
   - `elementCount < capacity / 2` 时 same-capacity rehash；
   - 否则倍增 rehash；
   - 在新表中仅凭 cached mixedHash 查找 vacant，不重新调用用户 hash／equality。
5. 先把完整 entry 写入 `elements`，再把 control byte 发布为 H2。
6. 增加 elementCount；只有消费 empty 时增加 occupied。
7. 推进 `changes` 与 `structure`。

以上保持当前“先完成 lookup，再提交”的顺序。rehash 构造独立新 storage，只有完整成功后才替换旧 storage；失败不发布半张表。

### 8.4 删除

命中后删除：

1. 复制待返回的 value。
2. 将 entry 槽写为 `MaybeUninit.uninit()`，释放 K／V 的 managed 引用。
3. 检查该组除当前槽外是否已有精确 empty：
   - 若有，将 control 设为 empty，并减少 occupied；
   - 若没有，将 control 设为 deleted，occupied 不变。
4. 减少 elementCount，推进 `changes` 与 `structure`。

安全依据是组内八个槽对同一 probe step 等价：若组内已有 empty，则任何仍存活、首组选在更早 probe step 的 key 在插入时都不可能越过该组。把本槽恢复为 empty 不会让合法查找提前终止。若组曾经全满，则必须留下 deleted 维持后续 probe chain。

删除不执行 backward shift，不移动其他 entry，不调用用户代码。

### 8.5 Rehash

rehash 按 control word 遍历旧表，只处理精确 full byte：

- 读取 entry 和 cached mixedHash；
- 按新 group mask 计算探测序列；
- 找到第一个 empty 并写入；新表初始没有 deleted；
- 不调用 hash、equality、ModificationCounter 或公开 Map 操作。

same-capacity rehash 结束后 `occupied == elementCount`。增长同样成立。旧 storage 在新表完全形成且字段替换后自然变为不可达，由 GC 回收；不手工释放托管数组。

### 8.6 Clear 与迭代

非空 clear 按 controls 扫描，只把精确 full 槽写为 uninit，再把每个 control word 重置为空组；deleted 槽已经是 uninit，不重复写 entry。随后两个计数归零并推进两个 modification counter。空 map 的 clear 继续是 no-op，即使内部仍有 deleted，也不改变 iterator 语义或为清 tombstone 伪造结构修改。

iterator 按 group、slot 顺序扫描 control，只读取精确 full 槽。该次顺序不对外承诺，改变现有偶然顺序合法。每次非 exhausted 的 `next()` 先检查 structure stamp；value 替换不使 iterator 失效，且尚未返回的槽读取当前 value。返回的 `Map.Entry` 仍是独立值。到达末尾后永久 exhausted，后续修改不使其恢复。

## 9. 回调、异常和提交边界

以下顺序是必要语义，不是可删除的性能检查：

- hash 返回后检查 `changes`。
- 每次实际 equality 返回后检查 `changes`。
- 回调抛出时原异常直接传播；外层操作不提交自己的修改。
- 回调正常返回但修改了同一 map 时抛 `ConcurrentModificationException`，不继续使用旧 control、entry 或 probe 结果。
- 回调只读取 map 时不推进 counter，可继续查找。
- lookup 完成后，capacity 检查、rehash、entry 写入和 control 发布均不得调用用户方法。

`NoGC`、可内联或 primitive 实现都不等于“不会修改当前 map”。首版不让编译器根据被选中的 hash／equality 正文删除这些检查。若未来要消除，必须由真实 effect／别名证明单独设计，不能按类型名或 intrinsic 名硬编码。

## 10. GC 与内存顺序

controls 是普通无引用 ULong 数组。elements 继续使用 `MutableArray<MaybeUninit<HashEntry<K,V>>>`，沿现有 value layout、recursive scan 和数组写屏障处理，不新增 runtime layout 或扫描 API。

必要规则：

- 构造 storage 时所有 entry 槽初始化为合法 uninit；不能只初始化 control 而留下任意 payload 位型。
- 插入先写 entry、后写 full control。GC 不依赖 control 判断扫描；写入后的引用由实际 wrapper 布局和写屏障保活。
- 删除先写 uninit、后写 empty/deleted，保持当前释放引用的行为。
- rehash 期间新旧数组及局部 entry 按普通生成代码形成 roots；不能保存跨 safepoint 的裸元素地址。
- 不能以 control 为 empty 为由跳过 typed GC scan，也不为 HashMap 增加按 elementCount 扫描的特殊对象模型。
- 含 class、interface、String、closure、Option、含引用 struct／enum／tuple 及相应 MaybeUninit 的 K／V 都走同一实现。

集合本身不承诺线程安全。control word 不使用原子读写；这不改变 GC card 标记和并发 collector 对托管数组的既有合同。

## 11. 实际消费者

### 11.1 HashSet

`HashSet<T>` 继续复用 `HashMap<T, Unit>`。重复 add 不替换最早对象、不推进 modification counter；实际新增和删除沿 HashMap 结构修改语义。ZST key/value 必须覆盖，不能假设 entry 至少具有普通 K／V 字节。

### 11.2 LinkedHashMap

`LinkedHashMap<K,V>` 的 `positions: HashMap<K, Long>` 使用新 `HashProbe`，`LinkedStorage` 和双向链接保持不变：

- replace 保留原 key 与链接位置；
- remove 从 positions 得到稳定的 LinkedStorage index 后删除；
- 删除再插入移动到链尾；
- positions 内部 rehash 只重排哈希槽，不改变保存的 LinkedStorage index。

`LinkedHashMap` 自己的 `changes` 检查继续包围 positions lookup，不能因底层 HashMap 已有检查而删除；两个 counter 分别检测底层索引和外层 value/link 的真实修改。

### 11.3 Map views 与 codec

keys／values／entries 仍是 live view；map codec 继续按当前迭代顺序写 entry 序列。普通 HashMap 顺序本来未规定，改造不改变 wire 形状或重复 key 规则。LinkedHashMap 的 codec 顺序由链接存储保证，不受 positions 的槽顺序影响。

## 12. Spec、编译器与产物影响

首版是 `scoop.std` 普通源码实现调整：

- 语言语法、类型系统、Hash／Equals 接口和公开集合 API 不变。
- 标准库规范已有的等价、复杂度、迭代和修改语义不变，不需要新增规范能力。
- HIR／MIR／LIR、`.slib` 格式、runtime ABI、metadata ABI、对象头和 GC ABI 不变。
- 预编译 std 机器正文和依赖缓存需要正常重建；不保留旧内部 HashStorage 布局或双轨实现。
- 首版预期不改 compiler。若普通小 helper 未能内联或 bounds check 未消除，先以实际 MIR／机器码定位已有通用优化缺口；不直接增加 HashMap 专用 lowering。

若后续增加公开位操作、编译器 intrinsic、目标 SIMD 或新的复杂度／容量行为，必须另行 spec 先行，不能追记为本设计已授权的范围。

## 13. 实施顺序

按可独立验证的功能批次推进：

1. **控制字与纯 helper**：control 读写、exact byte、match mask、候选枚举、hash mixer、H1/H2 和组序列；用小而穷尽的 Cone 测试覆盖 256 个 byte 值、各槽位置、false positive、环绕和全部组数。
2. **完整 storage**：新布局、lookup、single-probe absent、insert、replace、remove、same-capacity rehash、growth rehash、clear、iteration；删除旧 `states`／`vacant`，不保留平行实现。
3. **消费者迁移**：HashSet、LinkedHashMap 内部 probe 接口、views 和 codec 组合测试。
4. **GC 与跨产物验证**：含引用 K／V、移动／minor GC、预编译 std 消费及三目标机器产物。
5. **完整性能对照**：基线与新实现使用同一 compiler、runtime、core/std 构建；确认收益、回退、内存和 tombstone 行为后再决定 7/8 负载率。

每批 Scoop 源码先使用正式 formatter；若没有 Rust／Python 改动，不为文档或 std 源码机械运行无关的全 workspace lint。最终实现仍按仓库规则完成相关格式、lint、Cone 测试与正式 CLI/GC 验收。

## 14. 正确性测试

普通行为优先放在 `scoop.std` 所属 Cone tests，不为普通集合 API 新增 fixture。

### 14.1 Storage 与算法

- 初始 8 槽、跨组、最后一组环绕、每个逻辑 byte 位置。
- empty／deleted／所有 128 个 H2、match false positive 和 exact 过滤。
- groupCount 1、2、4、8 及接近增长阈值的全部 elementCount。
- 命中、未命中、第一次 deleted 优先、empty 终止、全表同 H2。
- 恒定完整 hash、负 Long hash、Long 极值、零大小 key/value。
- 同容量清理和倍增后所有 key/value、size 和原 key 身份保持。
- 删除组有 empty 时恢复 empty；全组时留下 deleted；后续 key 不丢失。
- 固定大小长期 insert/remove churn 不出错、不无限循环。

### 14.2 公开语义

- `V = Option<T>` 时外层 None 与 `Some(None)` 层级保持。
- put 替换 value 保留第一次 key 身份；putIfAbsent／Set.add no-op 不修改 iterator。
- value 替换不使 iterator 失效；插入、删除、非空 clear 使未 exhausted iterator 失效。
- iterator 永久 exhausted；已返回 Entry 是独立值；live views 读取当前内容。
- hash／equality 抛出时外层不提交；回调重入 clear、replace、insert、remove 时保持现有诊断和最终状态。
- LinkedHashMap replace、remove/reinsert、扩容、codec 后仍保持插入顺序。
- HashMap codec wire 形状、重复 key 和异常 path 不变。

### 14.3 GC 与产物

- class／String key，class／interface／closure value。
- 含引用 struct、tuple、enum、Option 和 MaybeUninit 组合。
- 删除、clear、same-capacity rehash 和 growth 后，旧引用不被空槽意外保活。
- moving 与 minor stress 下 lookup、rehash、LinkedHashMap positions 和迭代结果正确。
- std 产物由普通 consumer 使用；不以删除或隐藏源码、工具验证缓存或产物。
- 保留现有 `gc-correctness/hash/values-gc` 等确实需要外部 stress 控制的 fixture，不为普通行为复制冷/热构建矩阵。

## 15. 性能对照与接受标准

微基准只用于选实现，不成为测试中的时间阈值。原始逐轮数据、机器码和临时变体放在仓库外，文档只保留范围与结果概要。

### 15.1 工作负载

至少覆盖：

| 维度 | 场景 |
| --- | --- |
| 大小 | 0～16 小表；每个倍增周期的低、中、高负载；64K 与更大表 |
| key | Long；短/长 String；相同 String 对象；等内容不同对象；自定义便宜/昂贵 equality |
| hash | 正常均匀；连续值；同 H2；恒定完整 hash |
| 操作 | build；hit；miss；50% hit/miss；replace；remove+insert；clear；iterate |
| churn | 同 key 重插；不同 home group；固定 size 完整 turnover；deleted 接近阈值 |
| value | Long；引用；interface；小/大值类型 |

大小不能只取二次幂或固定 50% 负载；应在完整 doubling sawtooth 上比较，避免让某一实现恰好处于最空或最满状态。

### 15.2 记录项

- 函数内 ns/op 或总阶段时间，预热、多轮交替、中位数与范围。
- build、lookup、replace、churn 分开，不把不同工作量比值解释成单项收益。
- 分配字节、峰值 RSS、minor/full 次数和扩容次数。
- 外部实验变体中的平均/分位组数、H2 候选数、equality 次数和 same-capacity rehash 次数；不把这些计数器提交进生产类。
- 优化后 LLVM IR 和最终 AArch64／x86-64 机器码：确认 control word 单次读取、SWAR 整数序列、候选 entry 读取和未命中单次探测。
- 编译时间、std 产物大小及最终 consumer 体积，防止为小热路径产生过量泛型机器码。

### 15.3 合并标准

必须先满足全部行为、回调和 GC 正确性。性能上不要求所有场景单调变快，但应同时满足：

- Long 和 String 的主流 hit/miss/build 至少没有稳定整体回退，且一项以上真实完整操作有可重复收益。
- 高负载或 tombstone 场景的探测/equality 明显减少；不能只展示 control-byte 局部探针。
- 小表、恒定 hash、大 value 和迭代的回退被单独记录并在可接受范围内。
- 同容量 control 元数据从 4 B/槽降至 1 B/槽；总 RSS 以实测为准，不把逻辑字节直接当成进程节省。
- 三目标正确性一致；性能结论至少包含一台 AArch64 和一台 x86-64 原生运行，不用交叉编译对象代替墙钟。

## 16. 后续候选

以下均不属于首版，只有对应证据出现后单独设计和验证：

1. **7/8 最大负载率**：比较查找、增长频率、RSS 与 churn 后再改变常量；不与首版分组改造混测。
2. **目标 SIMD**：以 SSE2／NEON 扩大或加速 control compare；保持普通 ULong 路径作为已验证语义基线，不先扩展语言 SIMD。
3. **`countTrailingZeroBits`**：若候选枚举在 profile 中可见，作为通用整数能力 spec 先行，而不是 HashMap 私有 opcode。
4. **Overflow counter**：若 deleted 清理在固定大小 churn 中仍是主要成本，比较 F14／新 unordered_dense 的组溢出计数；需证明计数增减、上溢和异常路径。
5. **大 entry 的密集值数组**：只有真实大值类型显示空槽浪费主导时评估 index+value vector；需要独立处理索引宽度、删除搬移和额外间接访问。
6. **增量 rehash**：只有大表插入尾延迟构成实际产品问题时评估，不因 Go 已实现就增加目录层。
7. **String hash 缓存或新 hash**：与 HashMap 私有 mixing 分开评估，涉及 String 布局或公开 hash 结果时按相应 spec 和 ABI 独立推进。

## 17. 实现与验证结果

首版已按本文设计落入 `scoop.std`，范围只包括 HashMap 内部存储、探测、`LinkedHashMap` 的内部调用适配和定向测试；没有修改编译器、runtime、语言语义、公开集合 API 或 spec。主要结果如下：

- control metadata 改为每组一个 `ULong`，即每槽一个 control byte；同容量逻辑元数据由 4 B/槽降为 1 B/槽。
- entry 缓存完整 mixed hash，查找以 H2 筛选候选；未命中插入直接复用 `Absent` 返回的槽位和 mixed hash。
- 组间使用有界三角步长探测；删除按同组是否已有 empty 选择恢复 empty 或留下 deleted。
- 超过 75% `occupied` 阈值时，按 live 数量选择同容量清理或倍增；rehash 不调用用户 `hash`／`equals`。
- 原 key 身份、修改检测、迭代器失效、`LinkedHashMap` 顺序和 GC 可见性合同保持不变。

### 17.1 性能对照

测试环境为 Apple M3 Ultra、macOS 27.2，仓库提交 `b8ca1c9ba68aa15a16e5a3fb55400f756e6ecb57`。编译器和基准使用仓库四个 host patch 构建的 LLVM 22.1.8。基线可执行文件在改动前保留；完成后用相同编译器、参数和输入生成新可执行文件。以下为 31 轮交替执行、轮内场景轮转后的中位数，正数表示变慢：

| 场景 | 基线 ns/op | 实现后 ns/op | 变化 |
| --- | ---: | ---: | ---: |
| build `Long` | 42.511 | 36.240 | -14.75% |
| hit `Long` | 12.560 | 12.960 | +3.19% |
| miss `Long` | 17.357 | 10.164 | -41.44% |
| replace `Long` | 14.556 | 13.900 | -4.51% |
| remove+insert `Long` | 43.797 | 43.413 | -0.88% |
| hit `String` | 24.002 | 24.005 | +0.01% |

build 和 miss 获得明确收益，replace 和固定大小 churn 小幅改善，String hit 持平。Long hit 有稳定但较小的 3.19% 回退；综合 build、miss、元数据占用和其他操作结果，首版保留 75% 阈值，不为这项回退引入第二套探测路径或 SIMD。逐轮原始数据保存在仓库外的 `/Volumes/Data/home/chenxu/scoop-performance-results/2026-11-03/hashmap-swiss-table/final/`。

### 17.2 机器码与正确性

AArch64 最终机器码中，每轮组探测只读取一次 64 位 control word；H2 匹配内联为普通整数 `eor`／`add`／`bic`／`and` 序列，枚举命中的 control byte 后才读取对应 entry。该结果不依赖 SIMD、runtime helper 或编译器 intrinsic。

新增定向测试穷举 256 个 control byte 值和八个槽位，覆盖 SWAR 候选无漏报、候选精确复核、empty/deleted 删除转换、同容量 rehash 和三角探测回绕。最终 `scoop.std` release 测试共 98 个容器、106 个测试，全部通过。由于实现未涉及编译器，按本次任务范围未运行 fixture、Rust 测试套件或其他平台性能验证；第 15.3 节的跨架构性能采样仍是后续发布验收项。

## 18. 参考

- [Abseil Swiss Tables Design Notes](https://abseil.io/about/design/swisstables)
- [Go Swiss map 实现说明](https://go.googlesource.com/go/+/refs/heads/master/src/internal/runtime/maps/map.go)
- [Go: Faster Go maps with Swiss Tables](https://go.dev/blog/swisstable)
- [Folly F14 Hash Table](https://github.com/facebook/folly/blob/main/folly/container/F14.md)
- [robin-hood-hashing](https://github.com/martinus/robin-hood-hashing)
- [unordered_dense](https://github.com/martinus/unordered_dense)
- [本仓库基础容器调研](CORE-CONTAINERS-RESEARCH.md#4-hashmap-与-swiss-table)
- [M62 HashMap 槽位掩码对照](../milestone62/EVALUATION.md#hashmap-槽位掩码)
