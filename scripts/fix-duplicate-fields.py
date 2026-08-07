#!/usr/bin/env python3
"""Resolve openapi-generator Rust field-name collisions.

Some specs declare two JSON properties that differ only in case (e.g. `payerID`
and `payerId`). The rust generator snake_cases both to the same field (`payer_id`),
producing a struct that can't compile. This pass renames the earlier (deprecated)
occurrence(s) to `<name>_legacy<n>`, leaving the last occurrence with the clean name.
The `#[serde(rename = "...")]` attribute is untouched, so the wire format is unchanged.

Idempotent: a tree with no duplicate Rust field names is left exactly as-is.

Usage: fix-duplicate-fields.py <src-dir>
"""
import re
import sys
import pathlib
from collections import Counter

struct_hdr = re.compile(r"^\s*pub struct (\w+)\s*\{")
field_decl = re.compile(r"^(\s*)pub (\w+)(:\s*.+,)\s*$")
ctor_hdr = re.compile(r"^\s*(\w+)\s*\{\s*$")
init_line = re.compile(r"^(\s*)(\w+)(:\s*.+,)\s*$")
init_short = re.compile(r"^(\s*)(\w+),\s*$")


def compute_renames(idents):
    """Return (dup_idents, {ident: [new_name_per_occurrence...]}) or None if no dups."""
    counts = Counter(idents)
    dups = {k for k, v in counts.items() if v > 1}
    if not dups:
        return None
    last = {n: j for j, n in enumerate(idents)}
    seqs, legacy = {}, {}
    for j, n in enumerate(idents):
        if n in dups and j != last[n]:
            legacy[n] = legacy.get(n, 0) + 1
            seqs.setdefault(n, []).append(f"{n}_legacy{legacy[n]}")
        else:
            seqs.setdefault(n, []).append(n)
    return dups, seqs


def process(text):
    lines = text.split("\n")
    n = len(lines)
    changed = False
    all_renames = {}  # struct name -> (dups, seqs)

    # Pass 1: rename duplicate fields in each struct body.
    i = 0
    while i < n:
        m = struct_hdr.match(lines[i])
        if not m:
            i += 1
            continue
        sname = m.group(1)
        depth = lines[i].count("{") - lines[i].count("}")
        j, field_idx = i + 1, []
        while j < n and depth > 0:
            depth += lines[j].count("{") - lines[j].count("}")
            if depth > 0 and field_decl.match(lines[j]):
                field_idx.append(j)
            j += 1
        idents = [field_decl.match(lines[k]).group(2) for k in field_idx]
        res = compute_renames(idents)
        if res:
            dups, seqs = res
            all_renames[sname] = (dups, seqs)
            ptr = {}
            for k in field_idx:
                fm = field_decl.match(lines[k])
                nm = fm.group(2)
                if nm in dups:
                    p = ptr.get(nm, 0)
                    ptr[nm] = p + 1
                    new = seqs[nm][p]
                    if new != nm:
                        lines[k] = f"{fm.group(1)}pub {new}{fm.group(3)}"
                        changed = True
        i = j

    if not all_renames:
        return text, False

    # Pass 2: apply the same renames to each struct's new() constructor body.
    i = 0
    while i < n:
        cm = ctor_hdr.match(lines[i])
        if not cm or cm.group(1) not in all_renames:
            i += 1
            continue
        dups, seqs = all_renames[cm.group(1)]
        depth = lines[i].count("{") - lines[i].count("}")
        j, ptr = i + 1, {}
        while j < n and depth > 0:
            depth += lines[j].count("{") - lines[j].count("}")
            if depth > 0:
                im = init_line.match(lines[j]) or init_short.match(lines[j])
                if im and im.group(2) in dups:
                    nm = im.group(2)
                    p = ptr.get(nm, 0)
                    if p < len(seqs[nm]):
                        ptr[nm] = p + 1
                        new = seqs[nm][p]
                        if new != nm:
                            lines[j] = f"{im.group(1)}{new}{lines[j][im.end(2):]}"
                            changed = True
            j += 1
        i = j

    return "\n".join(lines), changed


def main():
    root = pathlib.Path(sys.argv[1])
    fixed = 0
    for path in root.rglob("models/*.rs"):
        text = path.read_text()
        new_text, changed = process(text)
        if changed:
            path.write_text(new_text)
            fixed += 1
            print(f"    deduped fields in {path}")
    print(f"==> dedup: fixed {fixed} file(s)")


if __name__ == "__main__":
    main()
