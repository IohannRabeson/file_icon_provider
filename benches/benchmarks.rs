use std::rc::Rc;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn always_same_icon(c: &mut Criterion) {
    let file_path = locate_cargo_manifest::locate_manifest().expect("locate Cargo.toml");

    c.bench_function("file_icon_provider::get_file_icon", |b| {
        b.iter(|| file_icon_provider::get_file_icon(black_box(&file_path), black_box(32)))
    });
}

fn provider_always_same_icon(c: &mut Criterion) {
    let file_path = locate_cargo_manifest::locate_manifest().expect("locate Cargo.toml");
    let provider = &file_icon_provider::Provider::new(32, Rc::new).unwrap();

    c.bench_function("file_icon_provider::Provider::get_file_icon", |b| {
        b.iter(|| provider.get_file_icon(black_box(&file_path)))
    });
}

criterion_group!(benches, always_same_icon, provider_always_same_icon);
criterion_main!(benches);
