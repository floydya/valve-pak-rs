use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::fs;
use std::hint::black_box;
use std::io::Read;
use tempfile::TempDir;
use valve_pak::VPK;

// Helper function to create test files of various sizes
fn create_test_files(
    base_path: &std::path::Path,
    file_count: usize,
    file_size: usize,
) -> anyhow::Result<()> {
    fs::create_dir_all(base_path)?;

    for i in 0..file_count {
        let file_data: Vec<u8> = (0..file_size).map(|j| ((i + j) % 256) as u8).collect();
        fs::write(base_path.join(format!("file_{i:04}.dat")), &file_data)?;
    }

    Ok(())
}

fn bench_vpk_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("vpk_creation");

    for file_count in [10, 50, 100].iter() {
        for file_size in [1024, 10240, 102400].iter() {
            // 1KB, 10KB, 100KB
            group.bench_with_input(
                BenchmarkId::new("create", format!("{file_count}files_{file_size}bytes")),
                &(file_count, file_size),
                |b, &(file_count, file_size)| {
                    b.iter(|| {
                        let temp_dir = TempDir::new().unwrap();
                        let source_dir = temp_dir.path().join("source");
                        create_test_files(&source_dir, *file_count, *file_size).unwrap();

                        let vpk = VPK::from_directory(&source_dir).unwrap();
                        black_box(vpk);
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_vpk_saving(c: &mut Criterion) {
    let mut group = c.benchmark_group("vpk_saving");

    for file_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("save", format!("{file_count}files")),
            file_count,
            |b, &file_count| {
                // Setup
                let temp_dir = TempDir::new().unwrap();
                let source_dir = temp_dir.path().join("source");
                create_test_files(&source_dir, file_count, 10240).unwrap(); // 10KB files
                let vpk = VPK::from_directory(&source_dir).unwrap();

                b.iter(|| {
                    let vpk_path = temp_dir
                        .path()
                        .join(format!("bench_{}.vpk", fastrand::u32(..)));
                    vpk.save(&vpk_path).unwrap();
                    black_box(vpk_path);
                });
            },
        );
    }

    group.finish();
}

fn bench_vpk_reading(c: &mut Criterion) {
    let mut group = c.benchmark_group("vpk_reading");

    // Setup: Create a VPK file to read from
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("test.vpk");

    create_test_files(&source_dir, 50, 10240).unwrap(); // 50 files of 10KB each
    let vpk = VPK::from_directory(&source_dir).unwrap();
    vpk.save(&vpk_path).unwrap();

    group.bench_function("open_vpk", |b| {
        b.iter(|| {
            let vpk = VPK::open(&vpk_path).unwrap();
            black_box(vpk);
        });
    });

    group.bench_function("list_files", |b| {
        let vpk = VPK::open(&vpk_path).unwrap();
        b.iter(|| {
            let files = vpk.list_files();
            black_box(files);
        });
    });

    group.finish();
}

fn bench_file_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_operations");

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("test.vpk");

    create_test_files(&source_dir, 10, 102400).unwrap(); // 10 files of 100KB each
    let vpk = VPK::from_directory(&source_dir).unwrap();
    vpk.save(&vpk_path).unwrap();

    let vpk = VPK::open(&vpk_path).unwrap();

    group.bench_function("get_file", |b| {
        b.iter(|| {
            let file = vpk.get_file("file_0000.dat").unwrap();
            black_box(file);
        });
    });

    group.bench_function("read_file_all", |b| {
        b.iter(|| {
            let mut file = vpk.get_file("file_0000.dat").unwrap();
            let data = file.read_all().unwrap();
            black_box(data);
        });
    });

    group.bench_function("read_file_chunks", |b| {
        b.iter(|| {
            let mut file = vpk.get_file("file_0000.dat").unwrap();
            let mut buffer = vec![0u8; 8192];
            let mut total = 0;

            loop {
                let bytes_read = file.read(&mut buffer).unwrap();
                if bytes_read == 0 {
                    break;
                }
                total += bytes_read;
            }

            black_box(total);
        });
    });

    group.bench_function("verify_file", |b| {
        b.iter(|| {
            let mut file = vpk.get_file("file_0000.dat").unwrap();
            let result = file.verify().unwrap();
            black_box(result);
        });
    });

    group.finish();
}

fn bench_large_vpk(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_vpk");
    group.sample_size(10); // Fewer samples for large benchmarks

    // Create a large VPK with many files
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("large.vpk");

    create_test_files(&source_dir, 1000, 1024).unwrap(); // 1000 files of 1KB each
    let vpk = VPK::from_directory(&source_dir).unwrap();
    vpk.save(&vpk_path).unwrap();

    group.bench_function("open_large_vpk", |b| {
        b.iter(|| {
            let vpk = VPK::open(&vpk_path).unwrap();
            black_box(vpk.file_count());
        });
    });

    group.bench_function("iterate_all_files", |b| {
        let vpk = VPK::open(&vpk_path).unwrap();
        b.iter(|| {
            let mut count = 0;
            for file_path in vpk.file_paths() {
                count += file_path.len();
            }
            black_box(count);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_vpk_creation,
    bench_vpk_saving,
    bench_vpk_reading,
    bench_file_operations,
    bench_large_vpk
);
criterion_main!(benches);
