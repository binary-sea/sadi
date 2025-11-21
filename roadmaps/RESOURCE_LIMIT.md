# Resource Limits — Design Document

This document describes a practical proposal to implement Resource Limits in SaDi. The goal is to provide mechanisms to control the creation and usage of resources per binding (service) in the container, preventing bursts of creations, protecting system resources, and offering configurable policies.

## Goals

- Allow limiting the concurrency of instance creation per binding (max concurrent creations).
- Provide simple policies when limits are hit: `Deny` (fail fast) and `Block` (synchronously wait until a slot is freed).
- Support behavior applicable to both transient bindings and singletons (concurrency limits are useful for both; accurate instance-count tracking is optional for a later iteration).
- Offer an ergonomic API through helpers and macros (e.g. `bind(limit = 5, ...)`).
- Implement a synchronous, thread-safe first iteration; provide a simplified variant for non-`thread-safe` builds.

## Constraints and Scope

- We will not attempt to measure actual process memory — that requires allocator/OS integration and is out of scope.
- We will not change the public return type (`Shared<T>`) in the initial iteration. Therefore we will not implement accurate live-instance counting that depends on wrapping returned values. Accurate instance counting can be added later via weak-tracking or optional wrappers.
- The initial implementation focuses on concurrency limits (semaphore-style) with `Deny` and `Block` policies.
- Async/pluggable support (e.g., using `tokio::Semaphore`) is a planned enhancement but not required for the first iteration.

## Terminology

- Limits: a per-binding configuration with fields such as `max_concurrent_creations: Option<usize>` and `policy: Policy`.
- Policy: `{ Deny, Block }`.
- ResourceLimiter: the structure that enforces rules; in thread-safe mode it uses atomics + Mutex/Condvar, in non-thread-safe mode it uses simple counters.

## Proposed API (high-level)

- New public types:
  - `pub enum Policy { Deny, Block }`
  - `pub struct Limits { pub max_concurrent_creations: Option<usize>, pub policy: Policy }

- Bind helper variants:
  - `Container::bind_concrete_with_limits::<T,U,F>(&self, provider: F, limits: Limits) -> Result<(), Error>`
  - Macro sugar: `bind(limit = 5, Type => |c| ...)` (via an extension in `macros.rs`)

- Behavior:
  - When `Factory::provide` is invoked and the `Factory` has an associated `ResourceLimiter`, execution should first try to `acquire()` a slot:
    - If `acquire()` returns `Ok(Guard)`, continue creation; the `Guard` automatically releases the slot when dropped.
    - If `acquire()` fails and the policy is `Policy::Deny`, return `Error::resource_limit_exceeded(...)`.
    - If `acquire()` fails and the policy is `Policy::Block`, block until a slot is available (with possible timeout support in the future).

## Technical implementation (step-by-step)

1. Create the file `sadi/src/resource_limits.rs` with the types:
   - `Policy`, `Limits` (derive Debug/Clone), `ResourceLimiter`, and `AcquireGuard`.
   - Provide `ResourceLimiter::try_acquire(&self) -> Option<AcquireGuard>` (non-blocking) and `acquire_blocking(&self) -> AcquireGuard` (blocking).

2. Internal structure (thread-safe):
   - `struct ResourceLimiter { max: Option<usize>, counter: std::sync::atomic::AtomicUsize, mutex: std::sync::Mutex<()>, cvar: std::sync::Condvar }`.
   - `try_acquire` performs an atomic loop: read the counter; if `max.is_some()` and `counter >= max` return `None`; otherwise increment via compare-and-swap and return an `AcquireGuard`.
   - `acquire_blocking` uses a `Mutex` + `Condvar` to wait while `counter >= max`; when a slot is available it increments and returns an `AcquireGuard`.
   - `AcquireGuard` decrements the `counter` and notifies the `Condvar` on `Drop`.

3. Non-thread-safe variant (`cfg(not(feature = "thread-safe"))`):
   - Implement `ResourceLimiter` using `std::cell::Cell<usize>`; the non-thread-safe variant can adopt a simpler policy (e.g., `Deny` only) since blocking in single-threaded mode is uncommon.

4. Integration with `Factory`/`Container`:
   - Extend `Factory<T>` to optionally hold `limits: Option<Limits>` (or `limiter: Option<Arc<ResourceLimiter>>`).
   - Provide a constructor like `Factory::new_with_limits(provider, singleton, Option<Limits>)` that creates a `ResourceLimiter` and stores it in the `Factory`.
   - Update `Factory::provide(&self, container: &Container)` to check `self.limiter` and invoke `try_acquire()` or `acquire_blocking()` according to the policy.

   - Practical note: `Factory::provide` currently returns `Shared<T>` directly. To propagate errors cleanly we should change its return type to `Result<Shared<T>, Error>` and adapt callers accordingly. Since `Container::resolve` already returns `Result`, this refactor is feasible and keeps error handling explicit.

5. Errors
   - Add `ErrorKind::ResourceLimitExceeded` and a constructor `Error::resource_limit_exceeded(type_name, reason)`.

6. Macros
   - Update `macros.rs` to accept optional `limit = N` / `concurrency = N` syntax and expand to `bind_concrete_with_limits` or a helper that builds the `Limits` value.

7. Tests
   - Unit test (deny): configure `Limits { max_concurrent_creations: Some(1), policy: Policy::Deny }`, spawn two threads that call `resolve::<T>` for a binding whose provider sleeps for 200ms during construction. The second resolve should fail with `ErrorKind::ResourceLimitExceeded`.
   - Unit test (block): similar but with `policy: Policy::Block`; the second call should wait and succeed after the first completes.

8. Documentation
   - Update `README.md` with examples for `bind(limit = 5, ...)` and an explanation of policies.
   - Keep this file `roadmaps/RESOURCE_LIMIT.md` as the design document.

## API changes and compatibility

- Potential breaking change:
  - Changing `Factory::provide` to return `Result<Shared<T>, Error>` (and adapting `Container::resolve` to propagate errors) is a breaking change within the crate but should be isolated to internal call sites. It is preferable to return errors explicitly rather than panic.
- Minimal alternative: expose `try_acquire()` -> `bool` and panic on `Deny` when acquisition fails — this is unacceptable; therefore we recommend the explicit error-return approach.

## Example usage (README)

```rust
use sadi::{container, bind, Limits, Policy, Shared};

let c = container! {
    // Macro sugar will expand to bind_with_limits under the hood
    bind(limit = 5, LoggerService => |_| LoggerService::new())
};

// Or explicit API
c.bind_concrete_with_limits::<LoggerService, LoggerService, _>(
    |_| LoggerService::new(),
    Limits { max_concurrent_creations: Some(5), policy: Policy::Deny },
)?;
```

## Acceptance criteria

- [ ] Implementation added at `sadi/src/resource_limits.rs` and integrated into factories.
- [ ] `ErrorKind::ResourceLimitExceeded` added and used.
- [ ] Unit tests covering `Deny` and `Block` behaviors.
- [ ] README updated with example and minimal documentation.
- [ ] No regressions in existing tests (`cargo test -p sadi` passes).

## Future extensions (out of initial scope)

- Accurate live-instance counting using `Weak<T>` tracking or an optional `Limited<T>` return wrapper.
- Async-first integration using `tokio::Semaphore` and configurable timeouts.
- Advanced policies: queue + priority, evict/recycle.
- Observability: Prometheus metrics for limiter usage.

---

If you agree with the minimal approach (A) focused on concurrency limiting with `Deny`/`Block` policies, I will proceed with the implementation: add the `resource_limits` module, update `Factory` and `Container::bind_*` to accept limits, add tests, and update the README with examples. If you prefer the full variant (B), tell me and I'll outline the additional effort and steps.