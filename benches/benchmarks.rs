use std::{path::PathBuf, rc::Rc};

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn always_same_icon(c: &mut Criterion) {
    let program_file_path = std::env::args().next().expect("get program path");
    let program_file_path = PathBuf::from(&program_file_path);

    c.bench_function("file_icon_provider::get_file_icon", |b| {
        b.iter(|| file_icon_provider::get_file_icon(black_box(&program_file_path), black_box(32)))
    });
}

fn provider_always_same_icon(c: &mut Criterion) {
    let program_file_path = std::env::args().next().expect("get program path");
    let program_file_path = PathBuf::from(&program_file_path);
    let provider = &file_icon_provider::Provider::new(32, Rc::new).unwrap();

    c.bench_function("file_icon_provider::Provider::get_file_icon", |b| {
        b.iter(|| provider.get_file_icon(black_box(&program_file_path)))
    });
}

criterion_group!(benches, always_same_icon, provider_always_same_icon);
criterion_main!(benches);
