# Software Defined Memory (SDM) - Technical Documentation

## Overview

Software Defined Memory (SDM) is a revolutionary memory management system for Palindrome VM that replaces traditional OS-based virtual memory with a policy-driven, temporally-aware storage hierarchy. SDM makes the tape truly infinite while optimizing for the unique access patterns of a reversible VM.

## Architecture

### Core Components

```
src/tape/sdm/
├── mod.rs           # Main SDM tape implementation
├── address_space.rs # Virtual address space management
├── backends.rs      # Storage backend implementations
├── page_table.rs    # Smart page table with versioning
├── policy.rs        # Policy engine for placement decisions
└── predictor.rs     # Access pattern prediction
```

### Storage Hierarchy

1. **DRAM Cache** (100ns latency)
   - LRU cache for hot pages
   - Configurable size (default 100MB)
   - Zero-copy access

2. **Local SSD** (100μs latency)
   - Persistent storage for warm data
   - File-based backend
   - Automatic space management

3. **Network Storage** (planned)
   - Distributed tape segments
   - Remote page access
   - Replication support

4. **Cold Storage (S3)** (planned)
   - Long-term archive
   - Compressed historical data
   - Cost-optimized storage

## Key Features

### 1. Virtual Address Space

```rust
// Define memory regions with policy hints
let mut space = VirtualAddressSpace::new(4096);
space.define_region(0, 1024*1024, PolicyHint::Code, Some("code"));
space.define_region(1024*1024, 1024*1024, PolicyHint::Stack, Some("stack"));
```

Policy hints guide placement decisions:
- `Code`: Keep in fastest memory
- `Stack`: High locality, frequent access
- `Heap`: Variable access patterns
- `History`: Append-only, compress when cold
- `Checkpoint`: Durability matters

### 2. Policy Engine

SDM uses declarative policies instead of manual memory management:

```rust
// Example: Balanced policy
MemoryPolicy {
    rules: vec![
        // Code always in DRAM
        PlacementRule {
            condition: Condition::HasHint(PolicyHint::Code),
            action: PlacementAction::PlaceIn(StorageTier::Dram),
            priority: 100,
        },
        // Hot data in DRAM
        PlacementRule {
            condition: Condition::FrequencyAbove(50.0),
            action: PlacementAction::PlaceIn(StorageTier::Dram),
            priority: 90,
        },
        // Checkpoints on SSD for durability
        PlacementRule {
            condition: Condition::HasHint(PolicyHint::Checkpoint),
            action: PlacementAction::PlaceIn(StorageTier::Local),
            priority: 80,
        },
    ],
}
```

### 3. Temporal Awareness

SDM understands time-travel access patterns unique to Palindrome:

```rust
// Checkpoint prefetching
pub fn record_checkpoint(&mut self, name: String) {
    // Record recently accessed pages as potential rewind targets
}

// Rewind prediction
pub fn predict_rewind_targets(&self) -> Vec<i64> {
    // Use historical patterns to predict pages needed after rewind
}
```

### 4. Access Pattern Learning

The predictor learns from access patterns:

1. **Sequential Detection**: Identifies sequential access for prefetching
2. **Markov Chain**: Predicts next page based on transition probabilities
3. **Temporal Patterns**: Learns checkpoint/rewind correlations

### 5. Page Versioning

Every page maintains version history:

```rust
pub struct PageEntry {
    pub page_num: i64,
    pub location: StorageLocation,
    pub version: u64,
    pub stats: AccessStats,
}

pub struct HistoricalPage {
    pub version: u64,
    pub location: StorageLocation,
    pub replaced_at: u64,
}
```

This enables O(1) time-travel to any point in history.

## Usage

### Basic Operations

```rust
// Create SDM tape with default config
let tape = SdmTape::new();

// Write data
tape.write(position, data)?;

// Read current data
let data = tape.read(position, length)?;

// Read historical data
let old_data = tape.read_at_time(position, length, timestamp)?;

// Create checkpoint
tape.checkpoint("before_experiment")?;
```

### Configuration

```rust
let config = SdmConfig {
    page_size: 4096,                    // 4KB pages
    dram_cache_size: 100 * 1024 * 1024, // 100MB DRAM
    prefetch_depth: 5,                  // Prefetch 5 pages ahead
    enable_compression: true,
    compression_threshold: 64 * 1024,   // Compress > 64KB
};

let tape = SdmTape::with_config(config);
```

### Custom Policies

```rust
// Create a debug policy - everything in DRAM
let debug_policy = MemoryPolicy::debug();

// Create a production policy - cost optimized
let prod_policy = MemoryPolicy::balanced();

// Create custom policy
let custom_policy = MemoryPolicy {
    rules: vec![/* your rules */],
    compression: CompressionPolicy { /* settings */ },
    prefetch: PrefetchPolicy { /* settings */ },
    profile: PolicyProfile::Custom,
};
```

## Integration with Palindrome VM

### Zero-Copy Time Travel

When integrated with the tape's trail system:

1. **Write Operation**:
   - Tape records trail operation with old value
   - SDM increments page version
   - Old version preserved in appropriate tier
   - New version written to location per policy

2. **Rewind Operation**:
   - Tape replays trail backwards
   - SDM serves historical page versions
   - Prefetcher loads likely-needed pages
   - Access pattern recorded for learning

### Performance Characteristics

- **Sequential Access**: Detected and prefetched automatically
- **Random Access**: Markov chain prediction improves over time
- **Checkpoint/Rewind**: O(1) with prefetching for common patterns
- **Cold Data**: Transparent compression reduces storage costs

## Future Enhancements

### Short Term
1. **Async Prefetching**: Background threads for prefetch operations
2. **S3 Backend**: Implementation of cold storage tier
3. **Config Files**: TOML-based configuration
4. **Metrics**: Detailed performance statistics

### Long Term
1. **Distributed Tape**: Network backend for multi-node operation
2. **GPU Memory**: Support for GPU-accelerated operations
3. **Persistent Memory**: Integration with Intel Optane/CXL
4. **Smart Compression**: Content-aware compression algorithms

## Design Principles

1. **Transparency**: Applications see infinite tape, unaware of tiers
2. **Policy-Driven**: Declarative rules, not imperative code
3. **Learning**: Adapts to access patterns over time
4. **Temporal-First**: Time-travel is a primary concern, not afterthought
5. **Zero-Copy**: Minimize data movement through smart placement

## Implementation Notes

### Thread Safety
All SDM components use `Arc<RwLock<>>` for thread-safe access. Multiple readers can access pages concurrently.

### Page Allocation
Pages are allocated on first write. Unallocated pages read as zeros, maintaining tape semantics.

### Compression
Currently uses zstd for compression. Pages are compressed based on policy when moved to cold storage.

### Testing
Comprehensive test suite covers:
- Cross-page access
- Policy evaluation
- Sequential detection
- Access prediction
- Storage backends

Run tests with: `cargo test sdm --lib`

## Conclusion

SDM transforms Palindrome VM's infinite tape from a theoretical abstraction into a practical reality. By understanding and optimizing for temporal access patterns, SDM makes time-travel debugging not just possible but performant. The policy-driven approach eliminates manual memory management while providing better performance than traditional virtual memory systems.