# Observability — Design & Implementation Plan

This document describes how to add observability to SaDi: tracing (spans/events), OpenTelemetry export, and metrics (Prometheus). The goal is to provide actionable instrumentation with minimal disruption and optional feature flags so users only pay for what they enable.

Table of contents
- Goals
- Design principles
- High-level approach
- API surface & feature flags
- Implementation steps (priority order)
- Metrics & traces to capture
- Examples
- Tests and acceptance criteria
- Future work

---

## Goals

- Provide low-overhead instrumentation for container activity: registration (`bind`), resolution (`resolve`), provider execution, singleton creation, and circular-dependency detection.
- Allow users to export traces via OpenTelemetry (OTLP) and collect metrics via Prometheus.
- Reuse the existing `tracing` integration where possible and offer a minimal, optional "opentelemetry" feature that bridges tracing to OpenTelemetry.
- Keep the runtime behavior unchanged when observability features are disabled.
- Offer a clear developer-facing API for enabling observability and customizing behavior.

## Design principles

- Instrument at the logical operation level (not every tiny function): `bind`, `resolve(type)`, `factory.invoke`, `singleton.create`.
- Prefer emitting `tracing` spans/events and then provide adapters to export them to OTEL or other backends. This avoids coupling to specific backends and leverages Rust's tracing ecosystem.
- Use feature flags so additional dependencies (`opentelemetry`, `prometheus`) are optional.
- Provide lightweight metrics and be conservative about cardinality (e.g., tag by type name only when safe).

## High-level approach

1. Core instrumentation via `tracing` (existing `tracing` feature):
   - Add spans and events in `Container::bind_*`, `Container::resolve`, and `Factory::provide`.
   - Spans contain attributes: `type` (type_name), `singleton` (bool), `success`/`error`, `duration` (via span timing).
   - Use `tracing::instrument` or manual `tracing::span!`/`enter` for custom fields.

2. OpenTelemetry bridge (optional feature `opentelemetry`):
   - Add a crate feature `opentelemetry` which pulls in `tracing-opentelemetry` and `opentelemetry-otlp` (or user-selectable exporter).
   - Provide a helper API `sadi::observability::init_opentelemetry_collector(...)` or an example that shows wiring `tracing_subscriber` with `tracing_opentelemetry::layer()`.
   - Recommend the user initialize the OTLP exporter in their application (README examples), not implicitly within the library.

3. Metrics (optional feature `metrics` / `prometheus`):
   - Provide a small internal metrics recorder that emits counters/histograms for:
     - `sadi_resolve_attempts_total{type=...}`
     - `sadi_resolve_success_total{type=...}`
     - `sadi_resolve_failure_total{type=..., reason=...}`
     - `sadi_resolve_duration_seconds{type=...}` (histogram)
     - `sadi_singleton_creations_total{type=...}`
   - Use `opentelemetry-metrics` or `prometheus` crate when `prometheus` feature enabled. Provide an example HTTP handler exposing `/metrics` for Prometheus.
   - Keep default labels small (type name), avoid high-cardinality labels (do not add caller ids, etc.).

## API surface & Feature flags

Suggested feature flags in `sadi/Cargo.toml`:
- `tracing` (already exists) — enables logging/tracing hooks.
- `opentelemetry` (optional) — adds `tracing-opentelemetry` and `opentelemetry-otlp`/`opentelemetry` crates and example helpers.
- `metrics` (optional) — enables Prometheus or OpenTelemetry metric exporter.

Public helpers to provide (examples):
- `sadi::observability::init_tracing_subscriber_with_otlp(endpoint: &str)` — example helper that configures `tracing_subscriber` + `tracing-opentelemetry` to send to OTLP.
- `sadi::observability::install_prometheus_metrics_registry()` — helper example showing how to register collectors and expose `/metrics`.
- `sadi::observability::MetricsRecorder` trait (optional) — allows plugging custom metric backends.

Important note: the library should not auto-initialize global exporters (like OTLP) by default — instead provide examples and helpers so application authors choose how/where to export telemetry.

## Implementation steps (priority order)

1. Instrumentation via `tracing` (low-risk, high-value)
   - Add spans/events in these places:
     - `Container::bind_*` — event: `sadi.bind` with `type` and `singleton` fields.
     - `Container::resolve::<T>()` — span: `sadi.resolve` with `type` field; inside, record success/error events.
     - `Factory::provide` — span: `sadi.factory.provide` with `singleton` and provider result.
     - `ResolveGuard::push` error path — event: `sadi.circular_dependency` with chain info.
   - Ensure fields are stable strings or low-cardinality.

2. Add a tiny `observability` module in `sadi/src/observability.rs` that exposes helpers and docs. Minimum: re-exporting examples and optional functions that only compile with `opentelemetry` feature.

3. OpenTelemetry integration (feature `opentelemetry`) — opt-in
   - Add dependencies under the `opentelemetry` feature: `tracing-opentelemetry`, `opentelemetry`, `opentelemetry-otlp` (optional), `tracing-subscriber` (as example).
   - Provide `init_opentelemetry` helper that returns the `_uninstall` guard if needed, and example code in README showing how to wire it in app startup.

4. Metrics (feature `metrics` or `prometheus`) — opt-in
   - Add optional `prometheus` or `opentelemetry-metrics` dependency.
   - Add a `metrics` module that uses lazy_static or once_cell to register metrics, provide function `record_resolve_attempt(type_name)` etc.
   - Add small helper example to run a Prometheus HTTP endpoint using `hyper` or `tiny-http` in examples (documented only; not a hard dependency).

5. Tests & docs
   - Unit test that checks spans/events are emitted (use `tracing` test subscriber capturing spans) — ensure instrumentation code runs and fields are set.
   - Example in `examples/observability/` showing wiring OTLP and a small app using SaDi with tracing.

## Metrics & traces to capture (concrete)

Traces / Spans
- `sadi.resolve` (span)
  - Attributes: `type_name`, `singleton: bool`
  - Events: `started`, `success`, `error` (with `error.kind`)
  - Duration: overall resolve time

- `sadi.factory.provide` (span)
  - Attributes: `singleton`, `provider_id` (optional, avoid high-cardinality)
  - Events: `created_instance` (for singleton first-time)

- `sadi.bind` (event)
  - Attributes: `type_name`, `factory_kind` (concrete/abstract/singleton/instance)

Metrics
- Counter: `sadi_resolve_attempts_total{type}`
- Counter: `sadi_resolve_success_total{type}`
- Counter: `sadi_resolve_failure_total{type, reason}`
- Histogram: `sadi_resolve_duration_seconds{type}`
- Counter: `sadi_singleton_creations_total{type}`

Design note: histograms and counters can be recorded via OpenTelemetry metrics or Prometheus depending on enabled feature.

## Examples

1) Simple tracing-only setup (recommended minimal approach):

```rust
use tracing_subscriber;
use tracing_subscriber::prelude::*;

fn main() {
    tracing_subscriber::fmt::init(); // print to stdout

    // your saDi setup and usage
}
```

2) Bridge tracing to OpenTelemetry (application code, `opentelemetry` feature enabled):

```rust
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry_otlp::new_pipeline;

fn init_otlp(endpoint: &str) {
    let exporter = opentelemetry_otlp::new_pipeline().with_endpoint(endpoint).install_simple().unwrap();
    let otel_layer = OpenTelemetryLayer::new(exporter);
    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer()).with(otel_layer).init();
}

// call init_otlp("http://localhost:4317") at app startup
```

3) Prometheus metrics example (application code, `metrics`/`prometheus` feature enabled):

```rust
use prometheus::{TextEncoder, Encoder, Registry};
use hyper::Server;
// register metrics from sadi::observability::init_prometheus_registry()

// Expose /metrics handler returning encoder.encode(&registry.gather())
```

## Tests and acceptance criteria

- Instrumentation should be no-op without `tracing` feature or when no subscriber is installed.
- With `tracing` enabled, spans should be emitted with correct fields (unit tests using `tracing` test subscriber).
- With `opentelemetry` feature and example helper used, traces should be sent to configured collector (manually validated in example).
- Metrics should be recordable and the Prometheus example must expose scrape-able metrics.

## Future work

- More advanced metrics (per-container labels, instance counts).
- Async-friendly instrumentation (support async factories and non-blocking waits for resource limits).
- Expose a `ObservabilityLayer` trait for pluggable backends.
- Add higher-level dashboards and example Grafana dashboards for recommended metrics.

---

If you want, I can now:
- Implement step 1 (add `tracing` spans/events across the crate), or
- Add the `observability` module and example helpers for `opentelemetry` (feature-gated), or
- Create a small example under `examples/observability/` that wires tracing -> otel and metrics -> prometheus.

Which should I implement first?