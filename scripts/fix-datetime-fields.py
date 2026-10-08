#!/usr/bin/env python3
"""Point every generated chrono DateTime model field at the lenient deserializer.

The specs declare dates as `format: date-time`, so the rust generator types them as
`chrono::DateTime<chrono::FixedOffset>`, whose default deserializer requires an offset. DoseSpot
returns many dates without one ("2026-03-12T23:38:38.207"), failing the whole response. This
pass adds `deserialize_with = "crate::datetime::…"` (and `default` for Option fields, so a missing
field stays None) to the `#[serde(...)]` attribute directly above each such field.

Fails loudly on a DateTime field it cannot rewrite (no serde attribute directly above, or a type
shape it does not know), so a generator change cannot silently drop the fix.
Guarded by tests/datetime_fields.rs.

Idempotent: an attribute that already contains `deserialize_with` is skipped.

Usage: fix-datetime-fields.py <src-dir>
"""
import re
import sys
from pathlib import Path

DT = "chrono::DateTime<chrono::FixedOffset>"
FIELD = re.compile(r"^\s*pub \w+: (.*" + re.escape(DT) + r".*),$")
SERDE_ATTR = re.compile(r"^(\s*#\[serde\()(.*)(\)\]\s*)$")

INSERTS = {
    DT: 'deserialize_with = "crate::datetime::deserialize"',
    f"Option<{DT}>": 'default, deserialize_with = "crate::datetime::deserialize_option"',
}


def process(path: Path) -> int:
    lines = path.read_text().splitlines(keepends=True)
    changed = 0
    for i, line in enumerate(lines):
        field = FIELD.match(line)
        if not field:
            continue
        insert = INSERTS.get(field.group(1))
        attr = SERDE_ATTR.match(lines[i - 1]) if i else None
        if insert is None or attr is None:
            sys.exit(f"error: cannot apply lenient datetime deserializer at {path}:{i + 1}: {line.strip()}")
        if "deserialize_with" in attr.group(2):
            continue
        lines[i - 1] = f"{attr.group(1)}{attr.group(2)}, {insert}{attr.group(3)}"
        changed += 1
    if changed:
        path.write_text("".join(lines))
    return changed


def main() -> None:
    root = Path(sys.argv[1])
    files = 0
    sites = 0
    for path in sorted(root.glob("*/models/*.rs")):
        n = process(path)
        if n:
            files += 1
            sites += n
    print(f"    applied lenient datetime deserializer to {sites} field(s) across {files} file(s)")


if __name__ == "__main__":
    main()
