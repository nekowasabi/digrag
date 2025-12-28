# Lessons Learned: Content Extraction Feature

## Overview

This document captures the lessons learned during the implementation of the content extraction feature for digrag (Process 8, 9, 200, 300).

---

## Technical Insights

### 1. OpenRouter API Integration Pattern

**Category**: Technical / Best Practice
**Importance**: High

**Pattern**:
```rust
// Recommended: Use structured client with retry logic
pub struct OpenRouterClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}
```

**Key Points**:
- Bearer token authentication via `Authorization` header
- Include `HTTP-Referer` and `X-Title` headers for API attribution
- Implement exponential backoff for rate limits (429 responses)
- Parse `Retry-After` header for rate limit guidance
- Distinguish between network errors and API errors for proper handling

**Antipattern to Avoid**:
- Synchronous API calls blocking the event loop
- Missing retry logic for transient failures
- Hardcoded timeout values

---

### 2. LRU Cache with TTL

**Category**: Technical / Performance
**Importance**: High

**Pattern**:
```rust
pub struct LruCache<V: Clone> {
    entries: Arc<RwLock<HashMap<String, CacheEntry<V>>>>,
    max_size: usize,
    ttl: Duration,
}
```

**Key Points**:
- Use content hash + model as cache key for deterministic lookups
- Thread-safe access with `RwLock` for concurrent read operations
- TTL-based expiration to prevent stale summaries
- Track cache statistics (hits, misses, evictions) for monitoring
- LRU eviction when at capacity

**Recommended Settings**:
- Summary cache: 200 entries, 2 hour TTL
- Default cache: 100 entries, 1 hour TTL

---

### 3. Error Categorization Strategy

**Category**: Technical / Observability
**Importance**: Medium

**Pattern**:
```rust
pub enum ErrorCategory {
    Network,
    Authentication,
    RateLimit,
    ModelNotFound,
    InvalidRequest,
    ServerError,
    ParseError,
    Timeout,
    Unknown,
}
```

**Key Points**:
- Categorize errors for pattern analysis
- Track error frequency by category
- Enable automated alerting on critical error spikes
- Store recent errors for debugging

---

### 4. Fallback Strategy

**Category**: Technical / Reliability
**Importance**: Critical

**Implementation**:
```rust
match self.llm_summary(...).await {
    Ok(summary) => summary,
    Err(e) => {
        warn!(error = %e, "LLM summarization failed, falling back to rule-based");
        self.rule_based_summary(content, 200)
    }
}
```

**Key Points**:
- Always have a fallback when external APIs fail
- Log failures for later analysis
- Rule-based summarization as reliable fallback
- Graceful degradation over hard failures

---

## Process Insights

### 1. TDD Effectiveness

**Category**: Process
**Importance**: High

**Observations**:
- Writing tests first clarified API contract requirements
- Mock servers (wiremock) enabled reliable async testing
- Test-first approach caught edge cases early (empty responses, rate limits)
- Snapshot testing useful for complex output validation

**Metrics**:
- Test coverage increased from 137 to 376 tests
- All Process implementations validated before moving forward

---

### 2. Incremental Module Design

**Category**: Process / Architecture
**Importance**: Medium

**Pattern**:
```
src/extract/
├── mod.rs           # Core types (ExtractionStrategy, TruncationConfig)
├── changelog.rs     # Changelog-specific extraction
├── summarizer.rs    # Summarization strategies
├── openrouter_client.rs  # HTTP client
├── cache.rs         # LRU caching
└── telemetry.rs     # Usage statistics
```

**Key Points**:
- Each module has single responsibility
- Public interface in mod.rs, implementation in submodules
- Easy to test each component in isolation
- Clear dependency graph

---

## Antipatterns Identified

### 1. Unused Field Warnings

**Issue**: Rust warns about unused struct fields
**Solution**: Use `#[allow(dead_code)]` for intentionally reserved fields, or remove unused fields

### 2. Blocking on Async Code

**Issue**: Using `block_on` in sync contexts
**Solution**: Make calling code async, or use dedicated runtime

### 3. Missing Provider Configuration

**Issue**: Hardcoded provider preferences
**Solution**: Use `ProviderConfig` struct with flexible routing options

---

## Best Practices Established

1. **Configuration Hierarchy**: MCP params > Environment > Config file > Defaults
2. **Model Specification**: Use `provider/model` format (e.g., `cerebras/llama-3.3-70b`)
3. **Error Handling**: Distinguish network vs API errors, provide actionable messages
4. **Telemetry**: Track all API calls for usage analysis and cost optimization
5. **Caching**: Always cache expensive operations (API calls, embeddings)

---

## Future Improvements

1. **Rate Limiting Client-Side**: Implement proactive rate limiting before hitting API limits
2. **Circuit Breaker Pattern**: Fail fast when API is consistently unavailable
3. **Streaming Responses**: Support streaming for long-running summarizations
4. **Cost Tracking**: Track API costs by model and operation type

---

## References

- OpenRouter API Documentation: https://openrouter.ai/docs
- Rust async book: https://rust-lang.github.io/async-book/
- wiremock crate: https://docs.rs/wiremock
