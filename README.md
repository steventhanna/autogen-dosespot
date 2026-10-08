# autogen-dosespot

[![CI](https://github.com/steventhanna/autogen-dosespot/actions/workflows/ci.yml/badge.svg)](https://github.com/steventhanna/autogen-dosespot/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/autogen-dosespot.svg)](https://crates.io/crates/autogen-dosespot)
[![docs.rs](https://img.shields.io/docsrs/autogen-dosespot)](https://docs.rs/autogen-dosespot)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

Auto-generated, strongly-typed, async Rust client for the [DoseSpot](https://dosespot.com) v2 REST API.

Every request/response type and API method is generated directly from DoseSpot's published
swagger specs (`https://my.dosespot.com/webapi/v2/swagger/docs/<Plan>`) with
[openapi-generator](https://openapi-generator.tech/), so the surface stays faithful to the API and
updates automatically when a spec changes. A thin hand-written `DoseSpotClient` adds
authentication on top.

- **Complete** — all seven DoseSpot plan specs (Full, Full + EPCS, Hybrid, Hybrid + EPCS,
  Jumpstart, Jumpstart + EPCS, ReadOnly), every endpoint and model.
- **Async** — built on [`reqwest`](https://docs.rs/reqwest); works on any Tokio runtime.
- **Modular** — every plan is a Cargo feature; your account has one plan, so you compile only that.
- **Collision-free** — each plan lives in its own module (`full`, `jumpstart`, …), so identically
  named models across specs never clash.
- **Strict IDs** — entity IDs are shared newtypes (`ids::PatientId`, `ids::PrescriptionId`, …),
  not bare `i32`, so transposing two ID arguments is a compile error.
- **No magic** — generated code is committed; no build scripts, no proc-macros, no codegen at build time.

## Installation

Your DoseSpot account has exactly one plan — enable only that feature (plus a TLS backend):

```toml
[dependencies]
autogen-dosespot = { version = "0.2", default-features = false, features = ["full-epcs", "native-tls"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The default features enable **all seven** plans (so docs.rs documents everything), which is a
much slower build than you need.

> **Note:** with `default-features = false` you must enable a TLS backend — either `native-tls`
> or `rustls` — or HTTPS requests will fail at runtime.

## Quick Start

```rust,no_run
use autogen_dosespot::DoseSpotClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = DoseSpotClient::new("your-subscription-key", "your-access-token")?;

    // Each accessor returns a `Configuration` for that plan, with the base URL and auth set.
    #[cfg(feature = "full")]
    {
        let full = client.full();
        // Pass it to any function in the matching `apis` module:
        // autogen_dosespot::full::apis::patients_api::<operation>(&full, ...).await?;
        let _ = full;
    }
    Ok(())
}
```

## The seven plans

Each DoseSpot plan spec is a top-level module and a Cargo feature. The client accessor returns
that plan's generated `Configuration` (all plans share the base URL
`https://my.dosespot.com/webapi/v2`):

| Feature | Module | Accessor | Upstream spec |
|---------|--------|----------|---------------|
| `full` | `full` | `client.full()` | `FullV2` |
| `full-epcs` | `full_epcs` | `client.full_epcs()` | `Full_EPCSV2` |
| `hybrid` | `hybrid` | `client.hybrid()` | `HybridV2` |
| `hybrid-epcs` | `hybrid_epcs` | `client.hybrid_epcs()` | `Hybrid_EPCSV2` |
| `jumpstart` | `jumpstart` | `client.jumpstart()` | `JumpstartV2` |
| `jumpstart-epcs` | `jumpstart_epcs` | `client.jumpstart_epcs()` | `JumpStart_EPCSV2` |
| `readonly` | `readonly` | `client.readonly()` | `ReadOnlyV2` |

**TLS backends** (one required): `native-tls` (default) or `rustls`.

## Authentication

The DoseSpot v2 API authenticates every request with two headers, neither of which appears in
the swagger specs:

- `Subscription-Key: <key>` — your API subscription key
- `Authorization: Bearer <token>` — an OAuth2 access token from
  `POST https://my.dosespot.com/webapi/v2/connect/token`

The token endpoint is absent from the specs and its password grant has non-obvious field values
(notably `password` is the clinic key, not a user password), so the crate ships a stateless
`token::request_token` helper for it. `DoseSpotClient` then sets both headers as defaults on the
underlying HTTP client, so every generated API function sends them automatically:

```rust,no_run
use autogen_dosespot::{DoseSpotClient, token};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let http = autogen_dosespot::reqwest_middleware::ClientBuilder::new(autogen_dosespot::reqwest::Client::new()).build();
let token = token::request_token(
    &http,
    "https://my.dosespot.com",
    "your-subscription-key",
    "your-clinic-id",
    "your-clinic-key",
    "your-clinician-id",
)
.await?;
let client = DoseSpotClient::new("your-subscription-key", &token.access_token)?;
# Ok(())
# }
```

Pass the same `http` (with the same middleware chain) you give [`DoseSpotClient::builder`](#middleware)
so the token request is observed identically to every other call.

Access tokens expire (`token.expires_in`) — caching, refresh, and per-clinician token strategy
are up to you; mint a token and construct a new `DoseSpotClient` whenever your policy calls
for it. To point at
DoseSpot staging, a proxy, or a mock server, mutate `base_path` on the returned configuration:

```rust,no_run
# use autogen_dosespot::DoseSpotClient;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let client = DoseSpotClient::new("your-subscription-key", "your-access-token")?;
# #[cfg(feature = "full")] {
let mut full = client.full();
full.base_path = "https://my.staging.dosespot.com/webapi/v2".to_string();
# }
# Ok(())
# }
```

## Middleware

Every generated `Configuration.client` is a `reqwest_middleware::ClientWithMiddleware`. Attach
middleware — for example a `reqwest_tracing::TracingMiddleware` installed by the application —
with `DoseSpotClient::builder`:

```rust,no_run
# #[cfg(feature = "full")]
# fn run() -> Result<(), Box<dyn std::error::Error>> {
use autogen_dosespot::DoseSpotClient;

let client = DoseSpotClient::builder("your-subscription-key", "your-access-token")
    // .with(my_middleware)
    .build()?;
# Ok(())
# }
```

Middleware is applied in the order added, to every request the crate makes, including the token
request (pass the same chain to `token::request_token`; see [Authentication](#authentication)).
`autogen_dosespot::reqwest_middleware` is re-exported so you build middleware against the same
version this crate links. The crate itself creates no spans, logs no URLs, and has no
opentelemetry dependency — it only accepts and routes requests through what you attach.

> **Breaking in 0.2:** `Configuration.client` changed from `reqwest::Client` to
> `reqwest_middleware::ClientWithMiddleware` (and each plan's `apis::Error` gained a
> `ReqwestMiddleware` variant); `token::request_token` takes the HTTP client as its first
> argument; `token::TokenError::Reqwest(reqwest::Error)` became
> `TokenError::Http(reqwest_middleware::Error)`; and the crate moved to reqwest 0.13 /
> reqwest-middleware 0.5 (pair it with reqwest-tracing 0.7).

## Strict ID types

The specs type every entity ID as a bare `integer`, which would make generated signatures like
`(patient_id: i32, prescription_id: i32)` silently accept transposed arguments. Generation
promotes a curated allowlist of ID names to shared newtypes in the hand-written `ids` module —
`PatientId`, `ClinicId`, `ClinicianId`, `PrescriptionId`, `PharmacyId`, `AllergenId`, and more —
used consistently across function parameters and model fields in every plan module. Spec names
that alias one entity map to one type (`fromPatientId`/`toPatientId` → `PatientId`,
`supervisorId` → `ClinicianId`).

They are transparent wrappers: construct with `ids::PatientId(42)` (or `.into()`), unwrap with
`.0`, and they serialize as plain JSON numbers. IDs not on the allowlist stay `i32`; extend the
table in `scripts/fix-id-types.py` (and `src/ids.rs`) to promote more.

## Error Handling

Generated functions return `Result<T, apis::Error<E>>`, where `E` is the endpoint-specific error
enum. Each plan module has its own `apis::Error`, distinguishing transport errors,
(de)serialization errors, and structured API error responses:

```rust,ignore
use autogen_dosespot::full::apis::Error;

match some_call(&config).await {
    Ok(resp) => { /* ... */ }
    Err(Error::ResponseError(e)) => eprintln!("API returned HTTP {}: {}", e.status, e.content),
    Err(Error::Reqwest(e)) => eprintln!("transport error: {e}"),
    Err(e) => eprintln!("other error: {e}"),
}
```

## How It's Generated

```bash
# Requires only a JDK (the generator JAR is fetched automatically).
./generate.sh
```

`generate.sh` fetches all seven plan specs from
`https://my.dosespot.com/webapi/v2/swagger/docs/<Plan>`, records their combined SHA-256 in
`SPEC_HASH`, sanitizes the generic model names (`ItemResponse[X]` → `ItemResponseX` — the raw
names would otherwise become unusable Rust identifiers), runs openapi-generator (rust + reqwest
template) on each, and vendors the generated `apis/` and `models/` into a per-plan module under
`src/<plan>/`. Because the rust generator emits absolute `crate::apis` / `crate::models` paths,
the script rewrites them to `crate::<plan>::…` so the code compiles inside a submodule. Finally
it rewrites chrono `DateTime` query-param serialization to RFC 3339 (guarded by
`tests/datetime_query_params.rs`), points every date-time model field at a lenient deserializer
(guarded by `tests/datetime_fields.rs`), and runs `cargo check`.

The generator version is **pinned** (`GENERATOR_VERSION` in `generate.sh`) and the JAR is downloaded
directly from Maven Central, so local and CI runs are byte-for-byte identical — there's no dependence
on a brew/npm install whose default generator version drifts. The run is idempotent: a fresh
generation reproduces the committed tree exactly. Date-time fields are typed as
`chrono::DateTime<chrono::FixedOffset>` (generator 7.15+). DoseSpot returns many of them without an
offset (`"2026-03-12T23:38:38.207"`); those deserialize as UTC.

Only the entry points are hand-written and protected from regeneration: `Cargo.toml`, `src/lib.rs`,
`src/client.rs`, plus `README.md` and `CLAUDE.md`. Everything under `src/<plan>/` is generated —
**do not edit it by hand**; fix the spec upstream or adjust `generate.sh` instead.

### Staying in sync with the specs

The crate version is content-hash based, starting at `0.1.0`. A scheduled GitHub Action
(`update-spec.yml`) regenerates from the live specs **daily**; when the combined hash changes it
bumps the patch version and opens a PR. Merging that PR tags the release and publishes the new
version to crates.io (`tag-release.yml`).

Publishing requires a `CARGO_REGISTRY_TOKEN` repository secret (a crates.io API token). The first
`0.1.0` release should be published manually (`cargo publish`) to establish crate ownership;
automated patch releases flow through CI from then on.

## Why auto-generated?

Hand-written SDKs drift from the API and accumulate subtle type mismatches. Generating from the
specs keeps the client honest:

- **Always current** — a spec change becomes a PR, not a manually-tracked changelog entry.
- **Exhaustive** — every endpoint and model is present, not just the popular ones.
- **Auditable** — generated code is committed and reviewable; there's no build-time codegen to trust.

## License

MIT
