//! # autogen-dosespot
//!
//! Auto-generated, strongly-typed, async Rust client for the
//! [DoseSpot](https://dosespot.com) v2 REST API.
//!
//! Every request/response type and API method is generated directly from DoseSpot's published
//! swagger specs (`https://my.dosespot.com/webapi/v2/swagger/docs/<Plan>`) with
//! [openapi-generator](https://openapi-generator.tech/), so the surface stays faithful to the
//! API and updates automatically when a spec changes. A thin hand-written [`DoseSpotClient`]
//! adds authentication and hands you a per-plan
//! [`Configuration`](crate::client::DoseSpotClient) with the correct base URL already set.
//!
//! ## Why one module per plan?
//!
//! DoseSpot publishes **one spec per subscription plan** (Full, Full + EPCS, Hybrid,
//! Hybrid + EPCS, Jumpstart, Jumpstart + EPCS, ReadOnly). The plans overlap heavily but each is
//! its own document, so each is vendored into its own top-level module — [`full`], [`full_epcs`],
//! [`hybrid`], [`hybrid_epcs`], [`jumpstart`], [`jumpstart_epcs`], [`readonly`] — keeping their
//! (otherwise colliding) model names isolated. Every plan is a Cargo feature: since a DoseSpot
//! account has exactly one plan, enable only yours and skip compiling the other six entirely.
//!
//! ## Quick start
//!
//! ```no_run
//! use autogen_dosespot::DoseSpotClient;
//!
//! # #[cfg(feature = "full")]
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = DoseSpotClient::new("your-subscription-key", "your-access-token")?;
//!
//!     // `client.full()` returns a `Configuration` pointed at the Full-plan API,
//!     // ready to pass to any function in `autogen_dosespot::full::apis`.
//!     let _config = client.full();
//!     Ok(())
//! }
//! # #[cfg(not(feature = "full"))]
//! # fn main() {}
//! ```
//!
//! ## Authentication
//!
//! The DoseSpot v2 API authenticates every request with two headers, neither of which is
//! declared in the swagger specs:
//!
//! - `Subscription-Key: <key>` — your API subscription key
//! - `Authorization: Bearer <token>` — an OAuth2 access token from
//!   `POST https://my.dosespot.com/webapi/v2/connect/token`; the [`token`] module provides a
//!   stateless [`token::request_token`] helper for this endpoint (it is absent from the specs
//!   and its password grant has non-obvious field values)
//!
//! [`DoseSpotClient`] wires both into the underlying HTTP client as default headers, so every
//! generated API function sends them automatically. Tokens expire, so construct a new client
//! (or new configurations) when you refresh the token.
//!
//! Each `client.<plan>()` accessor returns the generated `Configuration` for that plan. The base
//! URL (`https://my.dosespot.com/webapi/v2`) is baked in from the spec; you can still override
//! `base_path` on the returned value to point at DoseSpot staging, a proxy, or a mock server.
//!
//! ## Middleware
//!
//! Every generated `Configuration.client` is a `reqwest_middleware::ClientWithMiddleware`. Attach
//! middleware with [`DoseSpotClient::builder`] — for example a `reqwest_tracing::TracingMiddleware`
//! installed by the application:
//!
//! ```no_run
//! # #[cfg(feature = "full")]
//! # fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use autogen_dosespot::DoseSpotClient;
//!
//! let client = DoseSpotClient::builder("your-subscription-key", "your-access-token")
//!     // .with(reqwest_tracing::TracingMiddleware::default())
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! This crate itself emits no spans, logs no URLs, and has no opentelemetry dependency — it only
//! accepts and routes requests through whatever middleware you attach.
//!
//! ## Error handling
//!
//! Calls return `Result<T, apis::Error<E>>`, where `E` is the endpoint-specific error enum.
//! Each plan module exposes its own `apis::Error`, which separates transport errors,
//! (de)serialization errors, and structured API error responses (carrying the HTTP status and body).
//!
//! ## Feature flags
//!
//! By default all plans are enabled (so docs.rs shows everything). Your account has exactly one
//! plan — select it (and a TLS backend — `native-tls` or `rustls`) to skip compiling the rest:
//!
//! ```toml
//! [dependencies]
//! autogen-dosespot = { version = "0.2", default-features = false, features = ["full-epcs", "native-tls"] }
//! ```

#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

#[cfg(feature = "full")]
pub mod full;
#[cfg(feature = "full-epcs")]
pub mod full_epcs;
#[cfg(feature = "hybrid")]
pub mod hybrid;
#[cfg(feature = "hybrid-epcs")]
pub mod hybrid_epcs;
#[cfg(feature = "jumpstart")]
pub mod jumpstart;
#[cfg(feature = "jumpstart-epcs")]
pub mod jumpstart_epcs;
#[cfg(feature = "readonly")]
pub mod readonly;

pub mod client;
mod datetime;
pub mod ids;
pub mod token;

pub use client::{ClientBuildError, DoseSpotClient, DoseSpotClientBuilder};

/// Re-exported so callers build middleware against the same `reqwest_middleware` version this
/// crate links.
pub use reqwest_middleware;
/// Re-exported for the same reason: the `Middleware` trait is written in terms of `reqwest` types.
pub use reqwest;
