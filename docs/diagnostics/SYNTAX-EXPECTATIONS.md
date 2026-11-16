# Parser 的预期语法参数

共有 **136** 个不同的静态预期描述，用于 `expected {what}, found …` 等共有模板；它们不额外计入错误模板数，也不是已经确定的独立错误规则。诸如缺少函数名、缺少括号是否需要独立编号，可结合调用位置与语法规则决定。见 [盘点总表](INVENTORY.md)。

| 序号 | 预期内容 | 调用位置 |
| --- | --- | --- |
| 1 | `` `(` `` | [parser/src/decl/functions/parameters.rs:8](../../compiler/parser/src/decl/functions/parameters.rs#L8)；[parser/src/expr/arguments.rs:31](../../compiler/parser/src/expr/arguments.rs#L31)；[parser/src/expr/callables/parameters.rs:8](../../compiler/parser/src/expr/callables/parameters.rs#L8)；[parser/src/stmt/control_flow/conditionals.rs:27](../../compiler/parser/src/stmt/control_flow/conditionals.rs#L27)；[parser/src/stmt/control_flow/exceptions.rs:122](../../compiler/parser/src/stmt/control_flow/exceptions.rs#L122)；[parser/src/stmt/control_flow/loops.rs:20](../../compiler/parser/src/stmt/control_flow/loops.rs#L20)；[parser/src/stmt/control_flow/loops.rs:72](../../compiler/parser/src/stmt/control_flow/loops.rs#L72)；[parser/src/stmt/control_flow/loops.rs:120](../../compiler/parser/src/stmt/control_flow/loops.rs#L120)；[parser/src/stmt/when/subject.rs:8](../../compiler/parser/src/stmt/when/subject.rs#L8)；[parser/src/ty.rs:125](../../compiler/parser/src/ty.rs#L125) |
| 2 | `` `(` after `constructor` `` | [parser/src/decl/nominals/constructors/parameters.rs:88](../../compiler/parser/src/decl/nominals/constructors/parameters.rs#L88) |
| 3 | `` `(` after `context` `` | [parser/src/decl/context.rs:17](../../compiler/parser/src/decl/context.rs#L17) |
| 4 | `` `(` after `get` `` | [parser/src/decl/properties.rs:292](../../compiler/parser/src/decl/properties.rs#L292) |
| 5 | `` `(` after `set` `` | [parser/src/decl/properties.rs:312](../../compiler/parser/src/decl/properties.rs#L312) |
| 6 | `` `(` in member requirement `` | [parser/src/decl/generics/requirements.rs:136](../../compiler/parser/src/decl/generics/requirements.rs#L136) |
| 7 | `` `)` `` | [parser/src/decl/generics/requirements.rs:29](../../compiler/parser/src/decl/generics/requirements.rs#L29)；[parser/src/decl/generics/requirements.rs:179](../../compiler/parser/src/decl/generics/requirements.rs#L179)；[parser/src/pattern.rs:168](../../compiler/parser/src/pattern.rs#L168)；[parser/src/stmt/control_flow/conditionals.rs:31](../../compiler/parser/src/stmt/control_flow/conditionals.rs#L31)；[parser/src/stmt/control_flow/exceptions.rs:132](../../compiler/parser/src/stmt/control_flow/exceptions.rs#L132)；[parser/src/stmt/control_flow/loops.rs:25](../../compiler/parser/src/stmt/control_flow/loops.rs#L25)；[parser/src/stmt/control_flow/loops.rs:91](../../compiler/parser/src/stmt/control_flow/loops.rs#L91)；[parser/src/stmt/control_flow/loops.rs:125](../../compiler/parser/src/stmt/control_flow/loops.rs#L125)；[parser/src/stmt/when/subject.rs:41](../../compiler/parser/src/stmt/when/subject.rs#L41) |
| 8 | `` `)` after `get(` `` | [parser/src/decl/properties.rs:293](../../compiler/parser/src/decl/properties.rs#L293) |
| 9 | `` `)` after context binding value `` | [parser/src/expr/context.rs:85](../../compiler/parser/src/expr/context.rs#L85) |
| 10 | `` `)` after setter parameter `` | [parser/src/decl/properties.rs:324](../../compiler/parser/src/decl/properties.rs#L324) |
| 11 | `` `,` or `}` after copy update field `` | [parser/src/expr/places.rs:63](../../compiler/parser/src/expr/places.rs#L63) |
| 12 | `` `,` or newline after variant `` | [parser/src/decl/enums.rs:236](../../compiler/parser/src/decl/enums.rs#L236) |
| 13 | `` `->` `` | [parser/src/stmt/when/arms.rs:64](../../compiler/parser/src/stmt/when/arms.rs#L64) |
| 14 | `` `.` after qualified `super` interface `` | [parser/src/expr/primary.rs:198](../../compiler/parser/src/expr/primary.rs#L198) |
| 15 | `` `::` `` | [parser/src/expr/callables.rs:36](../../compiler/parser/src/expr/callables.rs#L36) |
| 16 | `` `:` `` | [parser/src/decl/annotations/declaration.rs:85](../../compiler/parser/src/decl/annotations/declaration.rs#L85)；[parser/src/decl/enums/fields.rs:80](../../compiler/parser/src/decl/enums/fields.rs#L80)；[parser/src/decl/enums/fields.rs:118](../../compiler/parser/src/decl/enums/fields.rs#L118)；[parser/src/decl/functions/parameters.rs:30](../../compiler/parser/src/decl/functions/parameters.rs#L30)；[parser/src/decl/generics.rs:157](../../compiler/parser/src/decl/generics.rs#L157)；[parser/src/decl/generics/requirements.rs:21](../../compiler/parser/src/decl/generics/requirements.rs#L21)；[parser/src/decl/nominals/constructors/parameters.rs:58](../../compiler/parser/src/decl/nominals/constructors/parameters.rs#L58)；[parser/src/decl/nominals/constructors/parameters.rs:117](../../compiler/parser/src/decl/nominals/constructors/parameters.rs#L117)；[parser/src/decl/nominals/fields.rs:37](../../compiler/parser/src/decl/nominals/fields.rs#L37)；[parser/src/expr/callables/parameters.rs:21](../../compiler/parser/src/expr/callables/parameters.rs#L21) |
| 17 | `` `:` after context parameter name (modifiers are not allowed) `` | [parser/src/decl/context.rs:47](../../compiler/parser/src/decl/context.rs#L47) |
| 18 | `` `:` after copy update field name `` | [parser/src/expr/places.rs:37](../../compiler/parser/src/expr/places.rs#L37) |
| 19 | `` `;` or newline after statement `` | [parser/src/stmt.rs:28](../../compiler/parser/src/stmt.rs#L28) |
| 20 | `` `=` `` | [parser/src/stmt.rs:309](../../compiler/parser/src/stmt.rs#L309) |
| 21 | `` `=` in const property `` | [parser/src/decl/properties.rs:137](../../compiler/parser/src/decl/properties.rs#L137) |
| 22 | `` `=` in typealias declaration `` | [parser/src/decl/type_aliases.rs:57](../../compiler/parser/src/decl/type_aliases.rs#L57) |
| 23 | `` `>` after call type argument list `` | [parser/src/expr/primary.rs:338](../../compiler/parser/src/expr/primary.rs#L338) |
| 24 | `` `>` after qualified `super` interface `` | [parser/src/expr/primary.rs:195](../../compiler/parser/src/expr/primary.rs#L195) |
| 25 | `` `@` `` | [parser/src/expr/labels.rs:16](../../compiler/parser/src/expr/labels.rs#L16)；[parser/src/stmt.rs:352](../../compiler/parser/src/stmt.rs#L352) |
| 26 | `` `Companion` in projected constraint `` | [parser/src/decl/generics.rs:146](../../compiler/parser/src/decl/generics.rs#L146) |
| 27 | `` `]` `` | [parser/src/expr/places.rs:111](../../compiler/parser/src/expr/places.rs#L111) |
| 28 | `` `catch` or `finally` `` | [parser/src/stmt/control_flow/exceptions.rs:172](../../compiler/parser/src/stmt/control_flow/exceptions.rs#L172) |
| 29 | `` `class` `` | [parser/src/decl/mod.rs:137](../../compiler/parser/src/decl/mod.rs#L137)；[parser/src/decl/nominals.rs:122](../../compiler/parser/src/decl/nominals.rs#L122) |
| 30 | `` `class` after `annotation` `` | [parser/src/decl/annotations/declaration.rs:30](../../compiler/parser/src/decl/annotations/declaration.rs#L30) |
| 31 | `` `enum` `` | [parser/src/decl/enums.rs:34](../../compiler/parser/src/decl/enums.rs#L34) |
| 32 | `` `fun` `` | [parser/src/decl/functions.rs:126](../../compiler/parser/src/decl/functions.rs#L126)；[parser/src/expr/callables.rs:63](../../compiler/parser/src/expr/callables.rs#L63) |
| 33 | `` `fun` or `{` after `suspend` `` | [parser/src/expr/primary.rs:32](../../compiler/parser/src/expr/primary.rs#L32) |
| 34 | `` `fun`, `val`, `var`, `typealias`, `struct`, `enum`, `class` or `interface` `` | [parser/src/decl/mod.rs:222](../../compiler/parser/src/decl/mod.rs#L222) |
| 35 | `` `import` `` | [parser/src/header.rs:53](../../compiler/parser/src/header.rs#L53) |
| 36 | `` `in` `` | [parser/src/stmt/control_flow/loops.rs:122](../../compiler/parser/src/stmt/control_flow/loops.rs#L122) |
| 37 | `` `interface` `` | [parser/src/decl/nominals.rs:197](../../compiler/parser/src/decl/nominals.rs#L197)；[parser/src/decl/nominals/interface_implementations.rs:41](../../compiler/parser/src/decl/nominals/interface_implementations.rs#L41) |
| 38 | `` `object` `` | [parser/src/decl/nominals/objects.rs:26](../../compiler/parser/src/decl/nominals/objects.rs#L26) |
| 39 | `` `object` after `companion` `` | [parser/src/decl/nominals/objects.rs:99](../../compiler/parser/src/decl/nominals/objects.rs#L99) |
| 40 | `` `package` `` | [parser/src/header.rs:25](../../compiler/parser/src/header.rs#L25) |
| 41 | `` `struct` `` | [parser/src/decl/nominals.rs:39](../../compiler/parser/src/decl/nominals.rs#L39) |
| 42 | `` `typealias` `` | [parser/src/decl/type_aliases.rs:50](../../compiler/parser/src/decl/type_aliases.rs#L50) |
| 43 | `` `val` `` | [parser/src/decl/enums/fields.rs:77](../../compiler/parser/src/decl/enums/fields.rs#L77)；[parser/src/decl/nominals/fields.rs:34](../../compiler/parser/src/decl/nominals/fields.rs#L34) |
| 44 | `` `val` annotation parameter `` | [parser/src/decl/annotations/declaration.rs:78](../../compiler/parser/src/decl/annotations/declaration.rs#L78) |
| 45 | `` `val` or `var` `` | [parser/src/decl/properties.rs:69](../../compiler/parser/src/decl/properties.rs#L69) |
| 46 | `` `while` after do body `` | [parser/src/stmt/control_flow/loops.rs:69](../../compiler/parser/src/stmt/control_flow/loops.rs#L69) |
| 47 | `` `{` `` | [parser/src/decl/enums.rs:63](../../compiler/parser/src/decl/enums.rs#L63)；[parser/src/decl/nominals/interface_implementations.rs:62](../../compiler/parser/src/decl/nominals/interface_implementations.rs#L62)；[parser/src/decl/nominals/interfaces.rs:20](../../compiler/parser/src/decl/nominals/interfaces.rs#L20)；[parser/src/expr/callables.rs:146](../../compiler/parser/src/expr/callables.rs#L146)；[parser/src/stmt/blocks.rs:19](../../compiler/parser/src/stmt/blocks.rs#L19)；[parser/src/stmt/when.rs:48](../../compiler/parser/src/stmt/when.rs#L48) |
| 48 | `` `{` after requires parameters `` | [parser/src/decl/generics/requirements.rs:31](../../compiler/parser/src/decl/generics/requirements.rs#L31) |
| 49 | `` `}` `` | [parser/src/decl/enums.rs:77](../../compiler/parser/src/decl/enums.rs#L77)；[parser/src/decl/nominals/interface_implementations.rs:70](../../compiler/parser/src/decl/nominals/interface_implementations.rs#L70)；[parser/src/decl/nominals/interfaces.rs:31](../../compiler/parser/src/decl/nominals/interfaces.rs#L31)；[parser/src/decl/nominals/members.rs:21](../../compiler/parser/src/decl/nominals/members.rs#L21)；[parser/src/decl/nominals/members.rs:312](../../compiler/parser/src/decl/nominals/members.rs#L312)；[parser/src/expr/places.rs:73](../../compiler/parser/src/expr/places.rs#L73)；[parser/src/stmt/blocks.rs:41](../../compiler/parser/src/stmt/blocks.rs#L41)；[parser/src/stmt/when.rs:112](../../compiler/parser/src/stmt/when.rs#L112)；[parser/src/stmt/when.rs:117](../../compiler/parser/src/stmt/when.rs#L117) |
| 50 | `` `}` after interpolation expression `` | [parser/src/expr/interpolation.rs:27](../../compiler/parser/src/expr/interpolation.rs#L27) |
| 51 | ` a call has an argument list or an actual trailing input ` | [parser/src/parser/syntax/bodies/calls.rs:84](../../compiler/parser/src/parser/syntax/bodies/calls.rs#L84) |
| 52 | ` a non-empty successful input produces a first parsed source ` | [parser/src/source_input.rs:183](../../compiler/parser/src/source_input.rs#L183) |
| 53 | ` a successful top-level production has just finished its declaration ` | [parser/src/parser/syntax.rs:129](../../compiler/parser/src/parser/syntax.rs#L129) |
| 54 | ` annotation argument name ` | [parser/src/decl/annotations.rs:98](../../compiler/parser/src/decl/annotations.rs#L98) |
| 55 | ` annotation class name ` | [parser/src/decl/annotations/declaration.rs:33](../../compiler/parser/src/decl/annotations/declaration.rs#L33) |
| 56 | ` annotation parameter name ` | [parser/src/decl/annotations/declaration.rs:83](../../compiler/parser/src/decl/annotations/declaration.rs#L83) |
| 57 | ` annotation use-site target ` | [parser/src/decl/annotations.rs:44](../../compiler/parser/src/decl/annotations.rs#L44) |
| 58 | ` annotation value ` | [parser/src/decl/annotations/values.rs:18](../../compiler/parser/src/decl/annotations/values.rs#L18) |
| 59 | ` anonymous object body ` | [parser/src/decl/local_types.rs:110](../../compiler/parser/src/decl/local_types.rs#L110) |
| 60 | ` argument name ` | [parser/src/expr/arguments.rs:94](../../compiler/parser/src/expr/arguments.rs#L94) |
| 61 | ` base class or interface name ` | [parser/src/decl/nominals/supertypes.rs:22](../../compiler/parser/src/decl/nominals/supertypes.rs#L22) |
| 62 | `` base method name after `super.` `` | [parser/src/expr/primary.rs:166](../../compiler/parser/src/expr/primary.rs#L166) |
| 63 | `` binding name after `val` `` | [parser/src/pattern.rs:120](../../compiler/parser/src/pattern.rs#L120) |
| 64 | ` block grammar events are paired ` | [parser/src/parser/syntax/bodies.rs:186](../../compiler/parser/src/parser/syntax/bodies.rs#L186) |
| 65 | ` call arguments ` | [parser/src/expr/trailing.rs:33](../../compiler/parser/src/expr/trailing.rs#L33) |
| 66 | ` callable label ` | [parser/src/expr/labels.rs:15](../../compiler/parser/src/expr/labels.rs#L15) |
| 67 | `` callable name after `::` `` | [parser/src/expr/callables.rs:37](../../compiler/parser/src/expr/callables.rs#L37) |
| 68 | ` catch parameter name ` | [parser/src/stmt/control_flow/exceptions.rs:123](../../compiler/parser/src/stmt/control_flow/exceptions.rs#L123) |
| 69 | ` checked above ` | [parser/src/decl/properties.rs:182](../../compiler/parser/src/decl/properties.rs#L182) |
| 70 | ` class name ` | [parser/src/decl/nominals.rs:123](../../compiler/parser/src/decl/nominals.rs#L123) |
| 71 | ` companion object name ` | [parser/src/decl/nominals/objects.rs:106](../../compiler/parser/src/decl/nominals/objects.rs#L106) |
| 72 | ` completed declarations have a grammar owner ` | [parser/src/parser/syntax.rs:146](../../compiler/parser/src/parser/syntax.rs#L146) |
| 73 | ` constructor parameter name ` | [parser/src/decl/nominals/constructors/parameters.rs:115](../../compiler/parser/src/decl/nominals/constructors/parameters.rs#L115) |
| 74 | `` context parameter name or `_` `` | [parser/src/decl/context.rs:45](../../compiler/parser/src/decl/context.rs#L45) |
| 75 | ` copy update field name ` | [parser/src/expr/places.rs:30](../../compiler/parser/src/expr/places.rs#L30) |
| 76 | ` declaration prefixes have a grammar owner ` | [parser/src/parser/syntax.rs:157](../../compiler/parser/src/parser/syntax.rs#L157) |
| 77 | ` end of function body ` | [parser/src/parser/entry.rs:17](../../compiler/parser/src/parser/entry.rs#L17) |
| 78 | ` end of type ` | [parser/src/parser/entry.rs:29](../../compiler/parser/src/parser/entry.rs#L29) |
| 79 | ` enum name ` | [parser/src/decl/enums.rs:35](../../compiler/parser/src/decl/enums.rs#L35) |
| 80 | ` expression ` | [parser/src/expr/primary.rs:138](../../compiler/parser/src/expr/primary.rs#L138)；[parser/src/stmt/when.rs:156](../../compiler/parser/src/stmt/when.rs#L156) |
| 81 | `` f-string text, `${...}`, or closing quote `` | [parser/src/expr/interpolation.rs:42](../../compiler/parser/src/expr/interpolation.rs#L42) |
| 82 | ` field declaration ` | [parser/src/decl/enums/fields.rs:102](../../compiler/parser/src/decl/enums/fields.rs#L102) |
| 83 | ` field name ` | [parser/src/decl/enums/fields.rs:78](../../compiler/parser/src/decl/enums/fields.rs#L78)；[parser/src/decl/enums/fields.rs:116](../../compiler/parser/src/decl/enums/fields.rs#L116)；[parser/src/decl/nominals/fields.rs:35](../../compiler/parser/src/decl/nominals/fields.rs#L35)；[parser/src/expr/postfix.rs:182](../../compiler/parser/src/expr/postfix.rs#L182)；[parser/src/pattern/lists.rs:137](../../compiler/parser/src/pattern/lists.rs#L137) |
| 84 | ` field name or tuple index ` | [parser/src/expr/postfix.rs:178](../../compiler/parser/src/expr/postfix.rs#L178) |
| 85 | ` function grammar records a header within its declaration ` | [parser/src/parser/syntax.rs:168](../../compiler/parser/src/parser/syntax.rs#L168) |
| 86 | ` function name ` | [parser/src/decl/functions.rs:163](../../compiler/parser/src/decl/functions.rs#L163) |
| 87 | `` identifier after `.` `` | [parser/src/header.rs:124](../../compiler/parser/src/header.rs#L124)；[parser/src/header.rs:167](../../compiler/parser/src/header.rs#L167) |
| 88 | ` import alias ` | [parser/src/header.rs:59](../../compiler/parser/src/header.rs#L59) |
| 89 | ` import selector ` | [parser/src/header.rs:139](../../compiler/parser/src/header.rs#L139) |
| 90 | `` index expression after `,` `` | [parser/src/expr/places.rs:107](../../compiler/parser/src/expr/places.rs#L107) |
| 91 | ` infix function name ` | [parser/src/expr.rs:171](../../compiler/parser/src/expr.rs#L171) |
| 92 | `` interface member name after qualified `super` `` | [parser/src/expr/primary.rs:201](../../compiler/parser/src/expr/primary.rs#L201) |
| 93 | ` interface name ` | [parser/src/decl/nominals.rs:198](../../compiler/parser/src/decl/nominals.rs#L198) |
| 94 | ` lambda or anonymous function after label ` | [parser/src/expr/labels.rs:21](../../compiler/parser/src/expr/labels.rs#L21)；[parser/src/expr/labels.rs:44](../../compiler/parser/src/expr/labels.rs#L44) |
| 95 | ` loop after label ` | [parser/src/stmt.rs:357](../../compiler/parser/src/stmt.rs#L357) |
| 96 | ` loop label ` | [parser/src/stmt.rs:351](../../compiler/parser/src/stmt.rs#L351)；[parser/src/stmt/jumps.rs:28](../../compiler/parser/src/stmt/jumps.rs#L28) |
| 97 | ` loop label on the same line ` | [parser/src/stmt/jumps.rs:26](../../compiler/parser/src/stmt/jumps.rs#L26) |
| 98 | ` nested nominal declaration ` | [parser/src/decl/nominals/members.rs:293](../../compiler/parser/src/decl/nominals/members.rs#L293) |
| 99 | `` nested type name after `.` `` | [parser/src/ty.rs:207](../../compiler/parser/src/ty.rs#L207) |
| 100 | `` newline or `;` between member requirements `` | [parser/src/decl/generics/requirements.rs:43](../../compiler/parser/src/decl/generics/requirements.rs#L43) |
| 101 | ` nominal header owner ` | [parser/src/parser/syntax/nominals.rs:45](../../compiler/parser/src/parser/syntax/nominals.rs#L45) |
| 102 | ` nominal member production ` | [parser/src/parser/syntax/nominals.rs:53](../../compiler/parser/src/parser/syntax/nominals.rs#L53)；[parser/src/parser/syntax/nominals.rs:58](../../compiler/parser/src/parser/syntax/nominals.rs#L58) |
| 103 | ` numeric literal after unary minus ` | [parser/src/pattern.rs:171](../../compiler/parser/src/pattern.rs#L171) |
| 104 | ` object ` | [parser/src/decl/nominals/objects.rs:27](../../compiler/parser/src/decl/nominals/objects.rs#L27)；[parser/src/decl/nominals/objects.rs:100](../../compiler/parser/src/decl/nominals/objects.rs#L100) |
| 105 | ` object name ` | [parser/src/decl/nominals/objects.rs:29](../../compiler/parser/src/decl/nominals/objects.rs#L29) |
| 106 | ` one condition is parsed before its separator ` | [parser/src/stmt/when/conditions.rs:63](../../compiler/parser/src/stmt/when/conditions.rs#L63) |
| 107 | ` one parenthesized expression ` | [parser/src/expr/primary.rs:380](../../compiler/parser/src/expr/primary.rs#L380) |
| 108 | ` one parenthesized pattern ` | [parser/src/pattern/lists.rs:62](../../compiler/parser/src/pattern/lists.rs#L62) |
| 109 | ` one parenthesized type ` | [parser/src/ty.rs:171](../../compiler/parser/src/ty.rs#L171) |
| 110 | ` parameter name ` | [parser/src/decl/functions/parameters.rs:28](../../compiler/parser/src/decl/functions/parameters.rs#L28)；[parser/src/expr/callables/parameters.rs:19](../../compiler/parser/src/expr/callables/parameters.rs#L19) |
| 111 | ` parse-all validates unique identities before construction ` | [parser/src/source_input.rs:186](../../compiler/parser/src/source_input.rs#L186) |
| 112 | ` path holds one ident ` | [parser/src/pattern.rs:212](../../compiler/parser/src/pattern.rs#L212) |
| 113 | ` path is non-empty ` | [parser/src/pattern.rs:220](../../compiler/parser/src/pattern.rs#L220) |
| 114 | ` pattern ` | [parser/src/pattern.rs:109](../../compiler/parser/src/pattern.rs#L109) |
| 115 | ` primary constructor parameter name ` | [parser/src/decl/nominals/constructors/parameters.rs:56](../../compiler/parser/src/decl/nominals/constructors/parameters.rs#L56) |
| 116 | ` property name ` | [parser/src/decl/properties.rs:115](../../compiler/parser/src/decl/properties.rs#L115) |
| 117 | `` receiver label after `this@` `` | [parser/src/expr/primary.rs:82](../../compiler/parser/src/expr/primary.rs#L82) |
| 118 | ` required argument name ` | [parser/src/decl/generics/requirements.rs:147](../../compiler/parser/src/decl/generics/requirements.rs#L147) |
| 119 | ` required companion method name ` | [parser/src/decl/generics/requirements.rs:92](../../compiler/parser/src/decl/generics/requirements.rs#L92) |
| 120 | ` required method name ` | [parser/src/decl/generics/requirements.rs:88](../../compiler/parser/src/decl/generics/requirements.rs#L88) |
| 121 | ` requirement receiver or type parameter ` | [parser/src/decl/generics/requirements.rs:85](../../compiler/parser/src/decl/generics/requirements.rs#L85) |
| 122 | ` return label ` | [parser/src/stmt/jumps.rs:62](../../compiler/parser/src/stmt/jumps.rs#L62) |
| 123 | ` return label on the same line ` | [parser/src/stmt/jumps.rs:60](../../compiler/parser/src/stmt/jumps.rs#L60) |
| 124 | ` setter parameter name ` | [parser/src/decl/properties.rs:320](../../compiler/parser/src/decl/properties.rs#L320) |
| 125 | ` struct name ` | [parser/src/decl/nominals.rs:40](../../compiler/parser/src/decl/nominals.rs#L40) |
| 126 | ` the lexer emits EOF ` | [parser/src/parser/entry.rs:112](../../compiler/parser/src/parser/entry.rs#L112) |
| 127 | ` the parser rejected an empty copy-update list ` | [parser/src/expr/places.rs:77](../../compiler/parser/src/expr/places.rs#L77) |
| 128 | ` token stream ends in EOF ` | [parser/src/stmt/control_flow/conditionals.rs:62](../../compiler/parser/src/stmt/control_flow/conditionals.rs#L62) |
| 129 | ` type ` | [parser/src/ty.rs:118](../../compiler/parser/src/ty.rs#L118) |
| 130 | ` type parameter name ` | [parser/src/decl/generics.rs:56](../../compiler/parser/src/decl/generics.rs#L56) |
| 131 | `` type parameter name in `where` constraint `` | [parser/src/decl/generics.rs:142](../../compiler/parser/src/decl/generics.rs#L142) |
| 132 | ` typealias ` | [parser/src/decl/type_aliases.rs:51](../../compiler/parser/src/decl/type_aliases.rs#L51) |
| 133 | ` typealias name ` | [parser/src/decl/type_aliases.rs:53](../../compiler/parser/src/decl/type_aliases.rs#L53) |
| 134 | ` variant name ` | [parser/src/decl/enums.rs:257](../../compiler/parser/src/decl/enums.rs#L257)；[parser/src/pattern.rs:206](../../compiler/parser/src/pattern.rs#L206) |
| 135 | ` virtual argument or inline typed argument ` | [parser/src/decl/generics/requirements.rs:158](../../compiler/parser/src/decl/generics/requirements.rs#L158) |
| 136 | ` virtual parameter name ` | [parser/src/decl/generics/requirements.rs:20](../../compiler/parser/src/decl/generics/requirements.rs#L20) |
