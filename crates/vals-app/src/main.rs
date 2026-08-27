//! Punto de entrada de VALS.
//!
//! Este crate se ocupa de ventana, input, render y reloj. La simulacion vive
//! entera en `vals-core` y no sabe que esto existe.

use macroquad::prelude::*;
use vals_core::{DT, InputFrame, World};

mod draw;
mod stats;

use stats::FrameStats;

/// "VALS" en ASCII. Semilla por defecto.
const SEED: u64 = 0x5641_4C53;

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
    let mut world = World::new(SEED);
    let mut stats = FrameStats::new();
    let mut accumulator = 0.0f32;
    let mut show_debug = true;

    loop {
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
        stats.push_render((get_time() - t1) as f32);

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
