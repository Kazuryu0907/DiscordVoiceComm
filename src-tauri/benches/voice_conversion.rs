use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use discordvoicecommv1_lib::vc::voice_manager::{i16tof32, convert_voice_data};

fn generate_test_data(samples: usize) -> Vec<i16> {
    (0..samples)
        .map(|i| ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 32767.0) as i16)
        .collect()
}

fn bench_i16tof32(c: &mut Criterion) {
    let mut group = c.benchmark_group("i16tof32");
    
    for size in [256, 512, 1024, 2048, 4096].iter() {
        let data = generate_test_data(*size);
        
        group.bench_with_input(BenchmarkId::new("samples", size), size, |b, _| {
            b.iter(|| {
                i16tof32(black_box(data.clone()))
            })
        });
    }
    
    group.finish();
}

fn bench_convert_voice_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("convert_voice_data");
    
    for size in [256, 512, 1024, 2048, 4096].iter() {
        let data = generate_test_data(*size);
        
        // Test with different volume levels
        for volume in [0.1, 0.5, 1.0, 1.5, 2.0].iter() {
            group.bench_with_input(
                BenchmarkId::new(format!("samples_{}_vol_{}", size, volume), size), 
                size, 
                |b, _| {
                    b.iter(|| {
                        convert_voice_data(black_box(data.clone()), black_box(*volume))
                    })
                }
            );
        }
    }
    
    group.finish();
}

fn bench_volume_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("volume_scaling");
    let data = generate_test_data(1024);
    let f32_data: Vec<f32> = data.iter().map(|&x| x as f32 / 32768.0).collect();
    
    for volume in [0.1, 0.5, 1.0, 1.5, 2.0].iter() {
        group.bench_with_input(
            BenchmarkId::new("direct_scaling", volume), 
            volume, 
            |b, &vol| {
                b.iter(|| {
                    let _result: Vec<f32> = f32_data
                        .iter()
                        .map(|&sample| black_box(sample * vol))
                        .collect();
                })
            }
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_i16tof32, bench_convert_voice_data, bench_volume_scaling);
criterion_main!(benches);