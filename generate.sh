#!/bin/bash
set -euo pipefail

# Regenerate every DoseSpot plan module from the upstream swagger specs.
#
# DoseSpot ships one Swagger 2.0 spec per subscription plan (Full, Full+EPCS, Hybrid,
# Hybrid+EPCS, Jumpstart, Jumpstart+EPCS, ReadOnly), all served from the same host. We generate
# each into a temp dir, then vendor only its `src/apis` and `src/models` into `src/<plan>/`,
# rewriting the generator's absolute `crate::apis` / `crate::models` paths to `crate::<plan>::…`
# so the code compiles inside a submodule. The run is idempotent: an unchanged spec reproduces
# the committed tree.
#
# Requires only a JDK. The openapi-generator version is pinned and the JAR is fetched directly
# from Maven Central (cached under ~/.cache), so local and CI runs are byte-for-byte identical —
# no dependence on a brew/npm install whose default generator version drifts.

# Portable in-place sed (macOS uses -i '', Linux uses -i).
sedi() {
  if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "$@"
  else
    sed -i "$@"
  fi
}

SPEC_BASE="https://my.dosespot.com/webapi/v2/swagger/docs"
# "<upstream spec name>:<rust module name>" pairs. The spec name is the URL path segment
# (case-sensitive — note JumpStart_EPCSV2 vs JumpstartV2); the module name doubles as the
# Cargo feature name with '_' replaced by '-'.
PLANS="
FullV2:full
Full_EPCSV2:full_epcs
HybridV2:hybrid
Hybrid_EPCSV2:hybrid_epcs
JumpstartV2:jumpstart
JumpStart_EPCSV2:jumpstart_epcs
ReadOnlyV2:readonly
"

# Pinned generator version. Bumping this is a deliberate act (it can change generated output, e.g.
# the String -> chrono date-time switch in 7.15+); regenerate and review the diff when you change it.
GENERATOR_VERSION="${GENERATOR_VERSION:-7.23.0}"
JAR_CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/openapi-generator"
JAR="$JAR_CACHE/openapi-generator-cli-${GENERATOR_VERSION}.jar"
if [ ! -f "$JAR" ]; then
  echo "==> Fetching openapi-generator ${GENERATOR_VERSION} JAR..."
  mkdir -p "$JAR_CACHE"
  curl -sS -L --fail -o "$JAR" \
    "https://repo1.maven.org/maven2/org/openapitools/openapi-generator-cli/${GENERATOR_VERSION}/openapi-generator-cli-${GENERATOR_VERSION}.jar"
fi
GEN=(java -jar "$JAR")

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "==> Fetching DoseSpot swagger specs from $SPEC_BASE ..."
for pair in $PLANS; do
  spec="${pair%%:*}"
  curl -sS -L --fail -o "$WORK/$spec.json" "$SPEC_BASE/$spec"
  echo "    $spec.json: $(wc -c < "$WORK/$spec.json" | tr -d ' ') bytes"
done

echo "==> Recording combined spec hash..."
( cd "$WORK" && shasum -a 256 $(for pair in $PLANS; do echo "${pair%%:*}.json"; done) \
  | sort | shasum -a 256 | awk '{print $1}' ) > SPEC_HASH
echo "    SPEC_HASH = $(cat SPEC_HASH)"

echo "==> Sanitizing generic model names (ItemResponse[X] -> ItemResponseX)..."
python3 scripts/fix-model-names.py $(for pair in $PLANS; do echo "$WORK/${pair%%:*}.json"; done)

for pair in $PLANS; do
  spec="${pair%%:*}"
  module="${pair##*:}"
  echo "==> Generating '$spec' -> src/$module ..."

  rm -rf "$WORK/gen-$spec"
  "${GEN[@]}" generate \
    -i "$WORK/$spec.json" \
    -g rust \
    --library reqwest \
    --skip-validate-spec \
    --additional-properties=packageName=autogen-dosespot-${module//_/-},supportAsync=true \
    -o "$WORK/gen-$spec" \
    2>&1 | tail -3

  rm -rf "src/$module"
  mkdir -p "src/$module"
  cp -R "$WORK/gen-$spec/src/apis" "src/$module/apis"
  cp -R "$WORK/gen-$spec/src/models" "src/$module/models"

  # Rewrite absolute crate paths so the vendored code resolves inside src/$module/.
  # Order matters: the grouped-import form `crate::{apis::…, models}` is rewritten first; the
  # standalone `crate::apis` / `crate::models` forms are then caught without double-rewriting.
  while IFS= read -r -d '' f; do
    sedi \
      -e "s/crate::{/crate::$module::{/g" \
      -e "s/crate::apis/crate::$module::apis/g" \
      -e "s/crate::models/crate::$module::models/g" \
      "$f"
  done < <(find "src/$module" -name '*.rs' -print0)

  printf 'pub mod apis;\npub mod models;\n' > "src/$module/mod.rs"
done

echo "==> Applying post-generation fixes..."
# Guard against specs declaring two properties that differ only in case; the rust generator
# snake_cases both to one field, producing an uncompilable duplicate. This idempotent pass
# renames the earlier collision to <name>_legacy<n>. (Currently a no-op for DoseSpot.)
python3 scripts/fix-duplicate-fields.py src

# Promote allowlisted integer entity IDs (patientId, prescriptionId, …) to the strict
# newtypes in src/ids.rs, so transposed ID arguments become compile errors. Allowlist-based:
# unknown IDs stay i32. Keep scripts/fix-id-types.py and src/ids.rs in sync.
python3 scripts/fix-id-types.py src

# The generator serializes chrono DateTime query params via Display ("2026-07-17 00:00:00 +00:00"),
# which is not RFC 3339 and is rejected by the API. Rewrite those sites to to_rfc3339_opts.
# Guarded by tests/datetime_query_params.rs.
python3 scripts/fix-datetime-query-params.py src

echo "==> Verifying compilation..."
cargo check --all-features
cargo check --no-default-features --features "readonly,native-tls"

echo "==> Done. Review changes with: git diff"
