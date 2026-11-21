mod support;
use support::checked;

#[test]
fn declarations_and_blocks() {
    assert_eq!(
        checked("fun add(a:Int,b:Int):Int {return a+b}"),
        "fun add(a: Int, b: Int): Int {\n    return a + b\n}\n"
    );
    assert_eq!(
        checked("package example\nimport a.B\nimport c.D\nfun f(){}\nfun g(){}"),
        "package example\n\nimport a.B\nimport c.D\n\nfun f() {}\n\nfun g() {}\n"
    );
    assert_eq!(checked(" \t\r\n"), "");
}

#[test]
fn kotlin_operator_and_colon_spacing() {
    assert_eq!(
        checked("fun f(n:Int){for(i in 0 .. n){ take(i ..< n) }\nval valid=1 in 0 .. n && n>0}"),
        "fun f(n: Int) {\n    for (i in 0..n) {\n        take(i..<n)\n    }\n    val valid = 1 in 0..n && n > 0\n}\n"
    );
    assert_eq!(
        checked(
            "class C<T:Bound>(val x:T):Base(), Face where T:Other {\nconstructor(x:T,y:Int):this(x){}\nval a=object:Face{}\nfun f(cb:(Int)->Int):T=x\n}"
        ),
        "class C<T : Bound>(val x: T) : Base(), Face where T : Other {\n    constructor(x: T, y: Int) : this(x) {}\n    val a = object : Face {}\n    fun f(cb: (Int) -> Int): T = x\n}\n"
    );
    let formatted = checked(
        "fun f() = veryLongValueNameFromTheFirstInput in 0 .. veryLongLimitFromTheSecondInput && anotherVeryLongCondition",
    );
    assert!(formatted.contains("0..veryLongLimitFromTheSecondInput"));
}

#[test]
fn annotation_spacing_and_groups() {
    for (source, expected) in [
        ("@Test  fun f(){}", "@Test fun f() {}\n"),
        ("@Test\nfun f(){}", "@Test\nfun f() {}\n"),
        (
            "@First @Second public fun f()=1",
            "@First @Second\npublic fun f() = 1\n",
        ),
        (
            "@First @With(1) @Second fun f(){}",
            "@First\n@With(1)\n@Second\nfun f() {}\n",
        ),
        (
            "class C(@field : Tag val x:Int){ @get: Tag val y=1 }",
            "class C(@field:Tag val x: Int) {\n    @get:Tag val y = 1\n}\n",
        ),
    ] {
        assert_eq!(checked(source), expected);
    }
}

#[test]
fn whitespace_sensitive_expressions() {
    for source in [
        "fun f(){ return\n value\n call\n (x)\n array\n [1,2]\n array[0] }",
        "fun f(){ val x=- -1; val y=+ +2; val z=!!flag; a++; --a; val v=a!!; val b=v !is T; val c=v as? T }",
        "fun f(){ outer@ while(true){return@outer value} }",
        "fun f(){ if(a) if(b) foo() else bar()\n do call() while(ready) }",
        "fun <T> f(a: Box<T?>): Box<Box<T>> = call<T>(a)",
        "fun f(){ val x=thing.{a:1,b:2}; val y=items[0,1]; call(*items) { x -> x+1 } }",
        "fun f(){ when(x) { 1 -> yes()\n 2 -> no()\n else -> other() } }",
        "fun f(){ when(x) { case Some(val x)->x\ncase val whole->whole } }",
        "fun f(){ when(x) { case Box{val item,other:val next}->item\ncase (val x,_)->x } }",
    ] {
        checked(source);
    }
}

#[test]
fn comments_strings_and_newlines_are_verbatim() {
    for source in [
        "#!/usr/bin/env scoop\n/// doc\n@Tag public fun f(){ // tail\n val s=\"a  b\"\n /** unattached */ s\n }\n/** eof */",
        "fun f(){ val s=\"\"\"a\r\n  b\n c\"\"\"; val t=f\"text ${1+2} tail\" }",
        "fun f() = f\"${a}${b}\"",
        "/** comment\r\n  * raw\n*/\nfun f(){return 1}\n",
        "fun f(){\r\nreturn 1\r\n}\r\n",
        "fun f(){\nreturn 1\r\n}\n",
    ] {
        checked(source);
    }
}

#[test]
fn lists_break_at_unicode_scalar_width() {
    let name = "参数".repeat(10);
    let source = format!("fun f({name}: Int, {name}b: Int, {name}c: Int) {{}}");
    let formatted = checked(&source);
    assert_eq!(formatted.lines().count(), 1, "{formatted}");
    let source = format!("fun longer({name}: Int, {name}b: Int, {name}c: Int, {name}d: Int) {{}}");
    let formatted = checked(&source);
    assert!(formatted.contains("(\n    "), "{formatted}");
    assert!(formatted.contains("\n) {}"), "{formatted}");
}

#[test]
fn multiline_parameters_and_argument_groups() {
    assert_eq!(
        checked("fun f(\na:Int,b:Int\n){\nplot(x=10,y=10,\nwidth=100,height=100,\nfill=true)\n}"),
        "fun f(\n    a: Int,\n    b: Int\n) {\n    plot(\n        x = 10, y = 10,\n        width = 100, height = 100,\n        fill = true\n    )\n}\n"
    );
    assert_eq!(
        checked(
            "fun f() {\ncall(\nfirstCoordinateWithALongDescriptiveName, secondCoordinateWithAnEvenLongerDescriptiveNameThanTheFirst,\nthird)\n}"
        ),
        "fun f() {\n    call(\n        firstCoordinateWithALongDescriptiveName,\n        secondCoordinateWithAnEvenLongerDescriptiveNameThanTheFirst,\n        third\n    )\n}\n"
    );
}

#[test]
fn multiline_supertypes_and_constraints() {
    assert_eq!(
        checked("class C : Base(),\nFirst,\nSecond {}"),
        "class C : Base(),\n    First,\n    Second {}\n"
    );
    assert_eq!(
        checked(
            "class Implementor : FirstTypeNameThatIsVeryLongIndeed(), SecondTypeNameThatIsJustAsLongAsFirstType, ThirdTypeNameThatIsQuiteLongToo {}"
        ),
        "class Implementor :\n    FirstTypeNameThatIsVeryLongIndeed(),\n    SecondTypeNameThatIsJustAsLongAsFirstType,\n    ThirdTypeNameThatIsQuiteLongToo {}\n"
    );
    assert_eq!(
        checked("fun <T> f(x:T):T where T:First,\nT:Second=x"),
        "fun <T> f(x: T): T\n    where T : First,\n          T : Second = x\n"
    );
    assert_eq!(
        checked("class Box<T>\n: First, Second\nwhere T:Reference {}"),
        "class Box<T> : First, Second\n    where T : Reference {}\n"
    );
}

#[test]
fn callable_bodies_annotations_and_control_layout() {
    for (source, expected) in [
        ("fun f()=apply {x->x+1}", "fun f() = apply { x -> x + 1 }\n"),
        (
            "fun f(){if(x)call() else other()}",
            "fun f() {\n    if (x) call() else other()\n}\n",
        ),
        (
            "fun f(){when(x){1->a()\nelse->b()}}",
            "fun f() {\n    when (x) {\n        1 -> a()\n        else -> b()\n    }\n}\n",
        ),
        ("fun f(){return\n  1}", "fun f() {\n    return\n    1\n}\n"),
        ("fun f() =\n 1+2", "fun f() = 1 + 2\n"),
    ] {
        assert_eq!(checked(source), expected);
    }
}

#[test]
fn kotlin_control_braces_and_clauses() {
    assert_eq!(
        checked(
            "fun f()\n{\nif (ok)\n{ first() }\nelse\nif (ready)\n{ second() }\nelse\n{ third() }\ntry\n{ first() }\ncatch(e: Error)\n{ second() }\nfinally\n{ done() }\ndo\n{ first() }\nwhile (ok)\n}"
        ),
        "fun f() {\n    if (ok) {\n        first()\n    } else if (ready) {\n        second()\n    } else {\n        third()\n    }\n    try {\n        first()\n    } catch (e: Error) {\n        second()\n    } finally {\n        done()\n    }\n    do {\n        first()\n    } while (ok)\n}\n"
    );
    assert_eq!(
        checked("fun f() {\nif (ok) // wait\nact()\nelse stop()\n}\nfun g() { catch(1) }"),
        "fun f() {\n    if (ok) // wait\n        act()\n    else\n        stop()\n}\n\nfun g() {\n    catch(1)\n}\n"
    );
}

#[test]
fn kotlin_short_if_and_multiline_condition() {
    assert_eq!(
        checked("fun choose(ok:Boolean)=if (ok) 1 else 2"),
        "fun choose(ok: Boolean) = if (ok) 1 else 2\n"
    );
    assert_eq!(
        checked(
            "fun f() { if (firstLongConditionThatMustBeChecked && secondLongConditionThatMustBeChecked && thirdLongConditionThatMustBeChecked) { work() } }"
        ),
        "fun f() {\n    if (firstLongConditionThatMustBeChecked &&\n        secondLongConditionThatMustBeChecked &&\n        thirdLongConditionThatMustBeChecked\n    ) {\n        work()\n    }\n}\n"
    );
}

#[test]
fn kotlin_when_branch_continuations() {
    assert_eq!(
        checked("fun f(){when(x){ first -> start()\nsecond ->\n{ work() }\nelse ->\n finish()\n}}"),
        "fun f() {\n    when (x) {\n        first -> start()\n\n        second -> {\n            work()\n        }\n\n        else ->\n            finish()\n    }\n}\n"
    );
}

#[test]
fn long_expressions_and_comments_keep_grammar_boundaries() {
    for source in [
        "fun f() = veryLongValueName + anotherLongValueName + yetAnotherLongValueName + finalLongValueName + lastLongValueName",
        "fun f() = items and (if (ready) 63 else 7)",
        "fun f(a: Int, // parameter\n b: Int) { call(a, // argument\n b) }",
        "class C { fun first(){}\nfun second(){} }",
        "fun f(){ call(1,2)\nval tuple=(1,2,)\nval array=[1,2] }",
        "fun f() = apply { x -> first(x); second(x) }",
        "fun f() = apply { x -> ({ y -> x + y }) }",
        "/// first\n/** second */\nfun f(/** parameter */ value: Int) {}\n/** unbound */",
    ] {
        checked(source);
    }
    assert_eq!(
        checked("fun f(){\r\nreturn 1\r\n}\r\n"),
        "fun f() {\r\n    return 1\r\n}\r\n"
    );
}

#[test]
fn block_imports_stay_in_place_and_preserve_statement_separators() {
    for source in [
        "fun f(){import api.pick as selected;selected(); {import api.*;pick()}; selected()}",
        "fun f()=apply { x -> import api.*; x.convert() }",
        "fun f(){call();\n// local\nimport api.pick\npick()}",
    ] {
        let formatted = checked(source);
        assert!(formatted.starts_with("fun f("), "{formatted}");
        assert!(formatted.contains("import api."), "{formatted}");
    }
}
