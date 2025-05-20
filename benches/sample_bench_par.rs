use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use presquile::{Mode, apply};

macro_rules! test_file {
    ($file_name:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/resources/test/", $file_name)
    };
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("valid chaps", |b| {
        b.iter(|| {
            apply(
                black_box(test_file!("valid_chaps.cvs").into()),
                black_box(test_file!("audio.mp3").into()),
                Mode::Parallel,
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
