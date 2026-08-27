//! Punto de entrada de VALS.
//!
//! Este crate se ocupa de ventana, input, render y reloj. La simulacion vive
//! entera en `vals-core` y no sabe que esto existe.

use macroquad::prelude::*;
use vals_core::bench::Stress;
use vals_core::{DT, InputFrame, World};

mod draw;
mod hot;
mod stats;

use hot::HotReload;
use stats::FrameStats;

/// "VALS" en ASCII. Semilla por defecto.
const SEED: u64 = 0x5641_4C53;

/// Cada cuantos frames se mira si cambio el fichero del patron.
///
/// Tres veces por segundo: imperceptible al guardar, e irrelevante para el
/// frame time comparado con dibujar un millar de balas.
const HOT_RELOAD_EVERY: u32 = 20;

/// Balas de la escena de stress si no se pide otra cosa.
const BENCH_DEFAULT: usize = 10_000;

/// Maximo de ticks de simulacion por frame.
///
/// Evita la espiral de la muerte: si un frame tarda muchisimo (un breakpoint,
/// la ventana minimizada, el navegador en otra pestana), preferimos que el
/// tiempo del juego se ralentice antes que encadenar simulaciones cada vez mas
/// largas hasta no recuperarnos nunca.
const MAX_STEPS_PER_FRAME: u32 = 5;

/// Techo del delta que aceptamos del sistema, en segundos.
const MAX_FRAME_DT: f32 = 0.25;

fn window_conf() -> Conf {
    Conf {
        window_title: "VALS".to_owned(),
        window_width: 1100,
        window_height: 860,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    match Args::parse() {
        Some(args) => run_bench(args).await,
        None => run_game().await,
    }
}

/// Configuracion de la escena de stress.
struct Args {
    target: usize,
    /// Si se indica, se mide ese numero de frames, se imprime el resumen y se
    /// sale. Es lo que hace que las filas de `docs/PERF.md` salgan de un
    /// comando repetible y no de mirar el overlay a ojo.
    frames: Option<u32>,
}

impl Args {
    /// Lee `--bench-scene [n] [--frames n]`.
    ///
    /// En web no hay argumentos, asi que alli siempre se juega.
    fn parse() -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        let i = args.iter().position(|a| a == "--bench-scene")?;
        let valor = |flag: &str| -> Option<usize> {
            let j = args.iter().position(|a| a == flag)?;
            args.get(j + 1)?.parse().ok()
        };
        Some(Self {
            target: args
                .get(i + 1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(BENCH_DEFAULT),
            frames: valor("--frames").map(|n| n as u32),
        })
    }
}

async fn run_game() {
    let mut world = World::new(SEED);
    let mut stats = FrameStats::new();
    let mut accumulator = 0.0f32;
    let mut show_debug = true;
    let mut hot = HotReload::new();
    let mut frames: u32 = 0;

    loop {
        frames += 1;
        if frames.is_multiple_of(HOT_RELOAD_EVERY)
            && let Some(def) = hot.poll()
        {
            world.reload_boss(&def);
        }

        let frame_dt = get_frame_time().min(MAX_FRAME_DT);
        stats.push_frame(frame_dt);

        if is_key_pressed(KeyCode::F1) {
            show_debug = !show_debug;
        }
        if is_key_pressed(KeyCode::R) {
            world = World::new(SEED);
            accumulator = 0.0;
        }

        // --- Simulacion: paso fijo, desacoplada del render ---
        let input = read_input();
        let t0 = get_time();
        accumulator += frame_dt;
        let mut steps = 0;
        while accumulator >= DT {
            world.step(input);
            accumulator -= DT;
            steps += 1;
            if steps >= MAX_STEPS_PER_FRAME {
                accumulator = 0.0;
                break;
            }
        }
        stats.push_sim((get_time() - t0) as f32);

        // Fraccion de tick pendiente. Es lo que permite que el render vaya a
        // 144 Hz con la simulacion a 60 sin que se vea a saltos.
        let alpha = (accumulator / DT).clamp(0.0, 1.0);

        // --- Render ---
        let t1 = get_time();
        let layout = draw::Layout::compute();
        draw::frame(&world, alpha, &layout);
        if show_debug {
            draw::debug_overlay(&world, &stats, steps);
        }
        if let Some((msg, error)) = hot.aviso() {
            draw::hot_reload_banner(msg, error);
        }
        stats.push_render((get_time() - t1) as f32);

        next_frame().await;
    }
}

/// Escena de stress: la misma que miden los benchmarks, pero con render.
///
/// No hay jugador ni input. Sirve para ver a ojo cuanto aguanta el render y
/// para sacar las cifras que van a `docs/PERF.md`.
async fn run_bench(args: Args) {
    let target = args.target;
    let mut stress = Stress::new(target);
    let mut stats = FrameStats::new();
    let mut accumulator = 0.0f32;
    let mut frames: u32 = 0;
    // Frames que no se cuentan: los primeros siempre incluyen la compilacion
    // de shaders y el llenado inicial del pool, y falsearian el p99.
    let calentamiento = 60;

    loop {
        let frame_dt = get_frame_time().min(MAX_FRAME_DT);
        stats.push_frame(frame_dt);

        let t0 = get_time();
        accumulator += frame_dt;
        let mut steps = 0;
        while accumulator >= DT {
            stress.step(DT);
            accumulator -= DT;
            steps += 1;
            if steps >= MAX_STEPS_PER_FRAME {
                accumulator = 0.0;
                break;
            }
        }
        stats.push_sim((get_time() - t0) as f32);

        let t1 = get_time();
        let layout = draw::Layout::compute();
        clear_background(BLACK);
        draw::draw_bullets(&stress.bullets, &layout);
        draw::bench_overlay(&stats, &stress.bullets, stress.target(), steps);
        stats.push_render((get_time() - t1) as f32);

        frames += 1;
        if let Some(limite) = args.frames
            && frames >= limite + calentamiento
        {
            println!("balas={} objetivo={target}", stress.live_count());
            println!(
                "p50={:.3}ms p99={:.3}ms sim={:.3}ms draw={:.3}ms fps={:.1}",
                stats.p50_ms(),
                stats.p99_ms(),
                stats.sim_ms(),
                stats.render_ms(),
                stats.fps()
            );
            std::process::exit(0);
        }
        if frames == calentamiento {
            stats = FrameStats::new();
        }

        next_frame().await;
    }
}

/// Traduce el teclado a un `InputFrame`.
///
/// Es el unico sitio donde se mapean teclas: la simulacion solo ve bits, asi
/// que anadir mando o rebindeo mas adelante no toca nada del juego.
fn read_input() -> InputFrame {
    let mut f = InputFrame::NONE;
    f.set(
        InputFrame::UP,
        is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
    );
    f.set(
        InputFrame::DOWN,
        is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
    );
    f.set(
        InputFrame::LEFT,
        is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
    );
    f.set(
        InputFrame::RIGHT,
        is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
    );
    f.set(
        InputFrame::FOCUS,
        is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
    );
    f.set(InputFrame::SHOOT, is_key_down(KeyCode::Z));
    f.set(InputFrame::DASH, is_key_down(KeyCode::X));
    f.set(InputFrame::PARRY, is_key_down(KeyCode::C));
    f
}
