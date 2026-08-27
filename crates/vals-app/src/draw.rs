//! Todo el dibujado. Es la unica parte del proyecto que sabe que macroquad
//! existe, y esta escrita para poder tirarse a la basura y rehacerse sobre
//! miniquad puro o wgpu sin tocar `vals-core`.

use macroquad::prelude::*;
use vals_core::bullets::{BULLET_KINDS, Bullets};
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
// Un color por tipo de bala. Que cada tipo se lea de un vistazo importa mas
// que que sea bonito: con la pantalla llena, el color es la unica pista de a
// que velocidad viene algo.
const BULLET_COLORS: [Color; 4] = [
    color_u8!(120, 230, 255, 255), // pequena: cian
    color_u8!(255, 190, 90, 255),  // media: ambar
    color_u8!(200, 130, 255, 255), // grande: violeta
    color_u8!(255, 245, 210, 255), // aguja: blanco caliente
];

// El jefe. Formas geometricas girando: arte procedural, cero dibujo.
const BOSS_RING: Color = color_u8!(120, 240, 255, 255);
const BOSS_INNER: Color = color_u8!(255, 110, 190, 255);
const BOSS_CORE: Color = color_u8!(30, 20, 60, 255);
const BOSS_FLASH: Color = color_u8!(255, 255, 255, 255);
const HP_BAR: Color = color_u8!(255, 90, 160, 255);
const HP_BAR_BG: Color = color_u8!(40, 26, 48, 255);
const SHOT: Color = color_u8!(180, 255, 240, 255);
const VICTORY: Color = color_u8!(180, 255, 220, 255);

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
    draw_boss(world, alpha, layout);
    draw_player_shots(world, layout);
    draw_trail(world, layout);
    // Las balas van encima del jefe pero debajo del jugador: taparte tu propia
    // hitbox seria exactamente lo contrario de lo que hace falta.
    draw_bullets(&world.bullets, layout);
    draw_player(world, alpha, layout);
    draw_hp_bar(world, layout);
    if world.victory {
        draw_victory(layout);
    }
}

/// El jefe: poligonos concentricos girando a distintas velocidades.
///
/// Todo procedural. No hay ni un pixel dibujado a mano en este juego, y esa
/// decision es la que hace que anadir un jefe nuevo sea escribir un RON.
fn draw_boss(world: &World, alpha: f32, l: &Layout) {
    let b = &world.boss;
    if b.defeated {
        return;
    }
    let p = b.render_pos(alpha);
    let s = l.to_screen(p.x, p.y);
    let r = l.len(b.radius);
    let t = world.tick as f32 + alpha;

    // Cada anillo gira a su ritmo y en su sentido. Es lo que hace que la
    // figura parezca viva estando hecha de tres poligonos.
    draw_poly_lines(s.x, s.y, 6, r * 1.55, t * 0.6, 2.0, fade(BOSS_RING, 0.55));
    draw_poly_lines(s.x, s.y, 3, r * 1.15, -t * 1.1, 2.5, fade(BOSS_INNER, 0.8));
    draw_circle(s.x, s.y, r * 0.72, BOSS_CORE);
    draw_poly_lines(s.x, s.y, 8, r * 0.72, t * 0.25, 1.5, BOSS_RING);

    if b.hit_flash > 0 {
        draw_circle(s.x, s.y, r * 0.8, fade(BOSS_FLASH, 0.5));
    }
}

/// Barra de vida de la fase, con una marca por fase superada.
fn draw_hp_bar(world: &World, l: &Layout) {
    let b = &world.boss;
    if b.defeated {
        return;
    }
    let o = l.to_screen(0.0, 0.0);
    let w = l.len(ARENA_W);
    let alto = 6.0;
    let y = o.y + 8.0;

    draw_rectangle(o.x, y, w, alto, HP_BAR_BG);
    draw_rectangle(o.x, y, w * b.hp_ratio(), alto, HP_BAR);

    // Un punto por fase: lleno el que se esta jugando, hueco el que queda.
    for i in 0..b.phase_count() {
        let cx = o.x + 8.0 + i as f32 * 12.0;
        let cy = y + alto + 10.0;
        if i <= b.phase {
            draw_circle(cx, cy, 3.0, HP_BAR);
        } else {
            draw_circle_lines(cx, cy, 3.0, 1.0, HP_BAR_BG);
        }
    }
}

fn draw_victory(l: &Layout) {
    let o = l.to_screen(ARENA_W * 0.5, ARENA_H * 0.42);
    let texto = "FIN DEL VALS";
    let m = measure_text(texto, None, 48, 1.0);
    draw_text(texto, o.x - m.width * 0.5, o.y, 48.0, VICTORY);
}

/// Los disparos del jugador: trazos finos, para no confundirlos con las balas
/// que matan.
fn draw_player_shots(world: &World, l: &Layout) {
    for b in world.player_shots.iter_live() {
        let s = l.to_screen(b.pos.x, b.pos.y);
        let largo = l.len(14.0);
        draw_line(s.x, s.y - largo, s.x, s.y + largo, 2.0, fade(SHOT, 0.85));
        draw_circle(s.x, s.y, l.len(2.5), SHOT);
    }
}

/// Dibuja las balas.
///
/// Dos circulos por bala (halo y nucleo) con las primitivas normales de
/// macroquad. Es a proposito la version lenta: H6 la sustituye por un unico
/// draw call instanciado, y esto es el "antes" con el que se comparara.
pub fn draw_bullets(bullets: &Bullets, l: &Layout) {
    for b in bullets.iter_live() {
        let color = BULLET_COLORS[b.kind as usize];
        let r = l.len(BULLET_KINDS[b.kind as usize].draw_radius);
        let s = l.to_screen(b.pos.x, b.pos.y);
        draw_circle(s.x, s.y, r * 1.9, fade(color, 0.18));
        draw_circle(s.x, s.y, r, color);
    }
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

const X: f32 = 14.0;

fn put(text: &str, color: Color, y: &mut f32) {
    draw_text(text, X, *y, 18.0, color);
    *y += 18.0;
}

/// Bloque de rendimiento, comun al juego y a la escena de stress.
fn stats_block(stats: &FrameStats, steps: u32, y: &mut f32) {
    put(&format!("{:>6.1} fps", stats.fps()), TEXT, y);
    put(&format!("p50 {:>5.2} ms", stats.p50_ms()), TEXT_DIM, y);
    put(
        &format!("p99 {:>5.2} ms", stats.p99_ms()),
        // Rojo por encima de 20 ms: ahi ya se nota el tiron.
        if stats.p99_ms() > 20.0 {
            HITBOX
        } else {
            TEXT_DIM
        },
        y,
    );
    *y += 6.0;
    put(&format!("sim  {:>5.3} ms", stats.sim_ms()), TEXT_DIM, y);
    put(&format!("draw {:>5.3} ms", stats.render_ms()), TEXT_DIM, y);
    put(
        &format!("steps/frame {steps}"),
        if steps > 1 { HITBOX } else { TEXT_DIM },
        y,
    );
}

/// Overlay de la escena de stress. Sin jugador: solo cuenta y coste.
pub fn bench_overlay(stats: &FrameStats, bullets: &Bullets, target: usize, steps: u32) {
    let mut y = 26.0;
    put("ESCENA DE STRESS", HITBOX, &mut y);
    y += 6.0;
    stats_block(stats, steps, &mut y);
    y += 6.0;
    put(
        &format!("balas    {:>6}", bullets.live_count()),
        TEXT,
        &mut y,
    );
    put(&format!("objetivo {:>6}", target), TEXT_DIM, &mut y);
    put(
        &format!("slots    {:>6}", bullets.scanned_slots()),
        TEXT_DIM,
        &mut y,
    );
    draw_text(
        "estos numeros van a docs/PERF.md",
        X,
        screen_height() - 14.0,
        16.0,
        TEXT_DIM,
    );
}

pub fn debug_overlay(world: &World, stats: &FrameStats, steps: u32) {
    let mut y = 26.0;
    stats_block(stats, steps, &mut y);

    y += 6.0;
    put(&format!("tick    {}", world.tick), TEXT_DIM, &mut y);
    put(
        &format!("balas   {}", world.bullets.live_count()),
        TEXT_DIM,
        &mut y,
    );
    put(
        &format!("muertes {}", world.player.deaths),
        if world.player.deaths > 0 {
            HITBOX
        } else {
            TEXT_DIM
        },
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
        &format!(
            "dash {:>2}  iframes {:>3}",
            d.ticks_left, world.player.iframes
        ),
        if world.player.is_invulnerable() {
            HITBOX
        } else {
            TEXT_DIM
        },
        &mut y,
    );
    put(
        &format!("cd   {:>2}  buffer  {:>3}", d.cooldown, d.buffer),
        TEXT_DIM,
        &mut y,
    );

    let help = "F1 debug   R reset   SHIFT focus   X dash";
    draw_text(help, X, screen_height() - 14.0, 16.0, TEXT_DIM);
}

/// Aviso de recarga del patron. Verde si entro, rojo si el RON esta roto.
pub fn hot_reload_banner(msg: &str, error: bool) {
    let color = if error { HITBOX } else { VICTORY };
    let y = screen_height() - 38.0;
    draw_text(msg, X, y, 18.0, color);
}
