"""Call hierarchy groups real calls and retains the original selected project."""


def check(peer, root):
    provider = root / "dependency/src/callHierarchy.scoop"
    library = (
        "package call_graph\n"
        "public fun route(flag: Boolean): Boolean = flag\n"
        "public fun route(count: Int): Int = count\n"
        "public interface Reader { public fun read(): Boolean }\n"
    )
    consumer = root / "src/callHierarchy.scoop"
    source = (
        "package call_graph_use\n"
        "import call_graph.route\n"
        "import call_graph.Reader\n"
        "fun invoke(reader: Reader): Boolean {\n"
        "    val first = route(true)\n"
        "    val second = route(first)\n"
        "    val number = route(1)\n"
        "    return reader.read()\n"
        "}\n"
        "fun recurse(flag: Boolean): Boolean { if (flag) { return true }; return recurse(true) }\n"
        "fun referenceOnly(): (Boolean) -> Boolean = ::route\n"
        "fun enclosing(flag: Boolean): Boolean {\n"
        "    fun local(): Boolean = route(flag)\n"
        "    return local()\n"
        "}\n"
    )
    for file, text in [(provider, library), (consumer, source)]:
        file.write_text(text)
        peer.notify(
            "textDocument/didOpen",
            {
                "textDocument": {
                    "uri": file.as_uri(),
                    "languageId": "scoop",
                    "version": 1,
                    "text": text,
                }
            },
        )

    def prepare(file, text, line, name):
        [node] = peer.request(
            "textDocument/prepareCallHierarchy",
            {
                "textDocument": {"uri": file.as_uri()},
                "position": {"line": line, "character": text.splitlines()[line].index(name)},
            },
        )
        return node

    def expand(node, method):
        return peer.request(f"callHierarchy/{method}Calls", {"item": node})

    def lines(relation):
        return [site["start"]["line"] for site in relation["fromRanges"]]

    invoke = prepare(consumer, source, 3, "invoke")
    outgoing = expand(invoke, "outgoing")
    assert [edge["to"]["name"] for edge in outgoing] == ["read", "route", "route"], outgoing
    method, boolean, integer = outgoing
    assert method["to"]["kind"] == 6 and lines(method) == [7], method
    assert boolean["to"]["uri"] == provider.as_uri() and lines(boolean) == [4, 5], boolean
    assert integer["to"]["selectionRange"]["start"]["line"] == 2 and lines(integer) == [6], integer
    assert prepare(consumer, source, 4, "route") == boolean["to"]
    via_reference = prepare(consumer, source, 10, "route")
    assert via_reference == boolean["to"]
    incoming = expand(boolean["to"], "incoming")
    assert {edge["from"]["name"]: lines(edge) for edge in incoming} == {
        "invoke": [4, 5],
        "local": [12],
    }, incoming
    [local_call] = expand(prepare(consumer, source, 11, "enclosing"), "outgoing")
    assert local_call["to"]["name"] == "local" and lines(local_call) == [13]
    [from_local] = expand(local_call["to"], "outgoing")
    assert from_local["to"] == boolean["to"] and lines(from_local) == [12]
    recurse = prepare(consumer, source, 9, "recurse")
    assert [edge["to"]["name"] for edge in expand(recurse, "outgoing")] == ["recurse"]
    assert [edge["from"]["name"] for edge in expand(recurse, "incoming")] == ["recurse"]

    updated = source.replace("val second = route(first)", "val second = first")
    peer.notify(
        "textDocument/didChange",
        {
            "textDocument": {"uri": consumer.as_uri(), "version": 2},
            "contentChanges": [{"text": updated}],
        },
    )
    pending = peer.send_request("callHierarchy/outgoingCalls", {"item": invoke})
    peer.response(pending, error_code=-32801)
    invoke = prepare(consumer, updated, 3, "invoke")
    assert [lines(edge) for edge in expand(invoke, "outgoing")] == [[7], [4], [6]]
    for file in [provider, consumer]:
        peer.notify("textDocument/didClose", {"textDocument": {"uri": file.as_uri()}})
