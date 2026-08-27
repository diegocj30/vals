//! Benchmarks del bucle caliente.
//!
//! Miden `vals-core` en headless: sin ventana, sin GPU y sin driver de por
//! medio. Es la cifra honesta del coste de simulacion, y la que va a
//! `docs/PERF.md`.
//!
//! El objetivo no es sacar un numero bonito hoy, sino tener con que comparar
//! dentro de seis meses cuando se toque SIMD, hilos o GPU compute.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vals_core::DT;
use vals_core::bench::Stress;

/// Poblaciones de balas que se miden siempre. No se tocan: cambiarlas rompe la
/// comparabilidad con las filas ya escritas en PERF.md.
const POBLACIONES: [usize; 4] = [1_000, 5_000, 10_000, 20_000];

/// Mover todas las balas un tick.
fn mover_balas(c: &mut Criterion) {
    let mut g = c.benchmark_group("bullets_update");
    for n in POBLACIONES {
        let mut s = Stress::new(n);
        // Calentar hasta alcanzar la poblacion objetivo, para no medir el
        // relleno inicial junto con el movimiento.
        while s.live_count() < n {
            s.step(DT);
        }
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                s.step(black_box(DT));
            })
        });
    }
    g.finish();
}

/// Colision jugador contra todas las balas, en el peor caso.
///
/// Se prueba contra un punto donde no hay nada: asi el barrido no puede salir
/// antes de tiempo y se mide el coste completo de recorrer el pool.
fn colision_peor_caso(c: &mut Criterion) {
    let mut g = c.benchmark_group("hit_circle_worst_case");
    for n in POBLACIONES {
        let mut s = Stress::new(n);
        while s.live_count() < n {
            s.step(DT);
        }
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| black_box(s.worst_case_hit_test()))
        });
    }
    g.finish();
}

criterion_group!(benches, mover_balas, colision_peor_caso);
criterion_main!(benches);
