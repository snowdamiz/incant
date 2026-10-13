#!/usr/bin/env python3
"""Predict the exact libtest names of ignored GPU tests from Rust source alone.

CPU-only, standard library only; nothing is built or run. It walks each test
crate root through `mod` declarations (`#[path]`, inline modules and the cfg
gates of the default macOS test build) and lists every `#[test]` function that
carries `#[ignore]`. The guarded runner compares each frozen binary's actual
`--list --ignored` output with this prediction before any GPU child starts; a
difference refuses that suite instead of running an unbudgeted test.

  python3 -I -B listing.py [--json]
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

# Default macOS/aarch64 `cargo test -p incant_render -p incant_render_gpu_host`.
FEATURES = frozenset({"physics-deterministic"})
CFG = {"test": True, "unix": True, "windows": False, "debug_assertions": False,
       "target_os": "macos", "target_arch": "aarch64", "target_family": "unix",
       "target_env": "", "target_vendor": "apple"}

BINARIES = {
    "lib": "crates/incant_render/src/lib.rs",
    "model_gpu": "crates/incant_render/tests/model_gpu.rs",
    "gpu_host-game": "crates/incant_render_gpu_host/tests/game.rs",
    "gpu_host-ui": "crates/incant_render_gpu_host/tests/ui.rs",
}


class CfgError(Exception):
    pass


def _split_args(text):
    parts, depth, current = [], 0, ""
    for ch in text:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append(current.strip())
            current = ""
        else:
            current += ch
    if current.strip():
        parts.append(current.strip())
    return parts


def cfg(expr):
    """Evaluate the cfg predicates this crate graph uses; anything else refuses."""
    expr = expr.strip()
    match = re.fullmatch(r"(all|any|not)\s*\((.*)\)", expr, re.S)
    if match:
        args = [cfg(a) for a in _split_args(match.group(2))]
        if match.group(1) == "all":
            return all(args)
        if match.group(1) == "any":
            return any(args)
        if len(args) != 1:
            raise CfgError(f"not() takes one predicate: {expr}")
        return not args[0]
    match = re.fullmatch(r'(\w+)\s*=\s*"([^"]*)"', expr)
    if match:
        key, value = match.groups()
        if key == "feature":
            return value in FEATURES
        if key not in CFG:
            raise CfgError(f"unknown cfg key {key}")
        return CFG[key] == value
    if re.fullmatch(r"\w+", expr):
        if expr not in CFG:
            raise CfgError(f"unknown cfg flag {expr}")
        return bool(CFG[expr])
    raise CfgError(f"unsupported cfg expression {expr!r}")


def strip_comments(text):
    """Blank comments and literal contents; cfg strings survive in attributes
    because they are re-read from the raw text (see `_attribute_strings`)."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
            continue
        if text.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if text.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif text.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    if text[i] == "\n":
                        out.append("\n")
                    i += 1
            continue
        boundary = i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")
        raw = re.match(r'b?r(#*)"', text[i:i + 300]) if boundary else None
        if raw:
            end = '"' + raw.group(1)
            j = text.find(end, i + len(raw.group(0)))
            if j < 0:
                raise ValueError("unterminated raw string")
            body = text[i:j + len(end)]
            out.append('""' + "\n" * body.count("\n"))
            i = j + len(end)
            continue
        if c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            body = text[i:j + 1]
            # Keep attribute strings intact for cfg/path evaluation.
            out.append(body if _inside_attribute(out) else '""' + "\n" * body.count("\n"))
            i = j + 1
            continue
        if c == "'":
            literal = re.match(r"'(\\.[^']*|[^\\'])'", text[i:i + 12])
            if literal:
                out.append("' '")
                i += len(literal.group(0))
                continue
        out.append(c)
        i += 1
    return "".join(out)


def _inside_attribute(out):
    """True when the text emitted so far ends inside an open `#[` ... `]`."""
    tail = "".join(out[-400:])
    start = max(tail.rfind("#["), tail.rfind("#!["))
    if start < 0:
        return False
    level = 0
    for ch in tail[start:]:
        if ch == "[":
            level += 1
        elif ch == "]":
            level -= 1
    return level > 0


class Module:
    def __init__(self, file, path, mod_rs):
        self.file, self.path, self.mod_rs = file, path, mod_rs


def child_dir(module):
    """Directory of children declared without #[path] (mod-rs or path-loaded files
    own their directory; other files own `<dir>/<stem>/`)."""
    return module.file.parent if module.mod_rs else module.file.parent / module.file.stem


def resolve(directory, name):
    flat, nested = directory / f"{name}.rs", directory / name / "mod.rs"
    if flat.is_file() and nested.is_file():
        raise ValueError(f"ambiguous module {name} in {directory}")
    if flat.is_file():
        return flat, False
    if nested.is_file():
        return nested, True
    raise FileNotFoundError(f"module {name} not found under {directory}")


def _skip_brackets(text, start):
    """End of the bracketed attribute at `start`; strings kept inside attributes
    may contain brackets (e.g. error messages) and are skipped as units."""
    level, j = 0, start
    while True:
        if text[j] == '"':
            j += 1
            while text[j] != '"':
                j += 2 if text[j] == "\\" else 1
        elif text[j] == "[":
            level += 1
        elif text[j] == "]":
            level -= 1
            if level == 0:
                return j + 1
        j += 1


def _enabled(attrs):
    enabled = True
    for attr in attrs:
        match = re.match(r"#\[\s*cfg\s*\((.*)\)\s*\]$", attr, re.S)
        if match:
            enabled = enabled and cfg(match.group(1))
    return enabled


def _path(attrs):
    for attr in attrs:
        match = re.match(r'#\[\s*path\s*=\s*"([^"]+)"\s*\]$', attr, re.S)
        if match:
            return match.group(1)
    return None


def walk(module, tests):
    text = strip_comments(module.file.read_text())
    for inner in re.finditer(r"#!\[\s*cfg\s*\((.*?)\)\s*\]", text, re.S):
        if not cfg(inner.group(1)):
            return
    i, n, depth = 0, len(text), 0
    attrs, stack, prefix, disabled = [], [], [], None
    while i < n:
        if text.startswith("#![", i):
            i = _skip_brackets(text, i + 2)
            continue
        if text.startswith("#[", i):
            j = _skip_brackets(text, i + 1)
            attrs.append(text[i:j])
            i = j
            continue
        c = text[i]
        if c == "{":
            depth += 1
            i += 1
            attrs = []
            continue
        if c == "}":
            depth -= 1
            if stack and stack[-1] == depth:
                stack.pop()
                prefix.pop()
                if disabled is not None and depth <= disabled:
                    disabled = None
            i += 1
            attrs = []
            continue
        boundary = i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")
        match = re.match(r"(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(\w+)\s*([;{])", text[i:]) if boundary else None
        if match:
            name, end = match.groups()
            enabled, path = _enabled(attrs), _path(attrs)
            attrs = []
            i += match.end()
            if end == "{":
                stack.append(depth)
                depth += 1
                prefix.append(name)
                if not enabled and disabled is None:
                    disabled = depth - 1
                continue
            if not enabled or disabled is not None:
                continue
            if path is not None:
                if prefix:
                    raise ValueError(f"#[path] inside an inline module in {module.file}")
                target, mod_rs = (module.file.parent / path).resolve(), True
            else:
                base = child_dir(module)
                for part in prefix:
                    base = base / part
                target, mod_rs = resolve(base, name)
            walk(Module(target, module.path + prefix + [name], mod_rs), tests)
            continue
        match = re.match(r"(?:pub(?:\s*\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)",
                         text[i:]) if boundary else None
        if match:
            kinds = [m.group(1) for m in (re.match(r"#\[\s*(\w+)", a) for a in attrs) if m]
            if "test" in kinds and _enabled(attrs) and disabled is None:
                tests.append({"name": "::".join(module.path + prefix + [match.group(1)]),
                              "ignored": "ignore" in kinds,
                              "file": str(module.file.relative_to(ROOT))})
            attrs = []
            i += match.end()
            continue
        if c == ";":
            attrs = []
        i += 1


def predict():
    result = {}
    for binary, root in BINARIES.items():
        tests = []
        walk(Module((ROOT / root).resolve(), [], True), tests)
        names = [t["name"] for t in tests]
        if len(names) != len(set(names)):
            raise ValueError(f"duplicate test names in {binary}")
        result[binary] = {
            "root": root,
            "tests": len(tests),
            "ignored": sorted(t["name"] for t in tests if t["ignored"]),
            "ignored_files": sorted({t["file"] for t in tests if t["ignored"]}),
        }
    return result


def main():
    prediction = predict()
    if "--json" in sys.argv[1:]:
        print(json.dumps(prediction, indent=1))
        return
    total = 0
    for binary, entry in prediction.items():
        total += len(entry["ignored"])
        print(f"{binary}: {entry['tests']} tests, {len(entry['ignored'])} ignored")
    print(f"total ignored: {total}")


if __name__ == "__main__":
    main()
