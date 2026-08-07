#!/usr/bin/env python3
"""Strip brackets from DoseSpot generic model names before generation.

DoseSpot's swagger specs name their generic wrappers with brackets — e.g.
`ItemResponse[PatientAllergy]`, `PagedListResponse[OrderSetResponse]`. openapi-generator
sanitizes those into unusable Rust type names like
`ItemResponseLeftSquareBracketPatientAllergyRightSquareBracket`. This pass rewrites the spec
in place, renaming every definition (and every `$ref` to one) with the brackets removed, so
the generator emits `ItemResponsePatientAllergy` instead. Verified collision-free: no spec
contains both `Foo[Bar]` and `FooBar`.

Runs on the downloaded spec files in the temp workdir, after SPEC_HASH is recorded (the hash
always reflects the raw upstream documents).

Usage: fix-model-names.py <spec.json> [<spec.json> ...]
"""
import json
import sys

REF_PREFIX = "#/definitions/"


def clean(name: str) -> str:
    return name.replace("[", "").replace("]", "")


def walk(node) -> None:
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "$ref" and isinstance(value, str) and value.startswith(REF_PREFIX):
                node[key] = REF_PREFIX + clean(value[len(REF_PREFIX):])
            else:
                walk(value)
    elif isinstance(node, list):
        for value in node:
            walk(value)


def main() -> None:
    for path in sys.argv[1:]:
        with open(path) as f:
            spec = json.load(f)
        definitions = spec.get("definitions", {})
        renamed = sum(1 for name in definitions if "[" in name)
        cleaned = {clean(name): schema for name, schema in definitions.items()}
        if len(cleaned) != len(definitions):
            raise SystemExit(f"{path}: bracket-stripping would collide model names")
        spec["definitions"] = cleaned
        walk(spec)
        with open(path, "w") as f:
            json.dump(spec, f, indent=1)
        print(f"    {path}: renamed {renamed} bracketed model(s)")


if __name__ == "__main__":
    main()
