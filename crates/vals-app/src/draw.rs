//! Todo el dibujado. Es la unica parte del proyecto que sabe que macroquad
//! existe, y esta escrita para poder tirarse a la basura y rehacerse sobre
//! miniquad puro o wgpu sin tocar `vals-core`.

use macroquad::prelude::*;
use vals_core::{ARENA_H, ARENA_W, World, world};

use crate::stats::FrameStats;

// Paleta neon. El arte del juego es procedural: no se dibuja nada a mano.
const BG: Color = color_u8!(8, 8, 14, 255);
const ARENA_BG: Color = color_u8!(14, 14, 26, 255);
const GRID: Color = color_u8!(30, 32, 58, 255);
const BORDER: Color = color_u8!(90, 220, 255, 255);
const PLAYER_GLOW: Color = color_u8!(120, 210, 255, 40);
const PLAYER_BODY: Color = color_u8!(190, 240, 255, 255);
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

pub fn frame(world: &World, alpha: f32, layout: &Layout) {
    clear_background(BG);
    draw_arena(layout);
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

fn draw_player(world: &World, alpha: f32, l: &Layout) {
    let p = world.player.render_pos(alpha);
    let s = l.to_screen(p.x, p.y);

    // Halo. Hasta que llegue el shader de glow del hito H6, esto es un par de
    // circulos translucidos y cumple.
    draw_circle(
        s.x,
        s.y,
        l.len(world::PLAYER_SPRITE_RADIUS * 2.2),
        PLAYER_GLOW,
    );
    draw_circle(s.x, s.y, l.len(world::PLAYER_SPRITE_RADIUS), PLAYER_BODY);

    // La hitbox real. Siempre visible (para aprender el juego), y en modo focus
    // se marca con un anillo.
    draw_circle(s.x, s.y, l.len(world::PLAYER_HITBOX_RADIUS), HITBOX);
    if world.player.focused {
        let r = l.len(world::PLAYER_SPRITE_RADIUS * 1.6);
        draw_circle_lines(s.x, s.y, r, 1.5, HITBOX);
    }
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

    let help = "F1 debug   R reset   mantener SHIFT para focus";
    draw_text(help, x, screen_height() - 14.0, 16.0, TEXT_DIM);
}
