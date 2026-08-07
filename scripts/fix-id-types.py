#!/usr/bin/env python3
"""Promote allowlisted integer ID parameters and fields to strict newtypes.

The DoseSpot specs type every entity ID as a bare `integer`, so generated functions take runs
of adjacent `i32` parameters where transposed arguments compile fine. This pass rewrites a
curated allowlist of ID names — in generated function signatures and model struct fields — from
`i32` / `Option<i32>` to the shared hand-written newtypes in `src/ids.rs` (crate::ids). Spec
names that alias the same entity (fromPatientId, supervisorId, …) map to the same newtype.

Deliberately allowlist-based: an ID name not in the table stays `i32`, so new spec fields can
never break generation — they just arrive untyped until added here (keep src/ids.rs in sync).

Idempotent: a rewritten site no longer reads `: i32` and is skipped.

Usage: fix-id-types.py <src-dir>
"""
import re
import sys
from pathlib import Path

# Rust newtype (in crate::ids) -> generated snake_case names it covers.
ID_TYPES = {
    "PatientId": ["patient_id", "from_patient_id", "to_patient_id", "new_prescription_patient_id"],
    "ClinicId": ["clinic_id", "source_clinic_id"],
    "ClinicianId": ["clinician_id", "supervisor_id"],
    "PrescriptionId": ["prescription_id", "original_prescription_id", "referenced_prescription_id"],
    "PharmacyId": ["pharmacy_id"],
    "AllergenId": ["allergen_id"],
    "PatientAllergyId": ["patient_allergy_id"],
    "DispensableDrugId": ["dispensable_drug_id"],
    "SupplyId": ["supply_id"],
    "RefillId": ["refill_id"],
    "RxChangeId": ["rx_change_id"],
    "PriorAuthId": ["prior_auth_id"],
    "DiagnosisId": ["diagnosis_id", "primary_diagnosis_id", "secondary_diagnosis_id"],
    "EligibilityId": ["eligibility_id"],
    "OrderSetId": ["order_set_id"],
    "SelfReportedMedicationId": ["self_reported_medication_id"],
}

NAME_TO_TYPE = {
    name: rust_type for rust_type, names in ID_TYPES.items() for name in names
}

# Matches `<name>: i32` or `<name>: Option<i32>` in function signatures (plain and Option
# query params), struct fields (`pub <name>: Option<i32>,`), and `new()` constructor params.
SITE = re.compile(
    r"\b(" + "|".join(sorted(NAME_TO_TYPE, key=len, reverse=True)) + r"): (Option<)?i32\b"
)


def replacement(m: re.Match) -> str:
    rust_type = f"crate::ids::{NAME_TO_TYPE[m.group(1)]}"
    if m.group(2):
        return f"{m.group(1)}: Option<{rust_type}"
    return f"{m.group(1)}: {rust_type}"


def main() -> None:
    root = Path(sys.argv[1])
    files = 0
    sites = 0
    # Generated code only — never the hand-written files at the src root.
    paths = sorted(list(root.glob("*/apis/*.rs")) + list(root.glob("*/models/*.rs")))
    for path in paths:
        text = path.read_text()
        new_text, n = SITE.subn(replacement, text)
        if n:
            path.write_text(new_text)
            files += 1
            sites += n
    print(f"    promoted {sites} ID site(s) to strict newtypes across {files} file(s)")


if __name__ == "__main__":
    main()
