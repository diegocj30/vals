//! Todo el dibujado. Es la unica parte del proyecto que sabe que macroquad
//! existe, y esta escrita para poder tirarse a la basura y rehacerse sobre
//! miniquad puro o wgpu sin tocar `vals-core`.

use macroquad::prelude::*;
use vals_core::bullets::{BULLET_KINDS, Bullets, FLAG_PARRYABLE};
use vals_core::{ARENA_H, ARENA_W, Mode, World, player};

use crate::bullet_renderer::BulletRenderer;
use crate::skeleton::{self, HUESOS, N_CINTA, N_FALDA, Pose, REMATES};
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

// Las parryables. Rosa brillante y con un anillo que late: tienen que gritar
// "ven a por mi" desde el otro lado de la pantalla, porque en eso consiste el
// juego que anaden.
const PARRYABLE: Color = color_u8!(255, 145, 210, 255);
const PARRY_RING: Color = color_u8!(255, 255, 255, 255);
const METER: Color = color_u8!(120, 255, 200, 255);
const METER_FULL: Color = color_u8!(255, 235, 140, 255);
const METER_BG: Color = color_u8!(24, 40, 40, 255);

const VEIL: Color = color_u8!(6, 6, 12, 200);
const TITLE: Color = color_u8!(200, 245, 255, 255);
const DEFEAT: Color = color_u8!(255, 110, 150, 255);

const GROUND: Color = color_u8!(80, 120, 190, 255);
/// La falda va mas fria que el cuerpo: separa la tela de la piel sin necesidad
/// de dibujar ni una linea de detalle.
const FALDA: Color = color_u8!(120, 180, 255, 255);

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
        // Se ajusta por el eje mas apretado, no solo por el alto: en una
        // ventana estrecha —o en el navegador de un movil en vertical— escalar
        // solo por la altura saca la arena por los lados y recorta el campo de
        // juego, que en un danmaku es directamente injugable.
        let scale = ((screen_height() - 24.0) / ARENA_H)
            .min((screen_width() - 24.0) / ARENA_W)
            .max(0.05);
        let size = vec2(ARENA_W * scale, ARENA_H * scale);
        let origin = ((vec2(screen_width(), screen_height()) - size) * 0.5).round();
        Self { origin, scale }
    }

    fn to_screen(&self, x: f32, y: f32) -> Vec2 {
        self.origin + vec2(x, y) * self.scale
    }

    /// Esquina de la arena en pixeles. La necesita el shader instanciado.
    pub fn origin_px(&self) -> Vec2 {
        self.origin
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Region logica que cabe en la ventana, para descartar lo que no se ve.
    pub fn visible_bounds(&self) -> (Vec2, Vec2) {
        let min = -self.origin / self.scale;
        let max = (vec2(screen_width(), screen_height()) - self.origin) / self.scale;
        (min, max)
    }

    fn len(&self, logical: f32) -> f32 {
        logical * self.scale
    }
}

fn fade(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

/// Dibuja un frame de juego.
///
/// `bullets_gpu` decide el camino de render: con `Some` va el instanciado de un
/// solo draw call, con `None` el viejo a base de primitivas de macroquad. Los
/// dos conviven a proposito, porque es lo que hace reproducible la comparacion
/// de `docs/PERF.md` dentro de un ano.
pub fn frame(world: &World, alpha: f32, layout: &Layout, bullets_gpu: Option<&mut BulletRenderer>) {
    clear_background(BG);
    draw_arena(layout);
    if world.mode == Mode::Platform {
        draw_ground(layout);
    }
    draw_boss(world, alpha, layout);
    draw_player_shots(world, layout);
    draw_trail(world, layout);
    // Las balas van encima del jefe pero debajo del jugador: taparte tu propia
    // hitbox seria exactamente lo contrario de lo que hace falta.
    let phase = world.tick as f32 + alpha;
    match bullets_gpu {
        Some(r) => r.draw(&world.bullets, layout, pulse(phase), bullet_style),
        None => draw_bullets_at(&world.bullets, layout, phase),
    }
    draw_player(world, alpha, layout);
    draw_hp_bar(world, layout);
    draw_meter(world, layout);
    draw_hud(world, layout);
    draw_super_flash(world, layout);
}

/// El suelo del modo plataformas.
fn draw_ground(l: &Layout) {
    let o = l.to_screen(0.0, player::GROUND_Y + player::PLAYER_SPRITE_RADIUS);
    let w = l.len(ARENA_W);
    let alto = l.to_screen(0.0, ARENA_H).y - o.y;
    draw_rectangle(o.x, o.y, w, alto, ARENA_BG);
    draw_line(o.x, o.y, o.x + w, o.y, 2.0, GROUND);
}

/// Fogonazo del super. Es la unica cosa que ocupa la pantalla entera, y por eso
/// se lee como "ha pasado algo gordo" sin necesidad de explicarlo.
fn draw_super_flash(world: &World, l: &Layout) {
    if world.player.super_ticks == 0 {
        return;
    }
    let k = world.player.super_ticks as f32 / player::SUPER_TICKS as f32;
    let o = l.to_screen(0.0, 0.0);
    draw_rectangle(
        o.x,
        o.y,
        l.len(ARENA_W),
        l.len(ARENA_H),
        fade(METER, k * k * 0.5),
    );
}

/// Nombre del jefe, cual de cuantos es, y las vidas que quedan.
fn draw_hud(world: &World, l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let w = l.len(ARENA_W);

    if !world.boss.defeated {
        let etiqueta = format!(
            "{}  ({}/{})",
            world.boss.name,
            world.boss_index + 1,
            world.boss_count()
        );
        let m = measure_text(&etiqueta, None, 18, 1.0);
        draw_text(
            &etiqueta,
            o.x + w - m.width - 8.0,
            o.y + 34.0,
            18.0,
            TEXT_DIM,
        );
    }

    // Vidas, abajo a la derecha junto al medidor. Un punto por vida: contar
    // tres puntos es mas rapido que leer un numero.
    let y = l.to_screen(0.0, ARENA_H).y - 22.0;
    for i in 0..world.lives {
        draw_circle(o.x + w - 10.0 - i as f32 * 13.0, y, 4.0, HITBOX);
    }
}

/// Velo mas titulo mas subtitulo. Lo comparten menu, victoria y derrota.
fn draw_cartel(l: &Layout, titulo: &str, color: Color, lineas: &[&str]) {
    let o = l.to_screen(0.0, 0.0);
    draw_rectangle(o.x, o.y, l.len(ARENA_W), l.len(ARENA_H), VEIL);

    let cx = l.to_screen(ARENA_W * 0.5, 0.0).x;
    let mut y = l.to_screen(0.0, ARENA_H * 0.34).y;

    let m = measure_text(titulo, None, 56, 1.0);
    draw_text(titulo, cx - m.width * 0.5, y, 56.0, color);
    y += 46.0;

    for linea in lineas {
        let m = measure_text(linea, None, 20, 1.0);
        draw_text(linea, cx - m.width * 0.5, y, 20.0, TEXT);
        y += 26.0;
    }
}

/// Menu. De fondo corre el replay dorado, que es el modo atractor.
pub fn menu(l: &Layout, intentos: u32) {
    let cola = if intentos == 0 {
        String::new()
    } else {
        format!("   ({intentos} intentos)")
    };
    draw_cartel(
        l,
        "VALS",
        TITLE,
        &[
            &format!("Z   volar{cola}"),
            "X   plataformas (con salto)",
            "",
            "flechas mover    Z disparar    X dash",
            "C parry    SHIFT focus    M mudo",
            "",
            "Parriar las balas ROSAS llena la barra SUPER.",
            "Llena, ESPACIO limpia la pantalla y hace mucho dano.",
        ],
    );
}

pub fn fin_de_partida(l: &Layout, world: &World) {
    if world.victory {
        draw_cartel(
            l,
            "FIN DEL VALS",
            VICTORY,
            &["los tres han caido", "", "R otra partida    ESC al menu"],
        );
    } else {
        let quien = format!(
            "caiste ante {}  ({}/{})",
            world.boss.name,
            world.boss_index + 1,
            world.boss_count()
        );
        draw_cartel(
            l,
            "SE ACABO",
            DEFEAT,
            &[
                &quien,
                "",
                // Volver al primer jefe cada vez convertia practicar en un
                // peaje. Se reintenta ESTE.
                "R reintentar este jefe",
                "ESC al menu",
            ],
        );
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

/// Color de una bala y si lleva anillo de parryable.
///
/// La usan los dos caminos de render, el viejo y el instanciado, para que no
/// puedan divergir: la paleta vive aqui y el renderer de GPU no sabe de
/// estetica.
pub fn bullet_style(kind: u8, flags: u8) -> (Color, bool) {
    let parryable = flags & FLAG_PARRYABLE != 0;
    let c = if parryable {
        PARRYABLE
    } else {
        BULLET_COLORS[kind as usize]
    };
    (c, parryable)
}

/// Latido del anillo de las parryables, en `[0, 1]`.
pub fn pulse(phase: f32) -> f32 {
    0.5 + 0.5 * (phase * 0.18).sin()
}

/// Dibuja las balas.
///
/// Dos circulos por bala (halo y nucleo) con las primitivas normales de
/// macroquad. Es a proposito la version lenta: H6 la sustituye por un unico
/// draw call instanciado, y esto es el "antes" con el que se comparara.
pub fn draw_bullets(bullets: &Bullets, l: &Layout) {
    draw_bullets_at(bullets, l, 0.0)
}

/// `phase` anima el latido de las parryables. La escena de stress pasa 0.
pub fn draw_bullets_at(bullets: &Bullets, l: &Layout, phase: f32) {
    let latido = pulse(phase);
    for b in bullets.iter_live() {
        let s = l.to_screen(b.pos.x, b.pos.y);
        let (color, parryable) = bullet_style(b.kind, b.flags);
        let r = l.len(BULLET_KINDS[b.kind as usize].draw_radius);

        draw_circle(s.x, s.y, r * 1.9, fade(color, 0.18));
        draw_circle(s.x, s.y, r, color);

        if parryable {
            // El anillo es lo que de verdad las distingue del resto a simple
            // vista, mas que el color: se mueve.
            draw_circle_lines(
                s.x,
                s.y,
                r * (1.45 + 0.35 * latido),
                1.5,
                fade(PARRY_RING, 0.35 + 0.45 * latido),
            );
        }
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

    // El halo se queda: es lo que la separa del fondo cuando la pantalla se
    // llena. Lo que cambia es lo que hay dentro.
    draw_circle(s.x, s.y, sprite_r * 2.2, fade(PLAYER_GLOW, body_alpha));
    // La elegancia sube con cada jefe caido: la bailarina baila mejor segun
    // avanza. Es solo cosmetico, pero de un vistazo dice por donde vas.
    let elegancia = if world.boss_count() > 1 {
        world.boss_index as f32 / (world.boss_count() - 1) as f32
    } else {
        0.0
    };
    let figura = skeleton::pose(&world.player, t, world.mode == Mode::Platform, elegancia);
    draw_figura(&figura, s, l, body_alpha);

    // Anillo de focus: se cierra sobre la hitbox conforme entras en modo lento.
    let ft = world.player.focus_t;
    if ft > 0.0 {
        let r = sprite_r * (2.0 - 0.8 * ft);
        draw_circle_lines(s.x, s.y, r, 1.5, fade(HITBOX, ft * 0.9));
    }

    // Radio de roce, visible solo en focus: es informacion util al aprender,
    // pero pintarla siempre ensuciaria la pantalla justo cuando mas limpia
    // tiene que estar.
    if ft > 0.0 {
        draw_circle_lines(
            s.x,
            s.y,
            l.len(player::GRAZE_RADIUS),
            1.0,
            fade(METER, ft * 0.25),
        );
    }

    // La ventana de parry abierta. Se dibuja el alcance real, ni mas ni menos:
    // que el jugador pueda aprender la distancia mirando.
    if world.player.is_parrying() {
        let r = l.len(player::PARRY_RADIUS);
        draw_circle(s.x, s.y, r, fade(PARRYABLE, 0.10));
        draw_circle_lines(s.x, s.y, r, 2.0, fade(PARRY_RING, 0.75));
    }

    // Fogonazo del super.
    if world.player.super_ticks > 0 {
        let k = world.player.super_ticks as f32 / player::SUPER_TICKS as f32;
        draw_circle(
            s.x,
            s.y,
            sprite_r * (3.0 + 28.0 * (1.0 - k)),
            fade(METER, k * 0.35),
        );
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

/// Dibuja la figura: falda, huesos con grosor variable, remates y cabeza.
///
/// Cada trazo se pinta dos veces, una gruesa y tenue y otra fina y viva. Es un
/// truco barato que da el contorno de neon sin post-proceso ni una segunda
/// pasada de render.
fn draw_figura(p: &Pose, centro: Vec2, l: &Layout, alfa: f32) {
    let punto = |j: Vec2| centro + vec2(j.x, j.y) * l.scale();

    // --- Cintas ---
    // Van detras de todo: son lo mas lejano y lo mas tenue.
    for cinta in p.cintas {
        for k in 0..N_CINTA - 1 {
            let (a, b) = (punto(cinta[k]), punto(cinta[k + 1]));
            let t = 1.0 - k as f32 / (N_CINTA - 1) as f32;
            let g = l.len(0.8 * t).max(0.8);
            draw_line(a.x, a.y, b.x, b.y, g, fade(FALDA, alfa * 0.5 * t));
        }
    }

    // --- Falda, en dos capas ---
    // La de fuera larga y translucida, la de dentro corta y mas solida. Son dos
    // pasadas del mismo abanico: el volumen sale de la diferencia entre ambas,
    // no de mas geometria.
    let cadera = punto(p.joints[skeleton::CADERA]);
    for (escala, relleno, borde) in [(1.0, 0.18, 0.70), (0.62, 0.30, 0.95)] {
        for i in 0..N_FALDA - 1 {
            let a = cadera + (punto(p.falda[i]) - cadera) * escala;
            let b = cadera + (punto(p.falda[i + 1]) - cadera) * escala;
            draw_triangle(cadera, a, b, fade(FALDA, alfa * relleno));
            let g = l.len(0.85).max(1.0);
            draw_line(a.x, a.y, b.x, b.y, g, fade(FALDA, alfa * borde));
        }
    }

    // --- Corpino ---
    // El torso deja de ser una linea y pasa a tener silueta.
    let c: Vec<Vec2> = p.corpino.iter().map(|q| punto(*q)).collect();
    draw_triangle(c[0], c[1], c[2], fade(PLAYER_BODY, alfa * 0.85));
    draw_triangle(c[0], c[2], c[3], fade(PLAYER_BODY, alfa * 0.85));

    // --- Cuerpo ---
    for (grosor, halo) in [(1.9, 0.20), (1.0, 1.0)] {
        let c = fade(PLAYER_BODY, alfa * halo);
        for (a, b, r0, r1) in HUESOS {
            hueso(
                punto(p.joints[a]),
                punto(p.joints[b]),
                r0 * grosor,
                r1 * grosor,
                l,
                c,
            );
        }
        for (j, r) in REMATES {
            let q = punto(p.joints[j]);
            draw_circle(q.x, q.y, l.len(r * grosor), c);
        }
        let cabeza = punto(p.joints[skeleton::CABEZA]);
        draw_circle(
            cabeza.x,
            cabeza.y,
            l.len(skeleton::RADIO_CABEZA * grosor.min(1.25)),
            c,
        );
        let mono = punto(p.mono);
        draw_circle(
            mono.x,
            mono.y,
            l.len(skeleton::RADIO_MONO * grosor.min(1.3)),
            c,
        );
    }
}

/// Un hueso que se estrecha hacia la punta.
///
/// Se traza con circulos solapados en vez de con un poligono: a este tamano se
/// ve igual, las uniones salen redondeadas gratis y son cuatro lineas.
fn hueso(a: Vec2, b: Vec2, r0: f32, r1: f32, l: &Layout, color: Color) {
    const PASOS: usize = 7;
    for i in 0..=PASOS {
        let t = i as f32 / PASOS as f32;
        let p = a.lerp(b, t);
        let r = l.len(r0 + (r1 - r0) * t).max(0.6);
        draw_circle(p.x, p.y, r, color);
    }
}

/// Medidor del super, abajo. Simetrico con la vida del jefe, que va arriba:
/// lo suyo arriba, lo tuyo abajo.
fn draw_meter(world: &World, l: &Layout) {
    let o = l.to_screen(0.0, ARENA_H);
    let w = l.len(ARENA_W);
    let alto = 6.0;
    let y = o.y - alto - 8.0;
    let lleno = world.player.meter_full();

    draw_rectangle(o.x, y, w, alto, METER_BG);
    let color = if lleno { METER_FULL } else { METER };
    draw_rectangle(o.x, y, w * world.player.meter_ratio(), alto, color);

    // La barra no lleva etiqueta desde H4, y en las primeras partidas de
    // verdad el jugador no supo nunca para que servia. Una palabra lo arregla.
    draw_text(
        "SUPER",
        o.x,
        y - 6.0,
        16.0,
        if lleno { METER_FULL } else { TEXT_DIM },
    );

    if lleno {
        draw_rectangle_lines(o.x, y, w, alto, 2.0, METER_FULL);
        // Latido y aviso explicito de la tecla: si no lo dice, no existe.
        let t = (get_time() as f32 * 6.0).sin() * 0.5 + 0.5;
        let aviso = "ESPACIO";
        let m = measure_text(aviso, None, 20, 1.0);
        draw_text(
            aviso,
            o.x + w - m.width,
            y - 6.0,
            20.0,
            fade(METER_FULL, 0.55 + 0.45 * t),
        );
    }
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
pub fn bench_overlay(
    stats: &FrameStats,
    bullets: &Bullets,
    target: usize,
    steps: u32,
    gpu: Option<&BulletRenderer>,
) {
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
    y += 6.0;
    match gpu {
        Some(r) => {
            put("render  INSTANCIADO", VICTORY, &mut y);
            put(&format!("instancias {:>6}", r.drawn), TEXT_DIM, &mut y);
            put(&format!("culling    {:>6}", r.culled), TEXT_DIM, &mut y);
            put("draw calls      1", METER_FULL, &mut y);
        }
        None => {
            put("render  MACROQUAD", HITBOX, &mut y);
            put(
                &format!("draw calls {:>6}", bullets.live_count() * 2),
                HITBOX,
                &mut y,
            );
        }
    }
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
        &format!("jefe    {}/{}", world.boss_index + 1, world.boss_count()),
        TEXT_DIM,
        &mut y,
    );
    put(&format!("vidas   {}", world.lives), TEXT_DIM, &mut y);
    put(
        &format!("modo    {}", world.mode.nombre()),
        TEXT_DIM,
        &mut y,
    );
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

    y += 6.0;
    put(
        &format!("medidor {:>3.0}%", world.player.meter_ratio() * 100.0),
        if world.player.meter_full() {
            METER_FULL
        } else {
            TEXT_DIM
        },
        &mut y,
    );
    put(
        &format!(
            "parry {}  graze {}",
            world.player.parries, world.player.grazes
        ),
        TEXT_DIM,
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

    let help = "Z disparar  X dash  C parry  ESPACIO super  SHIFT focus  M mudo  F1  R";
    draw_text(help, X, screen_height() - 14.0, 16.0, TEXT_DIM);
}

/// Aviso de una linea. Verde si fue bien, rojo si no.
pub fn banner(msg: &str, error: bool) {
    let color = if error { HITBOX } else { VICTORY };
    draw_text(msg, X, screen_height() - 38.0, 18.0, color);
}

/// Chivato de grabacion. Se graba siempre, asi que conviene que se vea.
pub fn recording_badge(ticks: u64) {
    let texto = format!("REC {:>5}t   F2 guardar", ticks);
    let m = measure_text(&texto, None, 16, 1.0);
    draw_circle(screen_width() - m.width - 26.0, 20.0, 4.0, HITBOX);
    draw_text(
        &texto,
        screen_width() - m.width - 16.0,
        25.0,
        16.0,
        TEXT_DIM,
    );
}

/// Estado de la reproduccion de un replay.
pub fn replay_badge(tick: usize, total: usize, divergencia: Option<u64>) {
    let acabado = tick >= total;
    let texto = if acabado {
        format!("REPLAY  fin ({total}t)   R repetir")
    } else {
        format!("REPLAY  {tick}/{total}")
    };
    let m = measure_text(&texto, None, 18, 1.0);
    draw_text(&texto, screen_width() - m.width - 16.0, 25.0, 18.0, VICTORY);

    if let Some(t) = divergencia {
        let aviso = format!("DIVERGENCIA en el tick {t}");
        let m = measure_text(&aviso, None, 20, 1.0);
        draw_text(&aviso, screen_width() - m.width - 16.0, 48.0, 20.0, HITBOX);
    }
}
