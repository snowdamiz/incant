"""Gate for the one granted mutation (handoff 0048): exit 0 only if the approval card's exact
arguments are a single rename of entity ...0051 in the Main scene to "Allowance Rename Confirmed".

Reads a winctl `ax dump` taken with "Exact arguments" expanded. Prints the parsed arguments.

Usage: python3 -I verify_patch.py <ax-dump.jsonl>
"""
import json
import sys

EXPECTED_OPS = [{
    "op": "rename_entity",
    "scene_id": "01M4PEJQNPKJKBZRM6FFAP8SXA",
    "entity_id": "00000000000000000000000051",
    "name": "Allowance Rename Confirmed",
}]


def arguments_text(nodes):
    for index, node in enumerate(nodes):
        if node.get("label") == "Exact arguments" or (node.get("title") == "Exact arguments" and node["role"] != "AXButton"):
            depth = node["d"]
            if node.get("value"):
                return node["value"]
            parts = []
            for child in nodes[index + 1:]:
                if child["d"] <= depth:
                    break
                if child["role"] == "AXStaticText" and child.get("value"):
                    parts.append(child["value"])
            return "".join(parts)
    return None


def main():
    nodes = [json.loads(line) for line in open(sys.argv[1]) if line.startswith("{")]
    text = arguments_text(nodes)
    if text is None:
        print("no exact-arguments block found")
        sys.exit(4)
    try:
        args = json.loads(text)
    except json.JSONDecodeError as error:
        print("arguments are not JSON:", error, repr(text[:400]))
        sys.exit(5)
    print(json.dumps(args, indent=1, ensure_ascii=False))
    ops = args.get("ops")
    if ops != EXPECTED_OPS:
        print("REFUSE: operations differ from the single granted rename")
        sys.exit(6)
    if set(args) - {"ops", "description", "expected_revision"}:
        print("REFUSE: unexpected argument fields")
        sys.exit(7)
    print("OK: exactly the granted rename")


if __name__ == "__main__":
    main()
