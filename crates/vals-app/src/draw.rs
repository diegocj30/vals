//! Todo el dibujado. Es la unica parte del proyecto que sabe que macroquad
//! existe, y esta escrita para poder tirarse a la basura y rehacerse sobre
//! miniquad puro o wgpu sin tocar `vals-core`.

use macroquad::prelude::*;
use vals_core::{ARENA_H, ARENA_W, World, player};

use crate::stats::FrameStats;

// Paleta neon. El arte del juego es procedural: no se dibuja nada a mano.
const BG: Color = color_u8!(8, 8, 14, 255);
const ARENA_BG: Color = color_u8!(14, 14, 26, 255);
const GRID: Color = color_u8!(30, 32, 58, 255);
const BORDER: Color = color_u8!(90, 220, 255, 255);
const PLAYER_GLOW: Color = color_u8!(120, 210, 255, 40);
const PLAYER_BODY: Color = color_u8!(190, 240, 255, 255);
const TRAIL: Color = color_u8!(110, 190, 255, 255);
const TRAIL_DASH: Color = color_u8!(255, 120, 200, 255);
const HITBOX: Color = color_u8!(255, 70, 140, 255);
const TEXT: Color = color_u8!(150, 210, 235, 255);
const TEXT_DIM: Color = color_u8!(90, 120, 145, 255);

/// Separacion de la rejilla de fondo, en unidades logicas.
const GRID_STEP: f32 = 80.0;

/// Correspondencia entre las unidades logicas del mundo y los pixeles de la
/// ventana. El mundo nunca sabe cuantos pixeles mide nada.
pub struct Layout {
    origin: Vec2,
    scale: f32,
}

impl Layout {
    pub fn compute() -> Self {
        let scale = ((screen_height() - 24.0) / ARENA_H).max(0.05);
        let size = vec2(ARENA_W * scale, ARENA_H * scale);
        let origin = ((vec2(screen_width(), screen_height()) - size) * 0.5).round();
        Self { origin, scale }
    }

    fn to_screen(&self, x: f32, y: f32) -> Vec2 {
        self.origin + vec2(x, y) * self.scale
    }

    fn len(&self, logical: f32) -> f32 {
        logical * self.scale
    }
}

fn fade(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

pub fn frame(world: &World, alpha: f32, layout: &Layout) {
    clear_background(BG);
    draw_arena(layout);
    draw_trail(world, layout);
    draw_player(world, alpha, layout);
}

fn draw_arena(l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let w = l.len(ARENA_W);
    let h = l.len(ARENA_H);

    draw_rectangle(o.x, o.y, w, h, ARENA_BG);

    let mut x = GRID_STEP;
    while x < ARENA_W {
        let p = l.to_screen(x, 0.0);
        draw_line(p.x, o.y, p.x, o.y + h, 1.0, GRID);
        x += GRID_STEP;
    }
    let mut y = GRID_STEP;
    while y < ARENA_H {
        let p = l.to_screen(0.0, y);
        draw_line(o.x, p.y, o.x + w, p.y, 1.0, GRID);
        y += GRID_STEP;
    }

    draw_rectangle_lines(o.x, o.y, w, h, 2.0, BORDER);
}

/// La estela. Se muestrea a ritmo de tick en el core, asi que se ve igual a 60
/// que a 144 Hz.
fn draw_trail(world: &World, l: &Layout) {
    for (point, age) in world.player.trail.iter_newest_first() {
        // Cuadratico: la cola se apaga rapido y no deja una serpiente sucia.
        let strength = (1.0 - age) * (1.0 - age);
        let s = l.to_screen(point.pos.x, point.pos.y);
        let r = l.len(player::PLAYER_SPRITE_RADIUS * 0.8 * (1.0 - age));
        let color = if point.dashing { TRAIL_DASH } else { TRAIL };
        draw_circle(s.x, s.y, r, fade(color, strength * 0.5));
    }
}

fn draw_player(world: &World, alpha: f32, l: &Layout) {
    let p = world.player.render_pos(alpha);
    let s = l.to_screen(p.x, p.y);
    let sprite_r = l.len(player::PLAYER_SPRITE_RADIUS);

    // Fase continua en ticks: el parpadeo va al ritmo de la simulacion y no al
    // de los fps, asi que se ve igual en cualquier monitor.
    let t = world.tick as f32 + alpha;

    // Durante los i-frames el cuerpo palpita. Es la senal de "ahora mismo las
    // balas no te tocan", y tiene que leerse de un vistazo.
    let body_alpha = if world.player.is_invulnerable() {
        0.35 + 0.4 * ((t * 0.9).sin() * 0.5 + 0.5)
    } else {
        1.0
    };

    draw_circle(s.x, s.y, sprite_r * 2.2, fade(PLAYER_GLOW, body_alpha));
    draw_circle(s.x, s.y, sprite_r, fade(PLAYER_BODY, body_alpha));

    // Anillo de focus: se cierra sobre la hitbox conforme entras en modo lento.
    let ft = world.player.focus_t;
    if ft > 0.0 {
        let r = sprite_r * (2.0 - 0.8 * ft);
        draw_circle_lines(s.x, s.y, r, 1.5, fade(HITBOX, ft * 0.9));
    }

    // La hitbox real. Siempre visible: aprender que casi todo el personaje es
    // decorativo es parte de aprender el juego.
    draw_circle(
        s.x,
        s.y,
        l.len(player::PLAYER_HITBOX_RADIUS) * (1.0 + 0.6 * ft),
        HITBOX,
    );
}

pub fn debug_overlay(world: &World, stats: &FrameStats, steps: u32) {
    let x = 14.0;
    let mut y = 26.0;
    let line = 18.0;

    let put = |text: &str, color: Color, y: &mut f32| {
        draw_text(text, x, *y, 18.0, color);
        *y += line;
    };

    put(&format!("{:>6.1} fps", stats.fps()), TEXT, &mut y);
    put(&format!("p50 {:>5.2} ms", stats.p50_ms()), TEXT_DIM, &mut y);
    put(
        &format!("p99 {:>5.2} ms", stats.p99_ms()),
        // Rojo por encima de 20 ms: ahi ya se nota el tiron.
        if stats.p99_ms() > 20.0 {
            HITBOX
        } else {
            TEXT_DIM
        },
        &mut y,
    );
    y += 6.0;
    put(
        &format!("sim  {:>5.3} ms", stats.sim_ms()),
        TEXT_DIM,
        &mut y,
    );
    put(
        &format!("draw {:>5.3} ms", stats.render_ms()),
        TEXT_DIM,
        &mut y,
    );
    y += 6.0;
    put(&format!("tick {}", world.tick), TEXT_DIM, &mut y);
    put(
        &format!("steps/frame {steps}"),
        if steps > 1 { HITBOX } else { TEXT_DIM },
        &mut y,
    );

    // Frame data del dash: es lo que hay que mirar para afinar el feel.
    y += 6.0;
    let d = &world.player.dash;
    put(
        &format!("vel  {:>5.0}", world.player.vel.length()),
        TEXT_DIM,
        &mut y,
    );
    put(
        &format!("dash {:>2}  iframes {:>2}", d.ticks_left, d.iframes),
        if world.player.is_invulnerable() {
            HITBOX
        } else {
            TEXT_DIM
        },
        &mut y,
    );
    put(
        &format!("cd   {:>2}  buffer  {:>2}", d.cooldown, d.buffer),
        TEXT_DIM,
        &mut y,
    );

    let help = "F1 debug   R reset   SHIFT focus   X dash";
    draw_text(help, x, screen_height() - 14.0, 16.0, TEXT_DIM);
}
