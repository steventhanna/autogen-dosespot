//! Hand-written strict ID types shared by every plan module.
//!
//! The DoseSpot specs type every entity ID as a bare `integer`, so the generated functions
//! would take runs of adjacent `i32` parameters (patient, then prescription, …) where a
//! transposed argument compiles fine. `scripts/fix-id-types.py` rewrites a curated allowlist of
//! ID parameters and model fields to these newtypes after generation, so mixing up a
//! [`PatientId`] and a [`ClinicianId`] is a compile error. Spec names that are aliases of the
//! same entity (`fromPatientId`, `toPatientId`, `supervisorId`, …) map to the same newtype.
//!
//! IDs not on the allowlist remain `i32`; extend the table in `scripts/fix-id-types.py` to
//! promote more.
//!
//! Each type is a transparent wrapper: `PatientId(42)` to construct, `.0` (or `i32::from`) to
//! unwrap, serialized as a plain JSON number.

macro_rules! id_type {
    ($(#[doc = $doc:expr])* $name:ident) => {
        $(#[doc = $doc])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Default,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub i32);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<i32> for $name {
            fn from(value: i32) -> Self {
                Self(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> i32 {
                value.0
            }
        }
    };
}

id_type! {
    /// A DoseSpot patient. Also used for `fromPatientId`, `toPatientId`, and
    /// `newPrescriptionPatientId` spec parameters.
    PatientId
}
id_type! {
    /// A DoseSpot clinic. Also used for `sourceClinicId`.
    ClinicId
}
id_type! {
    /// A DoseSpot clinician. Also used for `supervisorId` (a supervisor is a clinician).
    ClinicianId
}
id_type! {
    /// A prescription. Also used for `originalPrescriptionId` and `referencedPrescriptionId`.
    PrescriptionId
}
id_type! {
    /// A pharmacy.
    PharmacyId
}
id_type! {
    /// An allergen (allergy class or screenable ingredient).
    AllergenId
}
id_type! {
    /// A patient's recorded allergy (distinct from the [`AllergenId`] it refers to).
    PatientAllergyId
}
id_type! {
    /// A dispensable drug.
    DispensableDrugId
}
id_type! {
    /// A supply item.
    SupplyId
}
id_type! {
    /// A refill request.
    RefillId
}
id_type! {
    /// An RX change request.
    RxChangeId
}
id_type! {
    /// A prior authorization.
    PriorAuthId
}
id_type! {
    /// A diagnosis. Also used for `primaryDiagnosisId` and `secondaryDiagnosisId`.
    DiagnosisId
}
id_type! {
    /// An eligibility check.
    EligibilityId
}
id_type! {
    /// An order set.
    OrderSetId
}
id_type! {
    /// A patient's self-reported medication.
    SelfReportedMedicationId
}
