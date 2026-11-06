#!/usr/bin/env python3
"""Structural checks for Rust files that clippy can't express. See docs/rules/rust.md.

Errors (exit 1):
  - a `mod.rs` file outside `tests/<name>/mod.rs` or `crates/<crate>/tests/<name>/mod.rs`
  - a lint attribute (`allow` / `expect`) with no reason: `reason = "..."` in src/, or at least a
    trailing `//` comment in tests/ and examples/
  - a module root in a crate's src/ holding anything but `mod`, `use`, `const` and its plugin
  - a top-level module in a crate's src/ that is a single file instead of `name.rs` beside `name/`
  - a `const` or `static` whose name is not SCREAMING_SNAKE_CASE

Run with no arguments to check every tracked file, or pass paths.
"""

import os
import re
import subprocess
import sys

LINT_ATTR_RE = re.compile(r"^\s*#!?\[\s*(allow|expect)\(")
VISIBILITY_RE = re.compile(r"^pub(\s*\([^)]*\))?\s*")
ITEM_HEAD_RE = re.compile(
    r"^(?:(?:unsafe|async|default|extern\s+\"\s*\")\s+)*"
    r"(mod|use|const|static|fn|struct|enum|union|trait|type|impl|macro_rules!|extern\s+crate)(?![\w])"
)
NAME_AFTER_RE = re.compile(r"^\s*([A-Za-z_]\w*)")
PLUGIN_IMPL_RE = re.compile(r"^impl\s*(?:<[^>]*>\s*)?(?:[\w:]*::)?Plugin\s+for\s+([A-Za-z_]\w*)")
SEMICOLON_ITEM_RE = re.compile(
    r"(?:pub(?:\s*\([^)]*\))?\s+)?(?:const(?!\s+(?:unsafe\s+|async\s+)*fn\b)|static|use|type)\b"
)
RAW_STRING_RE = re.compile(r'(?:b|c)?r(#*)"')
CHAR_LITERAL_RE = re.compile(r"'(\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'")
CONST_NAME_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static)\s+(?:mut\s+)?([A-Za-z_]\w*)\s*:")
SCREAMING_SNAKE_RE = re.compile(r"^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$")
RULES_LINK = "docs/rules/rust.md"


def tracked_rust_files() -> list[str]:
    out = subprocess.run(["git", "ls-files", "*.rs"], capture_output=True, text=True, check=True)
    return [f for f in out.stdout.splitlines() if f and not is_vendored(f)]


def is_vendored(path: str) -> bool:
    return normalized(path).startswith("vendor/")


def normalized(path: str) -> str:
    return os.path.relpath(path).replace(os.sep, "/")


def src_parts(path: str) -> list[str] | None:
    parts = normalized(path).split("/")
    if parts[0] == "src":
        return parts
    if len(parts) > 3 and parts[0] == "crates" and parts[2] == "src":
        return parts[2:]
    return None


def in_src(path: str) -> bool:
    return src_parts(path) is not None


def is_bin_root(path: str) -> bool:
    parts = src_parts(path)
    if parts is None:
        return False
    if parts == ["src", "main.rs"]:
        return True
    if len(parts) == 3 and parts[:2] == ["src", "bin"]:
        return True
    return len(parts) == 4 and parts[:2] == ["src", "bin"] and parts[3] == "main.rs"


def is_module_root(path: str) -> bool:
    parts = src_parts(path)
    if parts is None:
        return False
    if len(parts) == 2 or is_bin_root(path):
        return True
    return os.path.isdir(normalized(path)[: -len(".rs")])


def check_module_file_name(path: str) -> str | None:
    if os.path.basename(path) != "mod.rs":
        return None
    parts = normalized(path).split("/")
    if len(parts) == 3 and parts[0] == "tests":
        return None
    if len(parts) == 5 and parts[0] == "crates" and parts[2] == "tests":
        return None
    return f"{path}: `mod.rs` is not used here; name the file after its module (`foo.rs` beside `foo/`)"


def check_lint_attrs(path: str, lines: list[str]) -> list[str]:
    violations = []
    for i, line in enumerate(lines):
        if not LINT_ATTR_RE.match(line):
            continue
        attr = line
        j = i
        while attr.count("(") > attr.count(")") and j + 1 < len(lines):
            j += 1
            attr += lines[j]
        if "reason" in attr:
            continue
        if not in_src(path) and "//" in attr:
            continue
        violations.append(f"{path}:{i + 1}: lint attribute without `reason = \"...\"`")
    return violations


def check_constant_names(path: str, lines: list[str]) -> list[str]:
    violations = []
    for i, line in enumerate(lines):
        match = CONST_NAME_RE.match(line)
        if match and match.group(1) != "_" and not SCREAMING_SNAKE_RE.match(match.group(1)):
            violations.append(f"{path}:{i + 1}: constant `{match.group(1)}` is not SCREAMING_SNAKE_CASE")
    return violations


def blank_out(segment: str) -> str:
    return re.sub(r"[^\n]", " ", segment)


def strip_comments_and_literals(text: str) -> str:
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        prev_is_word = i > 0 and (text[i - 1].isalnum() or text[i - 1] == "_")
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end == -1 else end
            out.append(blank_out(text[i:end]))
            i = end
        elif text.startswith("/*", i):
            depth = 0
            j = i
            while j < n:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                    if depth == 0:
                        break
                else:
                    j += 1
            out.append(blank_out(text[i:j]))
            i = j
        elif not prev_is_word and (m := RAW_STRING_RE.match(text, i)):
            closing = '"' + m.group(1)
            end = text.find(closing, m.end())
            end = n if end == -1 else end + len(closing)
            out.append(blank_out(text[i:end]))
            i = end
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append(blank_out(text[i : j + 1]))
            i = j + 1
        elif c == "'" and (m := CHAR_LITERAL_RE.match(text, i)):
            out.append(blank_out(m.group(0)))
            i = m.end()
        else:
            out.append(c)
            i += 1
    return "".join(out)


def skip_attribute(text: str, i: int) -> int:
    j = text.find("[", i)
    depth = 0
    while j < len(text):
        if text[j] == "[":
            depth += 1
        elif text[j] == "]":
            depth -= 1
            if depth == 0:
                return j + 1
        j += 1
    return len(text)


def top_level_items(text: str) -> list[tuple[int, str]]:
    """(line, head) for every item at depth 0 of comment- and literal-free source."""
    items = []
    i = 0
    n = len(text)
    while i < n:
        if text[i].isspace():
            i += 1
            continue
        if text[i] == "#":
            i = skip_attribute(text, i)
            continue
        start = i
        depth = 0
        braced = False
        ends_at_semicolon = bool(SEMICOLON_ITEM_RE.match(text, start))
        while i < n:
            ch = text[i]
            if ch in "([{":
                depth += 1
                braced = braced or (ch == "{" and depth == 1)
            elif ch in ")]}":
                depth -= 1
                if depth == 0 and ch == "}" and braced and not ends_at_semicolon:
                    i += 1
                    break
            elif ch == ";" and depth == 0:
                i += 1
                break
            i += 1
        items.append((text.count("\n", 0, start) + 1, " ".join(text[start:i].split())))
    return items


def classify(head: str) -> tuple[str, str]:
    head = VISIBILITY_RE.sub("", head)
    m = ITEM_HEAD_RE.match(head)
    if not m:
        return "?", head[:40]
    keyword = " ".join(m.group(1).split())
    rest = head[m.end() :]
    if keyword == "const" and re.match(r"^\s*(unsafe\s+|async\s+)*fn\b", rest):
        keyword = "fn"
        rest = rest[rest.index("fn") + 2 :]
    name_match = NAME_AFTER_RE.match(rest)
    name = name_match.group(1) if name_match else ""
    if keyword == "mod" and "{" in head:
        keyword = "inline mod"
    return keyword, name


def check_module_root(path: str, text: str) -> list[str]:
    violations = []
    items = top_level_items(strip_comments_and_literals(text))
    plugin_types = set()
    for _, head in items:
        m = PLUGIN_IMPL_RE.match(VISIBILITY_RE.sub("", head))
        if m:
            plugin_types.add(m.group(1))
    bin_root = is_bin_root(path)
    for line, head in items:
        keyword, name = classify(head)
        allowed = (
            keyword in ("use", "extern crate", "mod", "const")
            or (keyword == "fn" and (name == "plugin" or (bin_root and name == "main")))
            or (keyword == "impl" and PLUGIN_IMPL_RE.match(VISIBILITY_RE.sub("", head)))
            or (keyword == "struct" and name in plugin_types)
        )
        if allowed:
            continue
        violations.append(
            f"{path}:{line}: module root holds `{keyword} {name}`; a root holds only `mod`, `use`, "
            f"`const` and its plugin, so move it to a leaf under {normalized(path)[: -len('.rs')]}/"
        )
    return violations


def check_top_level_directories(files: list[str]) -> list[str]:
    violations = []
    for path in files:
        parts = src_parts(path)
        if parts is None or len(parts) != 2 or parts[1] in ("lib.rs", "main.rs"):
            continue
        if not os.path.isdir(normalized(path)[: -len(".rs")]):
            violations.append(
                f"{path}: a top-level module is a directory; add {normalized(path)[: -len('.rs')]}/ "
                "and move the code into leaf files there"
            )
    return violations


def main() -> int:
    files = sys.argv[1:] or tracked_rust_files()
    files = [f for f in files if f.endswith(".rs") and os.path.isfile(f) and not is_vendored(f)]
    violations = check_top_level_directories(files)

    for path in files:
        module_violation = check_module_file_name(path)
        if module_violation:
            violations.append(module_violation)
        with open(path, encoding="utf-8") as f:
            text = f.read()
        violations += check_lint_attrs(path, text.splitlines())
        violations += check_constant_names(path, text.splitlines())
        if is_module_root(path):
            violations += check_module_root(path, text)

    for violation in violations:
        print(violation)
    if violations:
        print(f"\n{len(violations)} problem(s). See {RULES_LINK}.")
    return 1 if violations else 0


if __name__ == "__main__":
    sys.exit(main())
