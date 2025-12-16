use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use russsty::bench::{
    EdgeFunctionRasterizer, FrameBuffer, Rasterizer, ScanlineRasterizer, Triangle,
};
use russsty::math::vec3::Vec3;

const BUFFER_WIDTH: u32 = 800;
const BUFFER_HEIGHT: u32 = 600;

fn create_buffer() -> Vec<u32> {
    vec![0u32; (BUFFER_WIDTH * BUFFER_HEIGHT) as usize]
}

fn small_triangle() -> Triangle {
    Triangle::new(
        [
            Vec3::new(100.0, 100.0, 0.0),
            Vec3::new(120.0, 100.0, 0.0),
            Vec3::new(110.0, 120.0, 0.0),
        ],
        0xFFFF0000,
        0.0,
    )
}

fn medium_triangle() -> Triangle {
    Triangle::new(
        [
            Vec3::new(100.0, 100.0, 0.0),
            Vec3::new(300.0, 100.0, 0.0),
            Vec3::new(200.0, 300.0, 0.0),
        ],
        0xFFFF0000,
        0.0,
    )
}

fn large_triangle() -> Triangle {
    Triangle::new(
        [
            Vec3::new(50.0, 50.0, 0.0),
            Vec3::new(750.0, 100.0, 0.0),
            Vec3::new(400.0, 550.0, 0.0),
        ],
        0xFFFF0000,
        0.0,
    )
}

fn benchmark_single_triangle(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_triangle");

    let scanline = ScanlineRasterizer::new();
    let edge_fn = EdgeFunctionRasterizer::new();

    for (name, triangle) in [
        ("small", small_triangle()),
        ("medium", medium_triangle()),
        ("large", large_triangle()),
    ] {
        group.bench_with_input(BenchmarkId::new("scanline", name), &triangle, |b, tri| {
            let mut buffer = create_buffer();
            b.iter(|| {
                let mut fb = FrameBuffer::new(&mut buffer, BUFFER_WIDTH, BUFFER_HEIGHT);
                scanline.fill_triangle(black_box(tri), &mut fb, tri.color);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("edge_function", name),
            &triangle,
            |b, tri| {
                let mut buffer = create_buffer();
                b.iter(|| {
                    let mut fb = FrameBuffer::new(&mut buffer, BUFFER_WIDTH, BUFFER_HEIGHT);
                    edge_fn.fill_triangle(black_box(tri), &mut fb, tri.color);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_many_triangles(c: &mut Criterion) {
    let mut group = c.benchmark_group("many_triangles");

    let scanline = ScanlineRasterizer::new();
    let edge_fn = EdgeFunctionRasterizer::new();

    // Generate a grid of small triangles
    let triangles: Vec<Triangle> = (0..20)
        .flat_map(|row| {
            (0..20).map(move |col| {
                let x = col as f32 * 40.0;
                let y = row as f32 * 30.0;
                Triangle::new(
                    [
                        Vec3::new(x, y, 0.0),
                        Vec3::new(x + 35.0, y, 0.0),
                        Vec3::new(x + 17.5, y + 25.0, 0.0),
                    ],
                    0xFFFF0000,
                    0.0,
                )
            })
        })
        .collect();

    group.bench_function("scanline_400_triangles", |b| {
        let mut buffer = create_buffer();
        b.iter(|| {
            let mut fb = FrameBuffer::new(&mut buffer, BUFFER_WIDTH, BUFFER_HEIGHT);
            for tri in &triangles {
                scanline.fill_triangle(black_box(tri), &mut fb, tri.color);
            }
        });
    });

    group.bench_function("edge_function_400_triangles", |b| {
        let mut buffer = create_buffer();
        b.iter(|| {
            let mut fb = FrameBuffer::new(&mut buffer, BUFFER_WIDTH, BUFFER_HEIGHT);
            for tri in &triangles {
                edge_fn.fill_triangle(black_box(tri), &mut fb, tri.color);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_single_triangle, benchmark_many_triangles);
criterion_main!(benches);
