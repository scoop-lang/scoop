"""Graph constructors, accessors, operators, defaults and function values."""

from rename_support import RenameWorkspace, manifest


def check(peer, root):
    project = root / "call-hierarchy-operations"
    manifest(project, "call-hierarchy-operations")
    file = "src/operations.scoop"
    text = (
        "package graph\n"
        "fun mark(number: Int): Int = number\n"
        "class Box(public val initial: Int = mark(1)) {\n"
        "    init { mark(2) }\n"
        "    public val computed: Int get() = mark(initial)\n"
        "}\n"
        "class Quantity { operator fun plus(other: Quantity): Quantity = other }\n"
        "fun useBox(): Int { val box = Box(); return box.computed }\n"
        "fun defaulted(number: Int = mark(3)): Int = number\n"
        "fun useDefault(): Int = defaulted()\n"
        "fun applyOp(a: Quantity, b: Quantity): Quantity = a + b\n"
        "private fun stable(number: Int): Int = number\n"
        "fun literalCall(): Int = (::stable)(1)\n"
        "fun dynamic(cb: (Int) -> Int): Int = cb(4)\n"
        "fun referenceOnly(): (Int) -> Int = ::stable\n"
        "var saved: Int = 0\n"
        "get() = mark(field)\n"
        "set(value) { field = mark(value) }\n"
        "fun assign() { saved = 1 }\n"
        "class Alternate(val value: Int) {\n"
        "    constructor(flag: Boolean) : this(mark(5)) { mark(6) }\n"
        "}\n"
        "fun secondary(): Alternate = Alternate(true)\n"
    )
    case = RenameWorkspace(peer, project, {file: text})
    case.clean()

    def prepare(anchor, token):
        [node] = peer.request(
            "textDocument/prepareCallHierarchy",
            {
                "textDocument": {"uri": case.paths[file].as_uri()},
                "position": case.position(file, anchor, token),
            },
        )
        return node

    def sites(node, direction):
        key = {"incoming": "from", "outgoing": "to"}[direction]
        edges = peer.request(f"callHierarchy/{direction}Calls", {"item": node})
        return {
            (edge[key]["name"], edge[key]["kind"]): [
                site["start"]["line"] for site in edge["fromRanges"]
            ]
            for edge in edges
        }

    box = prepare("box = Box()", "Box")
    assert box["kind"] == 9 and box == prepare("class Box", "Box"), box
    assert sites(box, "outgoing") == {("mark", 12): [2, 3]}
    assert sites(box, "incoming") == {("useBox", 12): [7]}
    assert sites(prepare("fun useBox", "useBox"), "outgoing") == {
        ("Box", 9): [7],
        ("computed", 7): [7],
    }
    computed = prepare("box.computed", "computed")
    assert computed == prepare("val computed", "computed")
    assert sites(computed, "outgoing") == {("mark", 12): [4]}
    assert sites(computed, "incoming") == {("useBox", 12): [7]}
    assert sites(prepare("fun mark", "mark"), "incoming") == {
        ("Box", 9): [2, 3],
        ("computed", 7): [4],
        ("defaulted", 12): [8],
        ("saved", 7): [16, 17],
        ("Alternate", 9): [20, 20],
    }
    assert sites(prepare("fun defaulted", "defaulted"), "outgoing") == {("mark", 12): [8]}
    plus = prepare("a + b", "+")
    assert plus["name"] == "plus" and sites(plus, "incoming") == {("applyOp", 12): [10]}
    assert sites(prepare("fun applyOp", "applyOp"), "outgoing") == {("plus", 6): [10]}

    stable = prepare("fun stable", "stable")
    assert sites(stable, "incoming") == {("literalCall", 12): [12]}
    assert sites(prepare("fun literalCall", "literalCall"), "outgoing") == {("stable", 12): [12]}
    callback = prepare("= cb(4)", "cb")
    assert callback["kind"] == 13 and callback["selectionRange"]["start"] == case.position(
        file, "dynamic(cb:", "cb"
    )
    assert sites(callback, "incoming") == {("dynamic", 12): [13]}
    assert sites(callback, "outgoing") == {}

    saved = prepare("saved = 1", "saved")
    assert saved == prepare("var saved", "saved")
    assert sites(saved, "outgoing") == {("mark", 12): [16, 17]}
    assert sites(saved, "incoming") == {("assign", 12): [18]}
    selected = prepare("= Alternate(true)", "Alternate")
    assert selected["kind"] == 9
    assert selected["selectionRange"]["start"]["line"] == 20
    assert sites(selected, "outgoing") == {("Alternate", 9): [20], ("mark", 12): [20, 20]}
    assert sites(selected, "incoming") == {("secondary", 12): [22]}
    case.finish()
