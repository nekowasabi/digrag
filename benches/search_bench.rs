//! Search benchmarks
//!
//! Benchmarks for the different search modes.

use criterion::{criterion_group, criterion_main, Criterion};
use digrag::config::{SearchConfig, SearchMode};
use digrag::search::Searcher;
use std::path::PathBuf;

/// Get the path to the .rag directory with indices
fn get_rag_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go up from cl-search to changelog
    path.push(".rag");
    path
}

/// Check if the .rag directory exists with required files
fn rag_dir_available() -> bool {
    let rag_dir = get_rag_dir();
    rag_dir.exists()
        && rag_dir.join("bm25_index.json").exists()
        && rag_dir.join("docstore.json").exists()
}

fn benchmark_bm25_search(c: &mut Criterion) {
    if !rag_dir_available() {
        println!("Skipping BM25 benchmark: .rag directory not available");
        return;
    }

    let rag_dir = get_rag_dir();
    let searcher = match Searcher::new(&rag_dir) {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping BM25 benchmark: Could not load indices");
            return;
        }
    };

    const TEST_QUERIES: &[&str] = &["メモ", "worklog", "設定", "コマンド", "実装"];

    c.bench_function("bm25_search_throughput", |b| {
        let mut query_idx = 0;
        b.iter(|| {
            let query = TEST_QUERIES[query_idx % TEST_QUERIES.len()];
            let config = SearchConfig::new()
                .with_mode(SearchMode::Bm25)
                .with_top_k(10)
                .with_rewrite(false);

            let _ = searcher.search(query, &config);
            query_idx += 1;
        });
    });
}

fn benchmark_semantic_search(c: &mut Criterion) {
    if !rag_dir_available() {
        println!("Skipping semantic benchmark: .rag directory not available");
        return;
    }

    let rag_dir = get_rag_dir();
    let searcher = match Searcher::new(&rag_dir) {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping semantic benchmark: Could not load indices");
            return;
        }
    };

    const TEST_QUERIES: &[&str] = &["メモ", "worklog", "設定", "コマンド", "実装"];

    c.bench_function("semantic_search_throughput", |b| {
        let mut query_idx = 0;
        b.iter(|| {
            let query = TEST_QUERIES[query_idx % TEST_QUERIES.len()];
            let config = SearchConfig::new()
                .with_mode(SearchMode::Semantic)
                .with_top_k(10)
                .with_rewrite(false);

            let _ = searcher.search(query, &config);
            query_idx += 1;
        });
    });
}

fn benchmark_hybrid_search(c: &mut Criterion) {
    if !rag_dir_available() {
        println!("Skipping hybrid benchmark: .rag directory not available");
        return;
    }

    let rag_dir = get_rag_dir();
    let searcher = match Searcher::new(&rag_dir) {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping hybrid benchmark: Could not load indices");
            return;
        }
    };

    const TEST_QUERIES: &[&str] = &["メモ", "worklog", "設定", "コマンド", "実装"];

    c.bench_function("hybrid_search_throughput", |b| {
        let mut query_idx = 0;
        b.iter(|| {
            let query = TEST_QUERIES[query_idx % TEST_QUERIES.len()];
            let config = SearchConfig::new()
                .with_mode(SearchMode::Hybrid)
                .with_top_k(10)
                .with_rewrite(false);

            let _ = searcher.search(query, &config);
            query_idx += 1;
        });
    });
}

criterion_group!(
    benches,
    benchmark_bm25_search,
    benchmark_semantic_search,
    benchmark_hybrid_search
);
criterion_main!(benches);
