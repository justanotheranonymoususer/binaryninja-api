use binaryninja::binary_view::BinaryViewExt;
use binaryninja::headless::Session;
use criterion::{criterion_group, criterion_main, Criterion};
use rayon::prelude::*;
use std::path::PathBuf;
use warp_ninja::build_function;
use warp_ninja::cache::FunctionCache;

// These are the target files present in OUT_DIR
// Add the files to fixtures/bin
static TARGET_FILES: [&str; 1] = ["atox.obj"];

pub fn function_benchmark(c: &mut Criterion) {
    let session = Session::new().expect("Failed to initialize session");
    let out_dir = env!("OUT_DIR").parse::<PathBuf>().unwrap();
    for file_name in TARGET_FILES {
        let path = out_dir.join(file_name);
        let view = session.load(&path).expect("Failed to load view");

        let functions: Vec<_> = view.functions().iter().map(|f| f.to_owned()).collect();
        // Bench all functions sequentially
        c.bench_function("signature all functions", |b| {
            b.iter(|| {
                for func in &functions {
                    let _ = build_function(&func, &func.low_level_il().unwrap());
                }
            })
        });

        // Bench all functions in parallel
        let cache = FunctionCache::default();
        c.bench_function("signature all functions rayon", |b| {
            b.iter(|| {
                functions
                    .par_iter()
                    .map_with(cache.clone(), |par_cache, func| {
                        let llil = func.low_level_il().ok()?;
                        Some(par_cache.function(func.as_ref(), llil.as_ref()))
                    })
                    .collect::<Vec<_>>()
            })
        });
    }
}

criterion_group!(benches, function_benchmark);
criterion_main!(benches);
