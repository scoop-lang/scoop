use super::*;

fn assert_writes(source: &str) {
    // Each marker immediately precedes an identifier that should be diagnosed.
    let marker = "/*warn*/";
    let expected: Vec<_> = source
        .match_indices(marker)
        .map(|(offset, _)| {
            let start = offset + marker.len();
            let len = source[start..]
                .find(|ch: char| !ch.is_alphanumeric() && ch != '_')
                .unwrap();
            (start, start + len)
        })
        .collect();
    let output = lower_user_output(scoop_parser::parse(source).unwrap()).unwrap();
    let mut actual: Vec<_> = output
        .warnings
        .iter()
        .filter(|warning| warning.code == scoop_source::DiagnosticCode::W0801)
        .map(|warning| {
            let span = warning.span().unwrap();
            (span.start as usize, span.end as usize)
        })
        .collect();
    actual.sort_unstable();
    assert_eq!(actual, expected, "{source}");
}

#[test]
fn checks_each_reaching_write_and_keeps_underscore_bindings_usable() {
    assert_writes(
        r#"
fun main() {
    var /*warn*/x = 1
    x = 2
    println(x)
    /*warn*/x = 3
    var _keep = 4
    _keep = 5
    println(_keep)
    _keep = 6
    val _ = _keep
    val (/*warn*/unused, _) = (1, 2)
}
"#,
    );
}

#[test]
fn follows_branches_shadowing_and_loop_edges() {
    assert_writes(
        r#"
fun run(flag: Boolean) {
    var x = 0
    if (flag) { x = 1 }
    while (flag) {
        println(x)
        x = 2
        if (flag) { continue } else { break }
    }
    println(x)
    if (flag) { val /*warn*/x = 3 }
    /*warn*/x = 4
}
fun main() {}
"#,
    );
}

#[test]
fn checks_writes_from_updates_and_reads_in_cleanup_and_closures() {
    assert_writes(
        r#"
fun demo(flag: Boolean) {
    var count = 0
    count += 1;
    ++/*warn*/count
    val captured = 8
    val f: (Int) -> Int = { /*warn*/unused -> captured }
    var last = f(1)
    try {
        if (flag) { return }
        last = 9
    } finally { println(last) }
}
fun main() {}
"#,
    );
}

#[test]
fn checks_pattern_bindings_catches_and_explicit_lambda_parameters() {
    assert_writes(
        r#"
fun f(opt: Int?, values: Iterable<Int>) {
    for (/*warn*/item in values) { println(1) }
    when (opt) {
        case Some(val /*warn*/payload) -> println(1)
        case None -> println(0)
    }
    try { println(1) } catch (/*warn*/problem: Exception) { println(0) }
    val implicit: (Int) -> Int = { 1 }
    val explicit: (Int) -> Int = { _ -> 1 }
    println(implicit(explicit(1)))
}
fun main() {}
"#,
    );
}

#[test]
fn overwrites_dead_paths_and_reads_before_exceptional_assignment() {
    assert_writes(
        r#"
fun next(): Int = 2
fun choose(flag: Boolean) {
    var /*warn*/slot = 1
    if (flag) { slot = 2 } else { slot = 3 }
    try {
        slot = next()
    } catch (_: Exception) { println(slot) }
    println(slot)
    /*warn*/slot = 4
    return
    println(slot)
}
fun main() {}
"#,
    );
}

#[test]
fn address_exposure_keeps_the_reaching_and_subsequent_writes() {
    assert_writes(
        r#"
fun main() {
    var /*warn*/number = 0
    number = 1
    @Unsafe {
        val p = addressOf(number)
        number = 2
        println(p.load())
    }
}
"#,
    );
}

#[test]
fn bound_receivers_and_local_function_captures_use_the_current_value() {
    assert_writes(
        r#"
class Reader(val value: Int) { fun read(): Int = value }
fun main() {
    var /*warn*/reader = Reader(0)
    reader = Reader(1)
    val bound = reader::read
    /*warn*/reader = Reader(2)
    val captured = 3
    fun local(): Int = captured
    println(bound() + local())
}
"#,
    );
}

#[test]
fn diagnoses_a_generic_definition_once_and_marks_constructor_and_default_bindings() {
    assert_writes(
        r#"
fun <T> keep(input: T): T {
    val /*warn*/discarded = input
    return input
}
class Box(val n: Int) {
    init { val /*warn*/inside = n }
}
fun defaulted(n: Int = if (true) { val /*warn*/inside = 1; 2 } else { 3 }): Int = n
fun main() {
    println(keep(1))
    println(keep("hello"))
    println(Box(defaulted()).n)
    println(defaulted())
}
"#,
    );
}
