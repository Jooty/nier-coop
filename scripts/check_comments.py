#!/usr/bin/env python3
"""Comment and item-order checks for Rust files. See docs/rules/rust.md.

Errors (exit 1), every file:
  - a comment directly above a `const`/`static` (rename the constant instead)
  - a comment directly above a private `fn` (rename it, or put a short comment inside the body)
  - top-level items out of order: `use`/`mod`, then `const`/`static`, then types; in src/ also
    then `impl` blocks, then free functions

Errors (exit 1), src/ only:
  - a module header (`//!`) longer than two lines
  - a comment block longer than three lines
  - an em dash in a comment
  - a doc comment that restates the name it sits on
  - a struct or enum whose every field or variant (three or more) carries a doc comment, at least
    three of them longer than a short unit note, except a `ShaderType` uniform

Run with no arguments to check every tracked file, or pass paths.
"""

import os
import re
import subprocess
import sys

MAX_HEADER_LINES = 2
MAX_BLOCK_LINES = 3
MIN_SWARM_MEMBERS = 3
MAX_RESTATING_WORDS = 6
MAX_SHORT_DOC_WORDS = 6
RULES_LINK = "docs/rules/rust.md"

COMMENT_RE = re.compile(r"^\s*//")
HEADER_RE = re.compile(r"^\s*//!")
DOC_COMMENT_RE = re.compile(r"^\s*///(?!/)")
CONST_DECL_RE = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?(const|static)\s+([A-Z_][A-Z0-9_]*)\s*:")
PRIVATE_FN_RE = re.compile(r"^\s*((const|async|unsafe)\s+)*fn\s+\w+")
TRAIT_BLOCK_RE = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?(unsafe\s+)?(trait\b|impl\b.*\bfor\b)")
ATTRIBUTE_RE = re.compile(r"^\s*#")
ITEM_RE = re.compile(
    r"^(pub(\([^)]*\))?\s+)?((const|async|unsafe)\s+)*(use|mod|const|static|struct|enum|trait|type|impl|fn)\b"
)
TYPE_NAME_RE = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?(struct|enum)\s+([A-Za-z_]\w*)")
CONST_TYPE_RE = re.compile(r":\s*(.+?)\s*=")
DECL_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(?:const|async|unsafe)\s+)*"
    r"(?:(?:fn|struct|enum|trait|type|const|static|mod)\s+)?([A-Za-z_]\w*)\s*([:({<=;,]|$)"
)
FIELD_TYPE_RE = re.compile(r":\s*(?:&(?:'\w+\s+)?(?:mut\s+)?)?([A-Za-z_][\w:]*)")
AGGREGATE_OPEN_RE = re.compile(r"^(\s*)(pub(\([^)]*\))?\s+)?(enum|struct)\s+(\w+)[^;(]*\{\s*$")
MEMBER_RE = re.compile(r"^(pub(\([^)]*\))?\s+)?[A-Za-z_]\w*")
SENTENCE_BREAK_RE = re.compile(r"[.;:!?]\s+\S|\s-\s")

ITEM_RANK = {"use": 0, "mod": 0, "const": 1, "static": 1, "struct": 2, "enum": 2, "trait": 2, "type": 2}
SRC_ITEM_RANK = ITEM_RANK | {"impl": 3, "fn": 4}
RANK_NAMES = ["use/mod", "const/static", "types", "impls", "free fns"]
STOPWORDS = {
    "a", "an", "the", "how", "what", "is", "are", "of", "to", "in", "on", "for", "its", "this",
    "it", "that", "which", "and", "or", "be", "by", "as", "at", "with", "from", "one", "each",
    "every", "any", "all", "if", "when", "whether", "has", "have", "not", "into", "their", "them",
    "set", "get", "gets", "sets", "returns", "current", "value", "given", "used", "s",
}


def tracked_rust_files() -> list[str]:
    out = subprocess.run(["git", "ls-files", "*.rs"], capture_output=True, text=True, check=True)
    return [f for f in out.stdout.splitlines() if f and not is_vendored(f)]


def normalized(path: str) -> str:
    return os.path.relpath(path).replace(os.sep, "/")


def is_vendored(path: str) -> bool:
    return normalized(path).startswith("vendor/")


def in_src(path: str) -> bool:
    parts = normalized(path).split("/")
    return parts[0] == "src" or (len(parts) > 3 and parts[0] == "crates" and parts[2] == "src")


def brace_delta(line: str) -> int:
    return line.count("{") - line.count("}")


def comment_body(line: str) -> str:
    return re.sub(r"^\s*//[/!]?", "", line).strip()


def check_const_comments(lines: list[str]) -> list[tuple[int, str]]:
    violations = []
    for i, line in enumerate(lines):
        if not COMMENT_RE.match(line) or (i > 0 and COMMENT_RE.match(lines[i - 1])):
            continue
        j = i + 1
        while j < len(lines) and COMMENT_RE.match(lines[j]):
            j += 1
        if j < len(lines) and CONST_DECL_RE.match(lines[j]):
            violations.append((i + 1, f"comment above a const/static; rename it instead: {lines[j].strip()}"))
    return violations


def check_private_fn_comments(lines: list[str]) -> list[tuple[int, str]]:
    violations = []
    block_is_trait: list[bool] = []
    for i, line in enumerate(lines):
        in_trait = bool(block_is_trait) and block_is_trait[-1]
        if not in_trait and PRIVATE_FN_RE.match(line):
            j = i - 1
            while j >= 0 and ATTRIBUTE_RE.match(lines[j]):
                j -= 1
            if j >= 0 and COMMENT_RE.match(lines[j]):
                violations.append((j + 1, f"comment above a private fn; rename it, or comment inside the body: {line.strip()}"))
        if not COMMENT_RE.match(line):
            is_trait = bool(TRAIT_BLOCK_RE.match(line))
            for _ in range(max(0, brace_delta(line))):
                block_is_trait.append(is_trait)
            for _ in range(max(0, -brace_delta(line))):
                if block_is_trait:
                    block_is_trait.pop()
    return violations


def local_type_names(lines: list[str]) -> set[str]:
    names = set()
    depth = 0
    for line in lines:
        if depth == 0 and (match := TYPE_NAME_RE.match(line)):
            names.add(match.group(4))
        depth += brace_delta(line)
    return names


def is_instance_of_local_type(line: str, local_types: set[str]) -> bool:
    type_match = CONST_TYPE_RE.search(line)
    if not type_match:
        return False
    return any(t in local_types for t in re.findall(r"[A-Za-z_]\w*", type_match.group(1)))


def check_item_order(lines: list[str], ranks: dict[str, int]) -> list[tuple[int, str]]:
    violations = []
    local_types = local_type_names(lines)
    depth = 0
    max_rank_seen = -1
    for lineno, line in enumerate(lines, start=1):
        current_depth = depth
        depth += brace_delta(line)
        if current_depth != 0 or not (match := ITEM_RE.match(line)):
            continue
        keyword = match.group(5)
        if keyword not in ranks:
            continue
        rank = ranks[keyword]
        if rank < max_rank_seen:
            if keyword in ("const", "static") and is_instance_of_local_type(line, local_types):
                continue
            violations.append(
                (
                    lineno,
                    f"`{keyword}` after {RANK_NAMES[max_rank_seen]}; order is "
                    + ", then ".join(RANK_NAMES[: max(ranks.values()) + 1]),
                )
            )
        max_rank_seen = max(max_rank_seen, rank)
    return violations


def check_header(lines: list[str]) -> list[tuple[int, str]]:
    header = [i for i, line in enumerate(lines) if HEADER_RE.match(line)]
    if len(header) <= MAX_HEADER_LINES:
        return []
    return [
        (
            header[0] + 1,
            f"module header is {len(header)} lines; at most {MAX_HEADER_LINES}, or none "
            "(move the why to docs/<module path>/)",
        )
    ]


def check_block_length(lines: list[str]) -> list[tuple[int, str]]:
    violations = []
    i = 0
    while i < len(lines):
        if not COMMENT_RE.match(lines[i]) or HEADER_RE.match(lines[i]):
            i += 1
            continue
        j = i
        while j < len(lines) and COMMENT_RE.match(lines[j]) and not HEADER_RE.match(lines[j]):
            j += 1
        if j - i > MAX_BLOCK_LINES:
            violations.append(
                (i + 1, f"comment block is {j - i} lines; at most {MAX_BLOCK_LINES}")
            )
        i = j
    return violations


def check_em_dashes(lines: list[str]) -> list[tuple[int, str]]:
    return [
        (i + 1, "em dash in a comment")
        for i, line in enumerate(lines)
        if "—" in line[line.find("//") :] and "//" in line
    ]


def identifier_words(identifier: str) -> list[str]:
    snake = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", identifier)
    return [w for w in snake.lower().split("_") if w and w not in STOPWORDS]


def content_words(text: str) -> list[str]:
    raw = text.lower().replace("_", " ").replace("-", " ").split()
    words = (w.strip(",.;:()`'\"[]!?*") for w in raw)
    return [w for w in words if w and w not in STOPWORDS]


def same_word(a: str, b: str) -> bool:
    if len(a) < 4 or len(b) < 4:
        return a == b
    return a[:4] == b[:4]


def opens_by_restating(name_words: list[str], text: str) -> bool:
    first_clause = content_words(re.split(r"[.\-,;:]", text, maxsplit=1)[0])
    if len(name_words) < 2 or not first_clause or len(first_clause) > 14:
        return False
    return all(any(same_word(n, t) for t in first_clause) for n in name_words)


def only_restates(name_words: list[str], type_words: list[str], text: str) -> bool:
    words = content_words(text)
    if not name_words or not words or len(words) > MAX_RESTATING_WORDS:
        return False
    if SENTENCE_BREAK_RE.search(text.strip()):
        return False
    known = name_words + type_words
    covered = [w for w in words if any(same_word(w, k) for k in known)]
    names_all_present = all(any(same_word(n, w) for w in words) for n in name_words)
    return names_all_present and len(covered) / len(words) >= 0.6


def check_restating(lines: list[str]) -> list[tuple[int, str]]:
    violations = []
    i = 0
    while i < len(lines):
        if not DOC_COMMENT_RE.match(lines[i]):
            i += 1
            continue
        j = i
        while j < len(lines) and DOC_COMMENT_RE.match(lines[j]):
            j += 1
        k = j
        while k < len(lines) and lines[k].strip().startswith("#["):
            k += 1
        if k < len(lines) and (decl := DECL_RE.match(lines[k])):
            identifier = decl.group(1)
            text = " ".join(comment_body(line) for line in lines[i:j])
            is_field = decl.group(2) == ":" and not CONST_DECL_RE.match(lines[k])
            type_match = FIELD_TYPE_RE.search(lines[k]) if is_field else None
            type_words = identifier_words(type_match.group(1).split("::")[-1]) if type_match else []
            name_words = identifier_words(identifier)
            if (is_field and opens_by_restating(name_words, text)) or only_restates(
                name_words, type_words, text
            ):
                violations.append(
                    (i + 1, f"doc comment restates `{identifier}`; delete it, or rename so it can go")
                )
        i = j
    return violations


def is_gpu_layout(lines: list[str], index: int) -> bool:
    j = index - 1
    while j >= 0 and (lines[j].strip().startswith("#[") or DOC_COMMENT_RE.match(lines[j])):
        if "ShaderType" in lines[j]:
            return True
        j -= 1
    return False


def check_swarms(lines: list[str]) -> list[tuple[int, str]]:
    violations = []
    for i, line in enumerate(lines):
        opened = AGGREGATE_OPEN_RE.match(line)
        if not opened or is_gpu_layout(lines, i):
            continue
        indent = len(opened.group(1))
        members = documented = substantial = depth = 0
        doc_lines = []
        for inner in lines[i + 1 :]:
            stripped = inner.strip()
            if stripped.startswith("}") and len(inner) - len(inner.lstrip()) == indent:
                break
            if depth == 0:
                if DOC_COMMENT_RE.match(inner):
                    doc_lines.append(comment_body(inner))
                    continue
                if not stripped or stripped.startswith("#[") or stripped.startswith("//"):
                    continue
                if MEMBER_RE.match(stripped):
                    members += 1
                    documented += bool(doc_lines)
                    words = len(" ".join(doc_lines).split())
                    substantial += len(doc_lines) > 1 or words > MAX_SHORT_DOC_WORDS
                doc_lines = []
            depth = max(0, depth + stripped.count("{") + stripped.count("(") - stripped.count("}") - stripped.count(")"))
        if members >= MIN_SWARM_MEMBERS and documented == members and substantial >= MIN_SWARM_MEMBERS:
            violations.append(
                (
                    i + 1,
                    f"every member of `{opened.group(5)}` carries a doc comment; names carry the "
                    "meaning, so keep only the ones that explain something a name cannot",
                )
            )
    return violations


def check_file(path: str, lines: list[str]) -> list[tuple[int, str]]:
    src = in_src(path)
    violations = check_const_comments(lines)
    violations += check_private_fn_comments(lines)
    violations += check_item_order(lines, SRC_ITEM_RANK if src else ITEM_RANK)
    if src:
        violations += check_header(lines)
        violations += check_block_length(lines)
        violations += check_em_dashes(lines)
        violations += check_restating(lines)
        violations += check_swarms(lines)
    return sorted(violations)


def main() -> int:
    files = sys.argv[1:] or tracked_rust_files()
    count = 0
    for path in files:
        if not path.endswith(".rs") or not os.path.isfile(path) or is_vendored(path):
            continue
        with open(path, encoding="utf-8") as f:
            lines = f.read().splitlines()
        for line_no, message in check_file(path, lines):
            count += 1
            print(f"{path}:{line_no}: {message}")
    if count:
        print(f"\n{count} problem(s). See {RULES_LINK}.")
    return 1 if count else 0


if __name__ == "__main__":
    sys.exit(main())
