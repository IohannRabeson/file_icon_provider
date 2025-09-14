use std::{hint::black_box, path::PathBuf, rc::Rc};

use criterion::{Criterion, criterion_group, criterion_main};
use file_icon_provider::Caching;

fn always_same_icon(c: &mut Criterion) {
    let program_file_path = std::env::args().next().expect("get program path");
    let program_file_path = PathBuf::from(&program_file_path);

    c.bench_function("file_icon_provider::get_file_icon", |b| {
        b.iter(|| file_icon_provider::get_file_icon(black_box(&program_file_path), black_box(32)))
    });
}

fn provider_always_same_icon_caching_enabled(c: &mut Criterion) {
    let program_file_path = std::env::args().next().expect("get program path");
    let program_file_path = PathBuf::from(&program_file_path);
    let provider = &file_icon_provider::Provider::new(32, Caching::Enabled, Rc::new).unwrap();

    c.bench_function("file_icon_provider::Provider::get_file_icon caching enabled", |b| {
        b.iter(|| provider.get_file_icon(black_box(&program_file_path)))
    });
}

fn provider_always_same_icon_caching_disabled(c: &mut Criterion) {
    let program_file_path = std::env::args().next().expect("get program path");
    let program_file_path = PathBuf::from(&program_file_path);
    let provider = &file_icon_provider::Provider::new(32, Caching::Disabled, Rc::new).unwrap();

    c.bench_function("file_icon_provider::Provider::get_file_icon caching disabled", |b| {
        b.iter(|| provider.get_file_icon(black_box(&program_file_path)))
    });
}

criterion_group!(benches, always_same_icon, provider_always_same_icon_caching_enabled, provider_always_same_icon_caching_disabled);
criterion_main!(benches);
