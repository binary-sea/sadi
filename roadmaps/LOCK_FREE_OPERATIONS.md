# Lock-Free Operations — Design & Implementation Plan

This document outlines the strategy for implementing lock-free operations in SaDi to minimize contention and improve performance in high-concurrency scenarios. The goal is to replace traditional locking primitives with lock-free or wait-free data structures while maintaining correctness and safety.

Table of contents
- Goals
- Current State Analysis
- Design Principles
- Implementation Strategies
- Phased Implementation Plan
- Performance Benchmarks
- Testing Strategy
- Migration Path
- Future Optimizations

---

## Goals

- **Minimize Lock Contention**: Reduce or eliminate blocking in hot paths (singleton reads, factory lookups)
- **Improve Concurrent Throughput**: Allow multiple threads to resolve services simultaneously without global locks
- **Maintain Correctness**: Ensure singleton semantics and circular dependency detection remain intact
- **Backward Compatibility**: Feature-gated implementation behind `thread-safe` flag
- **Zero-Cost Abstraction**: Non-thread-safe mode should remain unchanged
- **Measurable Gains**: Target 5-10x throughput improvement in concurrent resolution scenarios

## Current State Analysis

### Existing Lock Points

1. **Factories Map** (`container.rs`)
   - Current: `RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>`
   - Bottleneck: Write lock required for registration, read lock for all resolutions
   - Contention: High during concurrent service resolution
   - Impact: Global read lock blocks all concurrent resolves

2. **Singleton Instance Cache** (`factory.rs`)
   - Current: `Mutex<Option<Shared<T>>>`
   - Bottleneck: Exclusive lock for first initialization, lock for all subsequent reads
   - Contention: Medium to high for frequently-accessed singletons
   - Impact: Each singleton type has independent lock (better than global, but still suboptimal)

3. **Resolve Guard** (`resolve_guard.rs`)
   - Current: Thread-local or container-local tracking
   - Bottleneck: Minimal (already thread-local in practice)
   - Impact: Low contention

### Performance Characteristics (Current Implementation)

| Operation | Thread-Safe Mode | Single-Threaded Mode |
|-----------|-----------------|---------------------|
| Factory Registration | ~100-500ns (write lock) | ~50-100ns (RefCell) |
| Singleton Resolution (cached) | ~50-100ns (mutex read) | ~10-20ns (RefCell) |
| Transient Resolution | ~80-150ns (read lock + factory call) | ~30-50ns |
| Concurrent Resolution (16 threads) | ~1,000-2,000ns (lock contention) | N/A |

## Design Principles

1. **Progressive Enhancement**: Implement lock-free structures incrementally, validating each step
2. **Feature Gating**: Lock-free mode as optional feature (`lock-free` or enhanced `thread-safe`)
3. **Double-Checked Locking Pattern**: Use atomic operations for fast-path checks before slow initialization
4. **Immutable-Friendly**: Favor write-once, read-many patterns for singleton caching
5. **Lock-Free Reads**: Prioritize lock-free reads over lock-free writes (reads are 100x more common)
6. **Memory Ordering**: Use appropriate atomic orderings (Acquire/Release, not always SeqCst)

## Implementation Strategies

### Strategy 1: Lock-Free Singleton Caching (Priority: HIGH)

**Current Problem**: `Mutex<Option<Shared<T>>>` requires lock even for cached singleton reads

**Solution**: Replace with `OnceLock<Shared<T>>` (std lib, since Rust 1.70)

**Benefits**:
- Lock-free reads after initialization
- Write-once semantics perfect for singletons
- No external dependencies
- Built-in thread-safety guarantees

**Implementation**:
```rust
// types.rs
#[cfg(feature = "thread-safe")]
pub type InstanceCell<T> = std::sync::OnceLock<Shared<T>>;

// factory.rs
pub fn provide(&self, container: &Container) -> Shared<T> {
    if self.singleton {
        self.instance
            .get_or_init(|| (self.provider)(container))
            .clone()
    } else {
        (self.provider)(container)
    }
}
```

**Expected Improvement**: 5-10x faster singleton reads (from ~50-100ns to ~5-10ns)

**Risks**: 
- Multiple threads may race to initialize, but `OnceLock` handles this safely
- Provider function must be idempotent or initialization must be acceptable multiple times (discarded on race loss)

**Mitigation**: Document that singleton provider functions may be called multiple times during concurrent first-access, but only one result is kept

---

### Strategy 2: Lock-Free Factory Map (Priority: HIGH)

**Current Problem**: `RwLock<HashMap>` requires read lock for every resolution, write lock for registration

**Solution**: Replace with `DashMap<TypeId, Box<dyn Any + Send + Sync>>`

**Benefits**:
- Lock-free concurrent reads
- Sharded internal locks (16-64 shards by default)
- Near-HashMap performance for single-threaded access
- Battle-tested in production (used by tokio, actix)

**Implementation**:
```rust
// Cargo.toml
[dependencies]
dashmap = { version = "6.0", optional = true }

[features]
lock-free = ["thread-safe", "dashmap"]

// types.rs
#[cfg(all(feature = "thread-safe", feature = "lock-free"))]
pub type FactoriesMap = dashmap::DashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>;

#[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
pub type FactoriesMap = std::sync::RwLock<HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>>;

// container.rs - methods become simpler
fn bind_internal<T: ?Sized + 'static>(
    &self,
    provider: Provider<T>,
    singleton: bool,
) -> Result<(), Error> {
    let id = TypeId::of::<T>();
    
    if self.factories.contains_key(&id) {
        return Err(Error::AlreadyBound(std::any::type_name::<T>().to_string()));
    }

    let factory = Box::new(Factory::new(provider, singleton));
    self.factories.insert(id, factory);
    Ok(())
}
```

**Expected Improvement**: 3-5x faster concurrent resolutions (from ~80-150ns to ~20-40ns)

**Risks**:
- Adds external dependency
- Binary size increase (~100KB)
- Different API surface from HashMap (entry API works differently)

**Mitigation**: Make it optional behind `lock-free` feature flag, keep RwLock as default

---

### Strategy 3: ArcSwap for Hot-Swappable Singletons (Priority: MEDIUM)

**Current Problem**: `OnceLock` is write-once, but some use cases might want singleton replacement

**Solution**: Use `arc-swap` crate for lock-free atomic swapping

**Benefits**:
- Lock-free reads (just an atomic load)
- Allows singleton replacement/reset
- ~2-3ns read latency

**Implementation**:
```rust
// Cargo.toml
[dependencies]
arc-swap = { version = "1.7", optional = true }

// types.rs
#[cfg(all(feature = "thread-safe", feature = "lock-free-swap"))]
pub type InstanceCell<T> = arc_swap::ArcSwap<Option<Shared<T>>>;

// factory.rs
pub fn provide(&self, container: &Container) -> Shared<T> {
    if self.singleton {
        // Fast path: lock-free read
        let guard = self.instance.load();
        if let Some(inst) = guard.as_ref() {
            return inst.clone();
        }
        
        // Slow path: initialize
        drop(guard); // Release the guard
        let new_inst = (self.provider)(container);
        
        // Compare-and-swap pattern
        let new_arc = Arc::new(Some(new_inst.clone()));
        let _ = self.instance.compare_and_swap(&self.instance.load(), new_arc);
        
        // Always return from current value (handles race)
        self.instance.load().as_ref().unwrap().clone()
    } else {
        (self.provider)(container)
    }
}
```

**Expected Improvement**: ~2-3ns singleton reads (best possible)

**Trade-offs**: More complex than `OnceLock`, requires external dependency

**Use Case**: Advanced scenarios where singleton replacement is needed (rare)

---

### Strategy 4: Lock-Free Circular Dependency Tracking (Priority: LOW)

**Current State**: `ResolveGuard` uses thread-local or container-local `HashSet`

**Optimization**: Use lock-free stack-based tracking with thread-local storage

**Benefits**:
- Already mostly lock-free via TLS
- Minor memory optimization possible

**Priority**: LOW (not a bottleneck in practice)

## Phased Implementation Plan

### Phase 1: Foundation & Benchmarking (Week 1)
**Goals**: Establish baseline, add benchmarking infrastructure

- [ ] Create benchmark suite using `criterion`
  - [ ] Benchmark: Singleton resolution (cached)
  - [ ] Benchmark: Transient resolution
  - [ ] Benchmark: Concurrent resolution (2, 4, 8, 16, 32 threads)
  - [ ] Benchmark: Registration performance
  - [ ] Benchmark: Mixed workload (90% read, 10% write)
  
- [ ] Document baseline performance metrics
- [ ] Create performance regression tests
- [ ] Set up CI benchmarking (GitHub Actions)

**Success Criteria**: Reproducible benchmark suite with <5% variance

---

### Phase 2: Lock-Free Singleton Caching (Week 2)
**Goals**: Replace `Mutex<Option<T>>` with `OnceLock<T>`

- [ ] Update `InstanceCell` type alias for `thread-safe` feature
- [ ] Refactor `Factory::provide` to use `OnceLock::get_or_init`
- [ ] Update `Factory::new` initialization
- [ ] Add tests for concurrent singleton initialization
- [ ] Run benchmark suite, compare to baseline
- [ ] Update documentation

**Success Criteria**: 
- All existing tests pass
- 5x+ improvement in singleton read benchmarks
- No regressions in transient performance

---

### Phase 3: Lock-Free Factory Map (Week 3-4)
**Goals**: Replace `RwLock<HashMap>` with `DashMap`

- [ ] Add `dashmap` dependency (optional)
- [ ] Create `lock-free` feature flag
- [ ] Update `FactoriesMap` type alias with feature conditions
- [ ] Refactor `Container::bind_internal` for DashMap API
- [ ] Refactor `Container::resolve_internal` for DashMap API
- [ ] Refactor `Container::has` for DashMap API
- [ ] Add feature-gated tests
- [ ] Run full benchmark suite
- [ ] Update documentation and examples

**Success Criteria**:
- All tests pass with both `lock-free` enabled and disabled
- 3x+ improvement in concurrent resolution benchmarks
- No API changes for users (internal only)

---

### Phase 4: Advanced Optimizations (Week 5) [OPTIONAL]
**Goals**: Explore `arc-swap` and other micro-optimizations

- [ ] Prototype `arc-swap` implementation
- [ ] Benchmark against `OnceLock` implementation
- [ ] Evaluate trade-offs (complexity vs. performance gain)
- [ ] Decide: ship as experimental feature or defer
- [ ] Memory layout optimizations
- [ ] False sharing elimination

**Success Criteria**: Evidence-based decision on advanced features

---

### Phase 5: Documentation & Stabilization (Week 6)
**Goals**: Production-ready release

- [ ] Update README with performance characteristics
- [ ] Create migration guide for users
- [ ] Write blog post about lock-free implementation
- [ ] Add performance tuning guide
- [ ] Document memory ordering guarantees
- [ ] Code review and audit
- [ ] Release candidate testing
- [ ] Update roadmap to mark as complete

**Success Criteria**: Confident production release

## Performance Benchmarks

### Target Metrics

| Operation | Current (µs) | Phase 2 Target (µs) | Phase 3 Target (µs) | Improvement |
|-----------|-------------|---------------------|---------------------|-------------|
| Singleton read (1 thread) | 0.05 | 0.005 | 0.005 | 10x |
| Singleton read (16 threads) | 1.5 | 0.15 | 0.05 | 30x |
| Transient resolve (1 thread) | 0.08 | 0.08 | 0.04 | 2x |
| Transient resolve (16 threads) | 2.0 | 1.8 | 0.30 | 6.6x |
| Registration | 0.30 | 0.30 | 0.15 | 2x |

### Benchmark Suite Structure

```rust
// benches/lock_free.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use sadi::{Container, container, bind};

fn singleton_cached_read(c: &mut Criterion) {
    let container = container! {
        bind(singleton ExpensiveService => |_| ExpensiveService::new())
    };
    
    // Warm up the singleton
    let _ = container.resolve::<ExpensiveService>().unwrap();
    
    c.bench_function("singleton_cached_read", |b| {
        b.iter(|| {
            black_box(container.resolve::<ExpensiveService>().unwrap())
        })
    });
}

fn concurrent_singleton_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_singleton_reads");
    
    for thread_count in [2, 4, 8, 16, 32].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            thread_count,
            |b, &thread_count| {
                let container = Arc::new(container! {
                    bind(singleton ExpensiveService => |_| ExpensiveService::new())
                });
                
                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|_| {
                            let c = container.clone();
                            std::thread::spawn(move || {
                                black_box(c.resolve::<ExpensiveService>().unwrap())
                            })
                        })
                        .collect();
                    
                    for h in handles {
                        h.join().unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, singleton_cached_read, concurrent_singleton_reads);
criterion_main!(benches);
```

## Testing Strategy

### Unit Tests

1. **Lock-Free Singleton Initialization**
   - [ ] Test: Concurrent first access creates exactly one instance
   - [ ] Test: Subsequent accesses return same instance
   - [ ] Test: Thread safety under extreme contention (1000 threads)
   - [ ] Test: Memory visibility across threads (no stale reads)

2. **Lock-Free Factory Map**
   - [ ] Test: Concurrent registration of different types
   - [ ] Test: Concurrent resolution of multiple types
   - [ ] Test: Race between registration and resolution
   - [ ] Test: Type safety preserved under concurrency

3. **Correctness Preservation**
   - [ ] Test: Circular dependency detection still works
   - [ ] Test: Error handling unchanged
   - [ ] Test: Complex dependency graphs resolve correctly
   - [ ] Test: Singleton semantics maintained

### Integration Tests

1. **Real-World Scenarios**
   - [ ] Web server with concurrent request handling
   - [ ] Background task processor with shared services
   - [ ] Plugin system with dynamic registration
   - [ ] Large dependency graph (50+ types)

### Stress Tests

1. **Contention Testing**
   - [ ] 100 threads simultaneously resolving same singleton
   - [ ] 1000 types registered concurrently
   - [ ] Mixed read/write workload (90/10, 95/5, 99/1)
   - [ ] Sustained high load (1M resolutions/second)

### Property-Based Tests

1. **Quickcheck/Proptest**
   - [ ] Property: Singleton always returns same pointer
   - [ ] Property: Transient always returns different pointer
   - [ ] Property: No data races under arbitrary concurrent operations
   - [ ] Property: Type safety never violated

## Migration Path

### For Library Maintainers

**Phase 1-2 (Transparent to Users)**:
- `OnceLock` is drop-in replacement in thread-safe mode
- No API changes required
- Automatic performance improvement

**Phase 3 (Opt-in Feature)**:
```toml
# Current (default)
[dependencies]
sadi = "0.3"

# Lock-free mode (opt-in)
[dependencies]
sadi = { version = "0.3", features = ["lock-free"] }
```

### For End Users

**No Code Changes Required**:
```rust
// Same code works with both implementations
let container = container! {
    bind(singleton DatabaseService => |_| DatabaseService::new())
};

let db = container.resolve::<DatabaseService>().unwrap();
```

**Performance Tuning Guide**:
- Enable `lock-free` feature for high-concurrency workloads
- Use standard `thread-safe` for low-contention scenarios
- Benchmark your specific workload to determine benefit

### Breaking Changes

**None Expected** — This is a purely internal implementation change

## Future Optimizations

### Beyond Phase 5

1. **Adaptive Locking**
   - Detect contention at runtime
   - Switch between RwLock and DashMap dynamically
   - Profile-guided optimization

2. **Wait-Free Operations**
   - Explore wait-free algorithms for registration
   - Bounded latency guarantees
   - Real-time system compatibility

3. **NUMA-Aware Allocation**
   - Allocate singletons on NUMA node of first accessor
   - Reduce cross-socket traffic
   - Optimize for modern server CPUs

4. **Lockless Resolve Guard**
   - Stack-based circular detection without heap allocation
   - Thread-local optimization
   - Zero-allocation fast path

5. **Cache-Line Optimization**
   - Align singleton instances to cache lines
   - Eliminate false sharing
   - Optimize memory layout for CPU caches

6. **Compile-Time Resolution**
   - Macro-based dependency graph analysis
   - Generate specialized resolve functions
   - Zero runtime overhead for static graphs

## References & Resources

### Academic Papers
- "Simple, Fast, and Practical Non-Blocking and Blocking Concurrent Queue Algorithms" (Michael & Scott, 1996)
- "Lock-Free Data Structures" (Maged M. Michael, 2004)
- "Hazard Pointers: Safe Memory Reclamation for Lock-Free Objects" (Maged M. Michael, 2004)

### Rust Ecosystem
- `crossbeam` - Lock-free data structures library
- `dashmap` - Concurrent HashMap implementation
- `arc-swap` - Lock-free Arc swapping
- `parking_lot` - Fast synchronization primitives
- `once_cell` / `OnceLock` - Lazy initialization (std lib)

### Benchmarking
- `criterion` - Statistical benchmarking framework
- `dhat` - Dynamic heap analysis tool
- `perf` - Linux performance profiler
- `flamegraph` - Visualization for profiling data

### Books
- "The Art of Multiprocessor Programming" (Herlihy & Shavit)
- "Rust Atomics and Locks" (Mara Bos, 2023)

---

## Success Metrics

### Performance Targets
- [ ] 5x improvement in singleton reads (single-threaded)
- [ ] 10x improvement in singleton reads (16-thread concurrent)
- [ ] 3x improvement in transient resolution (concurrent)
- [ ] 2x improvement in registration performance
- [ ] <1% regression in single-threaded performance

### Quality Targets
- [ ] 100% test coverage for lock-free code paths
- [ ] Zero unsafe code (prefer safe abstractions)
- [ ] No Miri violations
- [ ] No data races detected by ThreadSanitizer
- [ ] No memory leaks (verified with Valgrind/ASAN)

### Adoption Targets
- [ ] Documentation covers lock-free feature comprehensively
- [ ] At least 3 example projects demonstrating benefits
- [ ] Performance comparison published
- [ ] Community feedback incorporated

---

**Status**: 📋 Planning Phase  
**Priority**: High  
**Estimated Effort**: 6 weeks  
**Dependencies**: None  
**Risk Level**: Medium (external dependencies, performance validation needed)

**Next Steps**:
1. Review and approve roadmap
2. Set up benchmark infrastructure
3. Begin Phase 1 implementation
4. Regular progress updates (weekly)

---

*Last Updated: January 5, 2026*  
*Author: João Pedro Martins*  
*Version: 1.0*
