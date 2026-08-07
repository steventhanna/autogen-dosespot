#!/usr/bin/env python3
"""Rewrite chrono DateTime query-param serialization to RFC 3339.

The upstream rust/reqwest generator template serializes every scalar query
parameter with `param_value.to_string()`. For `chrono::DateTime<FixedOffset>`
that is chrono's `Display` ("2026-07-17 00:00:00 +00:00"), which is not
RFC 3339 and is rejected by the API (tracked by
tests/datetime_query_params.rs). This pass rewrites those call sites to
`to_rfc3339_opts(SecondsFormat::Secs, true)` ("2026-07-17T00:00:00Z").

Idempotent: a rewritten site no longer contains `to_string()` and is skipped.

Usage: fix-datetime-query-params.py <src-dir>
"""
import re
import sys
from pathlib import Path

SIGNATURE = re.compile(r"^pub async fn \w+\((.*)\) -> ")
DATETIME_PARAM = re.compile(r"(\w+): Option<chrono::DateTime<chrono::FixedOffset>>")
QUERY_GUARD = re.compile(r"^\s*if let Some\(ref param_value\) = p_query_(\w+) \{")

REPLACEMENT = (
    "param_value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)"
)


def process(path: Path) -> int:
    lines = path.read_text().splitlines(keepends=True)
    datetime_params: set[str] = set()
    in_datetime_guard = False
    changed = 0
    for i, line in enumerate(lines):
        sig = SIGNATURE.match(line)
        if sig:
            datetime_params = set(DATETIME_PARAM.findall(sig.group(1)))
            in_datetime_guard = False
            continue
        guard = QUERY_GUARD.match(line)
        if guard:
            in_datetime_guard = guard.group(1) in datetime_params
            continue
        if in_datetime_guard:
            in_datetime_guard = False
            new = line.replace("param_value.to_string()", REPLACEMENT)
            if new != line:
                lines[i] = new
                changed += 1
    if changed:
        path.write_text("".join(lines))
    return changed


def main() -> None:
    root = Path(sys.argv[1])
    total = 0
    for path in sorted(root.glob("*/apis/*.rs")):
        n = process(path)
        if n:
            print(f"    {path}: rewrote {n} datetime query param(s) to RFC 3339")
            total += n
    print(f"    {total} datetime query param site(s) fixed")


if __name__ == "__main__":
    main()
