//! Los escenarios: lo que hay detras de cada baile.
//!
//! Hasta ahora los cuatro bailes se bailaban delante de la misma pared, y eso
//! los hacia parecer el mismo nivel con otro jefe. En Cuphead cada jefe tiene
//! su decorado, y es media razon de que cada combate se recuerde.
//!
//! Cada uno es el sitio de su baile y de su epoca: **Viena** para el vals, un
//! **arrabal** de Buenos Aires para el tango, un **club** de jazz para el
//! charleston y **el Moulin Rouge** para el cancan. Y no tienen la misma luz:
//! el vals y el tango son de noche, el club es dorado y el Moulin Rouge es
//! rojo. La variedad sale de ahi mas que del dibujo.
//!
//! Todo es fondo: se dibuja en la franja de encima del horizonte, detras del
//! jefe, y no hay nada aqui que sea informacion. Las balas pasan por encima.
//! Y laten con el compas: `brillo` es el mismo pulso que ya mueve los focos.

use macroquad::prelude::*;
use vals_core::ARENA_W;
use vals_core::math::{PI, sin_cos};
use vals_core::rng::Pcg32;

use crate::draw::Layout;
use crate::paleta::TINTA;

/// El decorado de un baile, en la franja de `alto` unidades de arriba.
///
/// `t` son ticks continuos y `brillo` el pulso del compas, de 0 a 1.
pub fn dibujar(l: &Layout, baile: usize, alto: f32, t: f32, brillo: f32) {
    match baile {
        0 => viena(l, alto, brillo),
        1 => arrabal(l, alto, brillo),
        2 => club(l, alto, t, brillo),
        _ => moulin_rouge(l, alto, t, brillo),
    }
}

// ---------------------------------------------------------------------------
// El Vals: un salon de Viena, de noche.
// ---------------------------------------------------------------------------

fn viena(l: &Layout, alto: f32, brillo: f32) {
    rect(l, 0.0, 0.0, ARENA_W, alto, color_u8!(24, 28, 54, 255));

    let noche = color_u8!(36, 54, 104, 255);
    let noche_baja = color_u8!(56, 78, 136, 255);
    let (ancho, arriba) = (104.0, alto * 0.30);
    for (i, cx) in [118.0, 320.0, 522.0].into_iter().enumerate() {
        let x0 = cx - ancho * 0.5;
        // El cristal, con el arco de arriba y la noche aclarando hacia abajo.
        circulo(l, cx, arriba, ancho * 0.5, noche);
        rect(l, x0, arriba, ancho, alto - arriba, noche);
        rect(l, x0, alto * 0.72, ancho, alto * 0.28, noche_baja);
        // Estrellas, y la luna en el del medio.
        let mut rng = Pcg32::new(0x51A5 + i as u64);
        for _ in 0..6 {
            let x = x0 + 8.0 + rng.next_f32() * (ancho - 16.0);
            let y = arriba - 20.0 + rng.next_f32() * alto * 0.35;
            circulo(
                l,
                x,
                y,
                1.3,
                fade(color_u8!(240, 232, 200, 255), 0.55 + 0.45 * brillo),
            );
        }
        if i == 1 {
            circulo(
                l,
                cx + 20.0,
                arriba + 6.0,
                16.0,
                color_u8!(238, 228, 196, 255),
            );
        }
        // La carpinteria, en tinta.
        linea(l, (cx, arriba - ancho * 0.5), (cx, alto), 2.0, TINTA);
        linea(l, (x0, alto * 0.6), (x0 + ancho, alto * 0.6), 2.0, TINTA);
        arco(l, cx, arriba, ancho * 0.5, 3.5, TINTA);
        linea(l, (x0, arriba), (x0, alto), 3.5, TINTA);
        linea(l, (x0 + ancho, arriba), (x0 + ancho, alto), 3.5, TINTA);
    }

    // Las pilastras entre ventanales, con su capitel.
    let piedra = color_u8!(44, 48, 82, 255);
    for x in [16.0, 219.0, 421.0, 624.0] {
        rect(l, x - 12.0, 0.0, 24.0, alto, piedra);
        rect(l, x - 17.0, alto * 0.16, 34.0, 8.0, piedra);
        linea(l, (x - 12.0, 0.0), (x - 12.0, alto), 2.0, TINTA);
        linea(l, (x + 12.0, 0.0), (x + 12.0, alto), 2.0, TINTA);
    }
}

// ---------------------------------------------------------------------------
// El Tango: un arrabal de Buenos Aires, de noche, con luna y farolas.
// ---------------------------------------------------------------------------

fn arrabal(l: &Layout, alto: f32, brillo: f32) {
    // El cielo, en tres franjas: un degradado a mano.
    for (i, c) in [
        color_u8!(26, 16, 34, 255),
        color_u8!(42, 24, 44, 255),
        color_u8!(66, 36, 50, 255),
    ]
    .into_iter()
    .enumerate()
    {
        rect(l, 0.0, alto * i as f32 / 3.0, ARENA_W, alto / 3.0 + 1.0, c);
    }

    // Una luna enorme, que es la mitad de un tango.
    let luna = color_u8!(240, 224, 186, 255);
    circulo(l, 470.0, alto * 0.32, 56.0, fade(luna, 0.08));
    circulo(l, 470.0, alto * 0.32, 36.0, luna);

    // Los tejados: una fila de casas bajas con alguna ventana encendida.
    // Sembradas con una semilla fija, asi que el barrio no cambia entre
    // fotogramas ni entre partidas.
    let casa = color_u8!(18, 11, 16, 255);
    let luz = color_u8!(236, 188, 104, 255);
    let mut rng = Pcg32::new(0x7A_4760);
    let mut x = -10.0;
    while x < ARENA_W {
        let w = 46.0 + rng.next_f32() * 60.0;
        let h = alto * (0.28 + rng.next_f32() * 0.30);
        rect(l, x, alto - h, w, h, casa);
        for fila in 0..2 {
            for col in 0..3 {
                if rng.next_f32() < 0.35 {
                    let vx = x + 8.0 + col as f32 * (w - 16.0) / 3.0;
                    let vy = alto - h + 10.0 + fila as f32 * 18.0;
                    rect(l, vx, vy, 7.0, 9.0, fade(luz, 0.75));
                }
            }
        }
        x += w;
    }

    // Dos farolas, y su luz late con el compas.
    for fx in [118.0, 522.0] {
        let arriba = alto * 0.34;
        tri(
            l,
            (fx - 6.0, arriba + 10.0),
            (fx + 6.0, arriba + 10.0),
            (fx + 70.0, alto),
            fade(luz, 0.05 + 0.07 * brillo),
        );
        tri(
            l,
            (fx - 6.0, arriba + 10.0),
            (fx - 70.0, alto),
            (fx + 70.0, alto),
            fade(luz, 0.05 + 0.07 * brillo),
        );
        rect(l, fx - 2.5, arriba, 5.0, alto - arriba, TINTA);
        rect(l, fx - 9.0, arriba - 16.0, 18.0, 20.0, TINTA);
        rect(
            l,
            fx - 6.0,
            arriba - 13.0,
            12.0,
            14.0,
            fade(luz, 0.75 + 0.25 * brillo),
        );
    }
}

// ---------------------------------------------------------------------------
// El Charleston: un club de jazz, con su sol Art Deco y su marquesina.
// ---------------------------------------------------------------------------

fn club(l: &Layout, alto: f32, t: f32, brillo: f32) {
    rect(l, 0.0, 0.0, ARENA_W, alto, color_u8!(20, 40, 34, 255));

    // El sol Art Deco: rayos alternos desde el centro del escenario. Es EL
    // motivo de los anos veinte, y aqui son dieciseis triangulos.
    let centro = (ARENA_W * 0.5, alto);
    let radio = alto * 1.05;
    let rayos = 16;
    for i in 0..rayos {
        let a0 = PI + PI * i as f32 / rayos as f32;
        let a1 = PI + PI * (i + 1) as f32 / rayos as f32;
        let (s0, c0) = sin_cos(a0);
        let (s1, c1) = sin_cos(a1);
        let c = if i % 2 == 0 {
            color_u8!(186, 142, 58, 255)
        } else {
            color_u8!(132, 98, 40, 255)
        };
        tri(
            l,
            centro,
            (centro.0 + c0 * radio, centro.1 + s0 * radio),
            (centro.0 + c1 * radio, centro.1 + s1 * radio),
            fade(c, 0.85 + 0.15 * brillo),
        );
    }

    // La concha de la orquesta: arcos concentricos en tinta.
    for k in [0.42, 0.58, 0.74] {
        arco(l, centro.0, centro.1, alto * k, 3.0, TINTA);
    }

    // Las bombillas de la marquesina, corriendo. Una de cada tres encendida y
    // el patron avanza solo: es lo que hace que un club parezca abierto.
    let bombillas = 19;
    let paso = (t / 9.0) as usize;
    for i in 0..bombillas {
        let a = PI + PI * (i as f32 + 0.5) / bombillas as f32;
        let (s, c) = sin_cos(a);
        let r = alto * 0.88;
        let encendida = (i + paso).is_multiple_of(3);
        let color = if encendida {
            color_u8!(255, 236, 170, 255)
        } else {
            color_u8!(110, 86, 44, 255)
        };
        circulo(l, centro.0 + c * r, centro.1 + s * r, 5.0, TINTA);
        circulo(l, centro.0 + c * r, centro.1 + s * r, 3.6, color);
    }
}

// ---------------------------------------------------------------------------
// El Cancan: el Moulin Rouge, con su molino y su telon.
// ---------------------------------------------------------------------------

fn moulin_rouge(l: &Layout, alto: f32, t: f32, brillo: f32) {
    for (i, c) in [
        color_u8!(56, 22, 40, 255),
        color_u8!(80, 32, 48, 255),
        color_u8!(106, 44, 54, 255),
    ]
    .into_iter()
    .enumerate()
    {
        rect(l, 0.0, alto * i as f32 / 3.0, ARENA_W, alto / 3.0 + 1.0, c);
    }

    // El molino: la casa, el tejado y las cuatro aspas girando. Que giren es
    // lo que hace que se reconozca sin letrero.
    let rojo = color_u8!(168, 40, 48, 255);
    let (cx, techo) = (ARENA_W * 0.5, alto * 0.50);
    rect(l, cx - 46.0, techo, 92.0, alto - techo, rojo);
    tri(
        l,
        (cx - 54.0, techo),
        (cx + 54.0, techo),
        (cx, alto * 0.30),
        color_u8!(120, 26, 34, 255),
    );
    linea(l, (cx - 46.0, techo), (cx - 46.0, alto), 3.0, TINTA);
    linea(l, (cx + 46.0, techo), (cx + 46.0, alto), 3.0, TINTA);
    rect(l, cx - 14.0, alto - 38.0, 28.0, 38.0, TINTA);

    let eje = (cx, alto * 0.36);
    let giro = t * 0.012;
    let lona = color_u8!(232, 206, 164, 255);
    for k in 0..4 {
        let a = giro + PI * 0.5 * k as f32;
        let (s, c) = sin_cos(a);
        let (ps, pc) = (c, -s);
        let (largo, ancho) = (104.0, 13.0);
        let base = (eje.0 + c * 12.0, eje.1 + s * 12.0);
        let punta = (eje.0 + c * largo, eje.1 + s * largo);
        let esquina = |p: (f32, f32), d: f32| (p.0 + pc * d, p.1 + ps * d);
        let (a0, a1) = (esquina(base, 0.0), esquina(base, ancho));
        let (b0, b1) = (esquina(punta, 0.0), esquina(punta, ancho));
        tri(l, a0, a1, b1, lona);
        tri(l, a0, b1, b0, lona);
        linea(l, a0, b0, 2.5, TINTA);
        linea(l, a1, b1, 2.5, TINTA);
        linea(l, b0, b1, 2.5, TINTA);
        // El enrejado de la vela.
        for f in [0.35, 0.6, 0.85] {
            let p = (
                base.0 + (punta.0 - base.0) * f,
                base.1 + (punta.1 - base.1) * f,
            );
            linea(l, esquina(p, 0.0), esquina(p, ancho), 1.5, TINTA);
        }
    }
    circulo(l, eje.0, eje.1, 9.0, TINTA);
    circulo(l, eje.0, eje.1, 5.0, color_u8!(214, 170, 84, 255));

    // El telon, a los lados, con sus pliegues y su bambalina de ondas arriba.
    let terciopelo = color_u8!(128, 20, 32, 255);
    let pliegue = color_u8!(84, 12, 22, 255);
    for (x0, lado) in [(0.0, 1.0), (ARENA_W - 64.0, -1.0)] {
        rect(l, x0, 0.0, 64.0, alto, terciopelo);
        for k in 1..4 {
            let x = x0 + 16.0 * k as f32;
            linea(l, (x, 0.0), (x + lado * 4.0, alto), 3.0, pliegue);
        }
    }
    let oro = color_u8!(214, 170, 84, 255);
    let mut x = 0.0;
    while x < ARENA_W {
        circulo(l, x + 16.0, 6.0, 17.0, terciopelo);
        x += 32.0;
    }
    linea(
        l,
        (0.0, 22.0),
        (ARENA_W, 22.0),
        2.5,
        fade(oro, 0.7 + 0.3 * brillo),
    );
}

// ---------------------------------------------------------------------------
// Primitivas en coordenadas de arena. Cuatro lineas cada una, y ahorran que
// cada decorado tenga que saber de pixeles.
// ---------------------------------------------------------------------------

fn rect(l: &Layout, x: f32, y: f32, w: f32, h: f32, c: Color) {
    let p = l.to_screen(x, y);
    draw_rectangle(p.x, p.y, l.len(w), l.len(h), c);
}

fn circulo(l: &Layout, x: f32, y: f32, r: f32, c: Color) {
    let p = l.to_screen(x, y);
    draw_circle(p.x, p.y, l.len(r), c);
}

fn tri(l: &Layout, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: Color) {
    let p = |q: (f32, f32)| l.to_screen(q.0, q.1);
    draw_triangle(p(a), p(b), p(c), color);
}

fn linea(l: &Layout, a: (f32, f32), b: (f32, f32), g: f32, c: Color) {
    let (p, q) = (l.to_screen(a.0, a.1), l.to_screen(b.0, b.1));
    draw_line(p.x, p.y, q.x, q.y, l.len(g).max(1.0), c);
}

/// La mitad de arriba de un circulo: arcos, cupulas y conchas.
fn arco(l: &Layout, x: f32, y: f32, r: f32, g: f32, c: Color) {
    let p = l.to_screen(x, y);
    draw_arc(p.x, p.y, 40, l.len(r), 180.0, l.len(g).max(1.0), 180.0, c);
}

fn fade(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}
