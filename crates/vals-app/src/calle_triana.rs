//! Triana: el rio que se cruza antes del flamenco.
//!
//! Sevilla al caer la tarde, que es cuando se sale a la calle: el cielo de
//! violeta a naranja con el sol hundiendose detras de la Giralda, la Torre del
//! Oro en la orilla y el puente de hierro de Triana a medio hacer. Las casas
//! son de **cal y albero**: paredes blancas con zocalos ocres y de almagra,
//! rejas negras, macetas de geranios y azulejos en los portales. Es lo que dice
//! Sevilla sin escribirlo, igual que la chapa pintada dice La Boca.
//!
//! Las capas, de lejos a cerca: el cielo quieto; Sevilla al 12 % (la Giralda,
//! la catedral, la Torre del Oro y las palmeras); las casas de Triana al 40 %;
//! el puente de hierro al 60 %, que solo asoma al llegar al rio; y la calle de
//! albero, con los naranjos, al 100 %. Los fosos son el Guadalquivir, y sobre
//! el van las barcas.
//!
//! El verbo es el **quiebro**, y el dibujo lo tiene que contar antes de que
//! pase: lo que quiebra se prepara en el tiempo de antes del acento
//! (`al_acento`) —la bailaora sube el brazo, el palmero abre las manos, el
//! torito baja la testuz— y en el acento suelta de golpe.

use macroquad::prelude::*;
use vals_core::math::{PI, TAU, atan2, sin_cos};
use vals_core::paseo::{
    BATA, Clase, ENCUADRE, Enemigo, Forma, Paseo, Proyectil, SUELO_Y, Tipo, VIDA_PALMADA, VISTA_H,
    VISTA_W, al_acento,
};
use vals_core::rng::Pcg32;

use crate::calle::{self, HUESO, ROSA, con_tinta, draw_rect_lines};
use crate::draw::{Layout, fade};
use crate::escenarios::{arco, circulo, linea, rect, tri};
use crate::fuentes::{self, Cara};
use crate::idioma::nombre;
use crate::paleta::{LUZ, TINTA};

// La tarde de Sevilla: violeta arriba y naranja abajo, donde se pone el sol.
const CIELO: [Color; 6] = [
    color_u8!(40, 26, 60, 255),
    color_u8!(62, 34, 72, 255),
    color_u8!(96, 46, 78, 255),
    color_u8!(140, 64, 76, 255),
    color_u8!(186, 94, 72, 255),
    color_u8!(220, 132, 82, 255),
];
const SOL: Color = color_u8!(250, 200, 124, 255);
/// Sevilla al fondo, en una tinta: mas oscura que el horizonte encendido.
const LEJOS: Color = color_u8!(92, 42, 64, 255);
/// El puente de hierro, mas cerca y mas oscuro.
const PUENTE: Color = color_u8!(56, 32, 54, 255);
const PUENTE_CLARO: Color = color_u8!(98, 60, 78, 255);
/// La cal de las casas del fondo: blanca, pero apagada por la hora, que el
/// fondo se queda detras de lo que se juega.
const CAL_FONDO: Color = color_u8!(170, 140, 142, 255);
const CAL_SOMBRA: Color = color_u8!(132, 102, 112, 255);
const TEJA: Color = color_u8!(110, 52, 46, 255);
/// La cal de lo que esta en la calle, viva: lo que se pisa es informacion.
const CAL: Color = color_u8!(236, 226, 206, 255);
const ALBERO: Color = color_u8!(204, 150, 70, 255);
const ALBERO_OSCURO: Color = color_u8!(150, 100, 48, 255);
const ALMAGRA: Color = color_u8!(150, 58, 42, 255);
const AZULEJO: Color = color_u8!(44, 84, 150, 255);
const AZULEJO_CLARO: Color = color_u8!(130, 166, 206, 255);
const AMARILLO: Color = color_u8!(230, 182, 66, 255);
const VENTANA: Color = color_u8!(240, 190, 108, 255);
const APAGADA: Color = color_u8!(74, 46, 60, 255);
const HIERRO: Color = color_u8!(30, 26, 32, 255);
const VERDE: Color = color_u8!(52, 92, 54, 255);
const VERDE_CLARO: Color = color_u8!(86, 132, 74, 255);
const NARANJA: Color = color_u8!(236, 140, 40, 255);
const GERANIO: Color = color_u8!(206, 44, 56, 255);
/// El Guadalquivir al atardecer: verde oscuro, con el cielo naranja encima.
const RIO: Color = color_u8!(30, 50, 58, 255);
const MADERA: Color = color_u8!(118, 70, 42, 255);
const MADERA_CLARA: Color = color_u8!(188, 136, 82, 255);
const PIEL: Color = color_u8!(232, 190, 156, 255);
const NEGRO: Color = color_u8!(34, 28, 34, 255);
/// El rojo del traje de flamenca y del clavel. Rojo, no rosa: rosa solo es lo
/// que se parria.
const ROJO: Color = color_u8!(196, 34, 44, 255);
const ROJO_OSCURO: Color = color_u8!(120, 22, 34, 255);

/// Un rectangulo con su contorno de tinta.
fn caja(l: &Layout, x: f32, y: f32, w: f32, h: f32, c: Color, g: f32) {
    rect(l, x, y, w, h, c);
    draw_rect_lines(l, x, y, w, h, g);
}

/// Lo que falta para el acento, de 0 (lejos) a 1 (ya): el aviso de todo lo
/// que quiebra.
fn prepara(tick: u64, tiempo: u32, ticks: f32) -> f32 {
    1.0 - (al_acento(tick, tiempo) as f32 / ticks).min(1.0)
}

/// Hacia donde va algo que vuela, o hacia la izquierda si esta quieto.
fn hacia(x: f32, y: f32) -> Vec2 {
    let n = (x * x + y * y).sqrt();
    if n > 1e-4 {
        vec2(x / n, y / n)
    } else {
        vec2(-1.0, 0.0)
    }
}

/// El rio: el foso mas largo, de muelle a muelle.
fn el_rio(p: &Paseo) -> Option<(f32, f32)> {
    p.def
        .suelo
        .windows(2)
        .map(|par| (par[0].1, par[1].0))
        .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
}

// ---------------------------------------------------------------------------
// El fondo
// ---------------------------------------------------------------------------

/// El fondo de Triana, del cielo a los naranjos de la acera.
pub fn fondo(p: &Paseo, l: &Layout, capa: &dyn Fn(f32) -> Layout, cam: f32, pulso: f32, t: f32) {
    cielo(l, pulso);
    sevilla(&capa(0.12), cam * 0.12, pulso);
    casas(&capa(0.4), cam * 0.4, pulso);
    puente(p, &capa(0.6), cam, pulso);
    maestranza(&capa(1.0), cam);
    naranjos(p, &capa(1.0), cam, pulso, t);
}

/// El cielo de la tarde, el sol poniendose y las primeras estrellas. Quietos:
/// estan tan lejos que moverlos seria mentir.
fn cielo(l: &Layout, pulso: f32) {
    let alto = 420.0 / CIELO.len() as f32;
    for (i, c) in CIELO.into_iter().enumerate() {
        rect(l, 0.0, alto * i as f32, VISTA_W, alto + 1.0, c);
    }
    rect(l, 0.0, 420.0, VISTA_W, VISTA_H - 420.0, CIELO[5]);
    circulo(l, 610.0, 320.0, 110.0, fade(SOL, 0.08 + 0.04 * pulso));
    circulo(l, 610.0, 320.0, 64.0, fade(SOL, 0.22));
    circulo(l, 610.0, 320.0, 42.0, SOL);
    // Arriba ya es de noche; abajo aun no.
    let mut rng = Pcg32::new(0x7A1A_4A00);
    for _ in 0..18 {
        let (x, y) = (rng.next_f32() * VISTA_W, rng.next_f32() * 110.0);
        let brillo = 0.3 + 0.5 * rng.next_f32();
        circulo(l, x, y, 1.1, fade(HUESO, brillo * (0.6 + 0.4 * pulso)));
    }
}

/// Sevilla a lo lejos, en una tinta: tejados bajos, la catedral con sus
/// pinaculos, la Giralda, la Torre del Oro y palmeras. Se repite cada
/// `PERIODO`, y solo se pintan las copias que se ven.
fn sevilla(l: &Layout, desde: f32, pulso: f32) {
    const PERIODO: f32 = 1600.0;
    let base = 360.0;
    let primera = (desde / PERIODO).floor() as i32;
    for k in primera..=primera + 1 {
        let x0 = k as f32 * PERIODO;
        let mut rng = Pcg32::new(0x5E_7111 + k as u64);
        let mut x = x0;
        while x < x0 + PERIODO {
            let w = 50.0 + rng.next_f32() * 80.0;
            let h = 20.0 + rng.next_f32() * 40.0;
            rect(l, x, base - h, w + 1.0, h + 120.0, LEJOS);
            if rng.next_f32() < 0.4 {
                let c = fade(VENTANA, 0.35 + 0.2 * pulso);
                rect(l, x + w * 0.5, base - h + 10.0, 4.0, 5.0, c);
            }
            x += w;
        }
        // La catedral: la nave con sus pinaculos.
        let cx = x0 + 420.0;
        rect(l, cx - 180.0, base - 90.0, 300.0, 90.0, LEJOS);
        for i in 0..8 {
            let px = cx - 170.0 + i as f32 * 40.0;
            tri(
                l,
                (px - 6.0, base - 90.0),
                (px, base - 122.0),
                (px + 6.0, base - 90.0),
                LEJOS,
            );
        }
        circulo(l, cx - 40.0, base - 96.0, 22.0, LEJOS);
        // La Giralda: la torre almohade, el campanario y el giraldillo.
        let gx = cx + 160.0;
        rect(l, gx - 20.0, base - 230.0, 40.0, 230.0, LEJOS);
        rect(l, gx - 15.0, base - 270.0, 30.0, 40.0, LEJOS);
        rect(l, gx - 10.0, base - 296.0, 20.0, 26.0, LEJOS);
        circulo(l, gx, base - 298.0, 8.0, LEJOS);
        linea(l, (gx, base - 306.0), (gx, base - 322.0), 2.0, LEJOS);
        circulo(l, gx, base - 324.0, 3.0, LEJOS);
        for dy in [-250.0, -215.0, -160.0, -100.0] {
            let c = fade(VENTANA, 0.3 + 0.25 * pulso);
            rect(l, gx - 3.0, base + dy, 6.0, 10.0, c);
        }
        // La Torre del Oro, a la orilla: doce lados en tres cuerpos.
        let ox = x0 + 1050.0;
        rect(l, ox - 30.0, base - 90.0, 60.0, 90.0, LEJOS);
        rect(l, ox - 18.0, base - 130.0, 36.0, 40.0, LEJOS);
        rect(l, ox - 9.0, base - 150.0, 18.0, 20.0, LEJOS);
        circulo(l, ox, base - 152.0, 9.0, LEJOS);
        for i in 0..5 {
            let mx = ox - 30.0 + i as f32 * 13.0;
            rect(l, mx, base - 98.0, 8.0, 8.0, LEJOS);
        }
        // Palmeras.
        for px in [x0 + 760.0, x0 + 1300.0, x0 + 1420.0] {
            palmera(l, px, base, LEJOS);
        }
    }
}

/// Una palmera: el tronco algo torcido y el penacho de hojas que caen.
fn palmera(l: &Layout, x: f32, base: f32, c: Color) {
    let copa = (x + 10.0, base - 120.0);
    linea(l, (x, base), (x + 4.0, base - 60.0), 5.0, c);
    linea(l, (x + 4.0, base - 60.0), copa, 4.0, c);
    for i in 0..7 {
        let a = PI * (0.1 + 0.8 * i as f32 / 6.0);
        let (s, co) = sin_cos(a);
        let punta = (copa.0 - co * 34.0, copa.1 - s * 10.0 + 18.0);
        let medio = (copa.0 - co * 20.0, copa.1 - s * 16.0);
        linea(l, copa, medio, 3.0, c);
        linea(l, medio, punta, 2.5, c);
    }
}

/// Las casas de Triana: cal apagada, zocalo de almagra, rejas y balcones, el
/// alero de teja y, cada tanto, la espadana de una iglesia. Bajas, para que
/// por encima se vea Sevilla.
fn casas(l: &Layout, desde: f32, pulso: f32) {
    const VANO: f32 = 170.0;
    let base = 470.0;
    let primero = (desde / VANO).floor() as i32 - 1;
    for k in primero..primero + (VISTA_W / VANO) as i32 + 3 {
        let x0 = k as f32 * VANO;
        let mut rng = Pcg32::new(0x7A_1A2A + k as u64);
        let alto = 120.0 + rng.next_f32() * 70.0;
        let techo = base - alto;
        // La espadana, una de cada siete.
        if k.rem_euclid(7) == 3 {
            let cx = x0 + VANO * 0.5;
            rect(l, cx - 22.0, techo - 70.0, 44.0, 70.0, CAL_FONDO);
            arco(l, cx, techo - 70.0, 22.0, 2.5, TINTA);
            circulo(l, cx, techo - 40.0, 9.0, APAGADA);
            linea(l, (cx - 22.0, techo - 70.0), (cx - 22.0, techo), 2.0, TINTA);
            linea(l, (cx + 22.0, techo - 70.0), (cx + 22.0, techo), 2.0, TINTA);
        }
        rect(l, x0, techo, VANO + 1.0, alto, CAL_FONDO);
        rect(l, x0, base - 34.0, VANO + 1.0, 34.0, fade(ALMAGRA, 0.75));
        linea(l, (x0, base - 34.0), (x0 + VANO, base - 34.0), 1.5, TINTA);
        // El alero de teja.
        rect(l, x0 - 4.0, techo - 10.0, VANO + 9.0, 10.0, TEJA);
        linea(l, (x0 - 4.0, techo), (x0 + VANO + 5.0, techo), 2.0, TINTA);
        let mut tx = x0;
        while tx < x0 + VANO {
            arco(l, tx + 6.0, techo - 4.0, 6.0, 1.0, fade(TINTA, 0.6));
            tx += 12.0;
        }
        linea(l, (x0, techo), (x0, base), 2.0, fade(TINTA, 0.6));
        // Las ventanas de arriba con su balcon, y la de abajo con su reja.
        for (i, wx) in [x0 + 45.0, x0 + VANO - 45.0].into_iter().enumerate() {
            let encendida = rng.next_f32() < 0.55;
            let c = if encendida {
                fade(VENTANA, 0.7 + 0.2 * pulso)
            } else {
                APAGADA
            };
            let wy = techo + 22.0;
            rect(l, wx - 13.0, wy, 26.0, 38.0, c);
            draw_rect_lines(l, wx - 13.0, wy, 26.0, 38.0, 1.8);
            rect(l, wx - 18.0, wy + 38.0, 36.0, 4.0, CAL_SOMBRA);
            for bx in 0..5 {
                let px = wx - 16.0 + bx as f32 * 8.0;
                linea(l, (px, wy + 26.0), (px, wy + 38.0), 1.2, HIERRO);
            }
            linea(
                l,
                (wx - 18.0, wy + 26.0),
                (wx + 18.0, wy + 26.0),
                1.5,
                HIERRO,
            );
            // Una maceta de geranios en el balcon.
            if (i as i32 + k).rem_euclid(2) == 0 {
                circulo(l, wx + 10.0, wy + 22.0, 4.0, fade(GERANIO, 0.7));
            }
            if wy + 60.0 < base - 40.0 {
                let ry = base - 84.0;
                rect(l, wx - 12.0, ry, 24.0, 34.0, APAGADA);
                for bx in 0..4 {
                    let px = wx - 9.0 + bx as f32 * 6.0;
                    linea(l, (px, ry), (px, ry + 34.0), 1.2, HIERRO);
                }
                draw_rect_lines(l, wx - 12.0, ry, 24.0, 34.0, 1.5);
            }
        }
        // Un azulejo junto a la puerta, alguna vez.
        if rng.next_f32() < 0.4 {
            let ax = x0 + VANO * 0.5;
            rect(
                l,
                ax - 9.0,
                base - 70.0,
                18.0,
                18.0,
                fade(AZULEJO_CLARO, 0.8),
            );
            draw_rect_lines(l, ax - 9.0, base - 70.0, 18.0, 18.0, 1.2);
        }
    }
}

/// El puente de Triana, el de hierro, a medio hacer: tres arcos con sus aros,
/// el tablero sin cerrar en el del medio y la grua encima. Esta en su capa,
/// centrado en el rio de la calle: se ve al llegar a el.
fn puente(p: &Paseo, l: &Layout, cam: f32, pulso: f32) {
    let Some((a, b)) = el_rio(p) else { return };
    // Donde queda el centro del rio en esta capa cuando la camara lo tiene
    // en medio de la vista.
    let medio = (a + b) * 0.5;
    let x = VISTA_W * 0.5 + (medio - VISTA_W * ENCUADRE) * 0.6;
    let vista = cam * 0.6;
    if x + 700.0 < vista || x - 700.0 > vista + VISTA_W {
        return;
    }
    let (tablero, agua) = (290.0, 490.0);
    // Los estribos y las dos pilas, de piedra.
    for (px, w) in [
        (x - 600.0, 80.0),
        (x - 220.0, 50.0),
        (x + 220.0, 50.0),
        (x + 600.0, 80.0),
    ] {
        let arriba = if w > 60.0 { tablero } else { 420.0 };
        caja(l, px - w * 0.5, arriba, w, agua - arriba, PUENTE_CLARO, 2.0);
    }
    // Los arcos: un arco rebajado de hierro de pila a pila, y entre el arco
    // y el tablero, los aros, que son la firma del puente.
    for (x0, x1) in [
        (x - 560.0, x - 245.0),
        (x - 195.0, x + 195.0),
        (x + 245.0, x + 560.0),
    ] {
        let (c, s) = ((x1 - x0) * 0.5, 90.0);
        let r = (c * c + s * s) / (2.0 * s);
        let centro = (x0 + c, 420.0 - s + r);
        let y_arco = |px: f32| centro.1 - (r * r - (px - centro.0).powi(2)).max(0.0).sqrt();
        let n = 18;
        for i in 0..n {
            let (u, v) = (
                x0 + 2.0 * c * i as f32 / n as f32,
                x0 + 2.0 * c * (i + 1) as f32 / n as f32,
            );
            linea(l, (u, y_arco(u)), (v, y_arco(v)), 6.0, PUENTE);
            linea(l, (u, y_arco(u) + 10.0), (v, y_arco(v) + 10.0), 2.0, PUENTE);
        }
        let mut px = x0 + 26.0;
        while px < x1 - 20.0 {
            let hueco = y_arco(px) - tablero - 14.0;
            if hueco > 16.0 {
                let rr = hueco * 0.5 - 2.0;
                let s = l.to_screen(px, tablero + 14.0 + hueco * 0.5);
                draw_circle_lines(s.x, s.y, l.len(rr), l.len(2.5), PUENTE);
            }
            px += 34.0;
        }
    }
    // El tablero, con la barandilla y sus farolas. En el medio, sin cerrar.
    for (u, v) in [(x - 640.0, x - 110.0), (x + 110.0, x + 640.0)] {
        caja(l, u, tablero, v - u, 14.0, PUENTE, 1.5);
        linea(l, (u, tablero - 16.0), (v, tablero - 16.0), 2.5, PUENTE);
        let mut bx = u + 10.0;
        while bx < v {
            linea(l, (bx, tablero - 16.0), (bx, tablero), 1.5, PUENTE);
            bx += 16.0;
        }
        for fx in [u + 40.0, v - 40.0] {
            linea(l, (fx, tablero), (fx, tablero - 46.0), 2.5, PUENTE);
            circulo(l, fx, tablero - 50.0, 5.0, fade(VENTANA, 0.7 + 0.3 * pulso));
        }
    }
    // La grua de madera sobre el hueco, con el gancho colgando.
    let gx = x + 70.0;
    linea(l, (gx, tablero), (gx - 30.0, tablero - 130.0), 4.0, PUENTE);
    linea(
        l,
        (gx - 30.0, tablero - 130.0),
        (x - 80.0, tablero - 100.0),
        3.0,
        PUENTE,
    );
    linea(
        l,
        (x - 80.0, tablero - 100.0),
        (x - 80.0, tablero + 40.0),
        1.2,
        PUENTE,
    );
    rect(l, x - 86.0, tablero + 40.0, 12.0, 6.0, PUENTE);
}

/// La plaza de la Maestranza, al principio: el muro encalado con sus
/// pilastras de albero, los arcos del tendido y la Puerta del Principe. De
/// ahi se ha escapado el torito.
fn maestranza(l: &Layout, cam: f32) {
    let (x0, x1) = (180.0, 1360.0);
    if x1 < cam - 20.0 || x0 > cam + VISTA_W + 20.0 {
        return;
    }
    let (arriba, base) = (230.0, SUELO_Y);
    rect(l, x0, arriba, x1 - x0, base - arriba, CAL_FONDO);
    rect(l, x0, arriba, x1 - x0, 16.0, fade(ALBERO, 0.85));
    linea(l, (x0, arriba + 16.0), (x1, arriba + 16.0), 2.0, TINTA);
    linea(l, (x0, arriba), (x1, arriba), 2.5, TINTA);
    // El tendido de arriba: arcos sobre columnitas.
    let mut ax = x0 + 30.0;
    while ax < x1 - 30.0 {
        rect(l, ax - 14.0, arriba + 46.0, 28.0, 34.0, APAGADA);
        circulo(l, ax, arriba + 46.0, 14.0, APAGADA);
        arco(l, ax, arriba + 46.0, 14.0, 2.0, TINTA);
        ax += 48.0;
    }
    rect(l, x0, arriba + 84.0, x1 - x0, 8.0, fade(ALBERO, 0.85));
    linea(l, (x0, arriba + 92.0), (x1, arriba + 92.0), 1.5, TINTA);
    // Las pilastras de albero.
    let mut px = x0;
    while px <= x1 {
        rect(
            l,
            px - 7.0,
            arriba + 92.0,
            14.0,
            base - arriba - 92.0,
            fade(ALBERO, 0.85),
        );
        px += 120.0;
    }
    // La Puerta del Principe, con su reja.
    let cx = 760.0;
    rect(
        l,
        cx - 50.0,
        arriba + 110.0,
        100.0,
        base - arriba - 110.0,
        fade(ALMAGRA, 0.9),
    );
    circulo(l, cx, arriba + 150.0, 34.0, APAGADA);
    rect(
        l,
        cx - 34.0,
        arriba + 150.0,
        68.0,
        base - arriba - 150.0,
        APAGADA,
    );
    for i in 0..7 {
        let bx = cx - 30.0 + i as f32 * 10.0;
        linea(l, (bx, arriba + 130.0), (bx, base), 1.5, HIERRO);
    }
    arco(l, cx, arriba + 150.0, 34.0, 3.0, TINTA);
    linea(l, (x0, arriba), (x0, base), 3.0, TINTA);
    linea(l, (x1, arriba), (x1, base), 3.0, TINTA);
}

/// Los naranjos de las aceras y alguna farola fernandina, en la calle pero
/// detras de lo que se juega. Los naranjos de Sevilla dan sombra y azahar, y
/// sus naranjas laten con el compas.
fn naranjos(p: &Paseo, l: &Layout, cam: f32, pulso: f32, t: f32) {
    const CADA: f32 = 430.0;
    let mut x = (cam / CADA).floor() * CADA + 120.0 - CADA;
    while x < cam + VISTA_W + CADA {
        let n = (x / CADA) as i32;
        let lejos_del_borde = p.def.hay_suelo(x - 40.0) && p.def.hay_suelo(x + 40.0);
        if lejos_del_borde && x < p.def.puerta - 300.0 && !(180.0..1360.0).contains(&x) {
            if n % 3 == 1 {
                farola(l, x, pulso);
            } else {
                // El tronco, el alcorque y la copa redonda.
                rect(l, x - 18.0, SUELO_Y - 6.0, 36.0, 6.0, ALBERO_OSCURO);
                rect(l, x - 4.0, SUELO_Y - 70.0, 8.0, 64.0, MADERA);
                let mece = (t * 0.02 + n as f32).sin() * 2.0;
                for (dx, dy, r) in [
                    (-20.0, -86.0, 24.0),
                    (18.0, -90.0, 26.0),
                    (0.0, -110.0, 28.0),
                ] {
                    circulo(l, x + dx + mece, SUELO_Y + dy, r + 2.0, TINTA);
                    circulo(l, x + dx + mece, SUELO_Y + dy, r, VERDE);
                }
                let mut rng = Pcg32::new(0x0A_2A05 + n as u64);
                for _ in 0..7 {
                    let ox = x - 34.0 + rng.next_f32() * 68.0 + mece;
                    let oy = SUELO_Y - 130.0 + rng.next_f32() * 56.0;
                    circulo(l, ox, oy, 3.2 + pulso * 0.6, NARANJA);
                }
            }
        }
        x += CADA;
    }
}

/// Una farola fernandina: el baculo de hierro y el farol de seis caras.
fn farola(l: &Layout, x: f32, pulso: f32) {
    let arriba = 300.0;
    tri(
        l,
        (x - 8.0, arriba + 10.0),
        (x - 80.0, SUELO_Y),
        (x + 80.0, SUELO_Y),
        fade(LUZ, 0.05 + 0.05 * pulso),
    );
    rect(l, x - 3.0, arriba, 6.0, SUELO_Y - arriba, HIERRO);
    rect(l, x - 8.0, SUELO_Y - 16.0, 16.0, 16.0, HIERRO);
    tri(
        l,
        (x - 13.0, arriba - 2.0),
        (x + 13.0, arriba - 2.0),
        (x, arriba + 10.0),
        HIERRO,
    );
    rect(
        l,
        x - 9.0,
        arriba - 26.0,
        18.0,
        24.0,
        fade(LUZ, 0.8 + 0.2 * pulso),
    );
    draw_rect_lines(l, x - 9.0, arriba - 26.0, 18.0, 24.0, 2.0);
    tri(
        l,
        (x - 13.0, arriba - 26.0),
        (x, arriba - 40.0),
        (x + 13.0, arriba - 26.0),
        HIERRO,
    );
}

// ---------------------------------------------------------------------------
// La calle: el albero, el rio, las barcas, los balcones y el tablao
// ---------------------------------------------------------------------------

/// Lo que se pisa en Triana: el albero con su bordillo de cal, el rio en los
/// fosos, las barcas, los balcones y corredores, el tablao y la puerta.
pub fn calle(
    p: &Paseo,
    l: &Layout,
    alpha: f32,
    cam: f32,
    t: f32,
    pulso: f32,
    tintas: (Color, Color),
) {
    rio(l, cam, t);
    suelo(p, l, cam);
    barcas(p, l, t);
    plataformas(p, l);
    tablao(p, l, pulso);
    puerta(p, l, alpha, t, pulso, tintas);
}

/// El Guadalquivir a lo ancho de la vista, con el naranja de la tarde
/// meciendose encima. La acera se pinta encima.
fn rio(l: &Layout, cam: f32, t: f32) {
    rect(
        l,
        cam - 10.0,
        SUELO_Y + 16.0,
        VISTA_W + 20.0,
        VISTA_H - SUELO_Y,
        RIO,
    );
    for i in 0..5 {
        let y = SUELO_Y + 26.0 + i as f32 * 10.0;
        let d = (t * 0.025 + i as f32).sin() * 14.0;
        let mut x = (cam / 80.0).floor() * 80.0 + (i * 29) as f32;
        while x < cam + VISTA_W + 80.0 {
            let c = if i % 2 == 0 { NARANJA } else { SOL };
            linea(l, (x + d, y), (x + d + 26.0, y), 1.5, fade(c, 0.22));
            x += 80.0;
        }
    }
}

/// El albero de la calle: la tierra ocre con su bordillo de cal encima, y en
/// los muelles, el ladrillo. Los bordes de cada foso, gordos.
fn suelo(p: &Paseo, l: &Layout, cam: f32) {
    let fondo = VISTA_H + 10.0;
    for &(a, b) in &p.def.suelo {
        if b < cam - 20.0 || a > cam + VISTA_W + 20.0 {
            continue;
        }
        let (a0, b0) = (a.max(cam - 20.0), b.min(cam + VISTA_W + 20.0));
        rect(l, a0, SUELO_Y, b0 - a0, fondo - SUELO_Y, ALBERO);
        // El ladrillo del muro del muelle, al tresbolillo.
        for fila in 0..4 {
            let y = SUELO_Y + 22.0 + fila as f32 * 13.0;
            linea(l, (a0, y), (b0, y), 1.0, fade(ALBERO_OSCURO, 0.8));
            let desfase = if fila % 2 == 0 { 0.0 } else { 17.0 };
            let mut x = ((a0 - desfase) / 34.0).floor() * 34.0 + desfase;
            while x < b0 {
                if x > a && x < b {
                    linea(l, (x, y), (x, y + 13.0), 1.0, fade(ALBERO_OSCURO, 0.8));
                }
                x += 34.0;
            }
        }
        // El bordillo de cal, que es lo que se ve pisar.
        rect(l, a0, SUELO_Y, b0 - a0, 12.0, CAL);
        linea(l, (a0, SUELO_Y), (b0, SUELO_Y), 3.5, TINTA);
        linea(l, (a0, SUELO_Y + 12.0), (b0, SUELO_Y + 12.0), 1.5, TINTA);
        for x in [a, b] {
            if x > cam - 20.0 && x < cam + VISTA_W + 20.0 {
                linea(l, (x, SUELO_Y), (x, fondo), 4.5, TINTA);
            }
        }
    }
}

/// Las barcas del puente: el casco de madera con sus tablas, la cubierta clara
/// que es lo que se pisa, las proas levantadas y la espuma donde tocan el agua.
fn barcas(p: &Paseo, l: &Layout, t: f32) {
    for (i, pl) in p.def.plataformas.iter().enumerate() {
        if pl.mueve.is_none() {
            continue;
        }
        let (x, y, w) = (pl.x, pl.y, pl.ancho);
        let quilla = y + 26.0;
        for (g, c) in [(2.5, TINTA), (0.0, MADERA)] {
            tri(
                l,
                (x - 10.0 - g, y - 6.0 - g),
                (x + 18.0, quilla + g),
                (x + 18.0, y),
                c,
            );
            tri(
                l,
                (x + w + 10.0 + g, y - 6.0 - g),
                (x + w - 18.0, quilla + g),
                (x + w - 18.0, y),
                c,
            );
            rect(l, x + 18.0 - g * 0.5, y, w - 36.0 + g, 26.0 + g, c);
        }
        linea(
            l,
            (x + 14.0, y + 12.0),
            (x + w - 14.0, y + 12.0),
            1.2,
            fade(TINTA, 0.6),
        );
        linea(
            l,
            (x + 18.0, y + 20.0),
            (x + w - 18.0, y + 20.0),
            1.2,
            fade(TINTA, 0.6),
        );
        // La cubierta.
        rect(l, x, y, w, 7.0, MADERA_CLARA);
        linea(l, (x, y), (x + w, y), 3.0, TINTA);
        let mut tx = x + 24.0;
        while tx < x + w - 10.0 {
            linea(l, (tx, y + 1.0), (tx, y + 7.0), 1.0, fade(TINTA, 0.5));
            tx += 24.0;
        }
        // Un noray con su cabo, y la espuma.
        rect(l, x + w * 0.5 - 4.0, y - 8.0, 8.0, 8.0, HIERRO);
        let ola = (t * 0.08 + i as f32).sin() * 3.0;
        for k in 0..4 {
            let ex = x + 10.0 + k as f32 * (w - 20.0) / 3.0;
            linea(
                l,
                (ex - 8.0 + ola, SUELO_Y + 18.0),
                (ex + 8.0 + ola, SUELO_Y + 18.0),
                2.0,
                fade(HUESO, 0.5),
            );
        }
    }
}

/// Balcones, poyetes y los corredores del corral. Lo ancho es un corredor,
/// con su barandilla de madera y sus pies derechos; lo bajo, un poyete de cal
/// con su cenefa de azulejo; lo demas, un balcon con reja y geranios.
fn plataformas(p: &Paseo, l: &Layout) {
    let def = &p.def;
    for pl in def.plataformas.iter().filter(|pl| pl.mueve.is_none()) {
        let (x, y, w) = (pl.x, pl.y, pl.ancho);
        if SUELO_Y - y < 70.0 {
            // El poyete, macizo hasta la acera.
            let abajo = def.suelo_en(x + w * 0.5).unwrap_or(SUELO_Y);
            caja(l, x, y, w, abajo - y, CAL, 2.5);
            let mut ax = x + 4.0;
            while ax < x + w - 14.0 {
                let c = if ((ax - x) / 14.0) as i32 % 2 == 0 {
                    AZULEJO
                } else {
                    AMARILLO
                };
                rect(l, ax, y + 12.0, 12.0, 12.0, c);
                ax += 14.0;
            }
            linea(l, (x, y), (x + w, y), 3.0, TINTA);
        } else if w >= 200.0 {
            // El corredor: el piso de madera, la barandilla y los pies
            // derechos hasta lo que tiene debajo.
            let debajo = def
                .plataformas
                .iter()
                .filter(|o| o.mueve.is_none() && o.y > y && o.x < x + w && o.x + o.ancho > x)
                .map(|o| o.y)
                .fold(def.suelo_en(x + w * 0.5).unwrap_or(SUELO_Y), f32::min);
            let mut px = x + 10.0;
            while px < x + w {
                rect(l, px - 3.0, y + 10.0, 6.0, debajo - y - 10.0, MADERA);
                linea(l, (px - 3.0, y + 10.0), (px - 3.0, debajo), 1.2, TINTA);
                px += (w - 20.0) / 3.0;
            }
            caja(l, x - 2.0, y, w + 4.0, 10.0, MADERA_CLARA, 2.0);
            linea(l, (x, y - 26.0), (x + w, y - 26.0), 3.0, MADERA);
            let mut bx = x + 6.0;
            while bx < x + w {
                linea(l, (bx, y - 26.0), (bx, y), 2.0, MADERA);
                bx += 14.0;
            }
            // La ropa tendida, que un corral sin ropa no es un corral.
            for (k, c) in [HUESO, AZULEJO_CLARO, GERANIO].into_iter().enumerate() {
                let rx = x + 40.0 + k as f32 * (w - 80.0) / 2.0;
                rect(l, rx - 8.0, y - 24.0, 16.0, 18.0, fade(c, 0.85));
            }
            linea(l, (x, y), (x + w, y), 3.0, TINTA);
        } else {
            // El balcon: la losa con sus mensulas y la reja.
            caja(l, x - 2.0, y, w + 4.0, 9.0, CAL, 2.0);
            let n = ((w / 40.0) as i32).max(2);
            for i in 0..n {
                let mx = x + 12.0 + (w - 24.0) * i as f32 / (n - 1) as f32;
                tri(
                    l,
                    (mx - 6.0, y + 9.0),
                    (mx + 6.0, y + 9.0),
                    (mx, y + 28.0),
                    TINTA,
                );
            }
            linea(l, (x, y - 24.0), (x + w, y - 24.0), 2.5, HIERRO);
            let mut bx = x + 4.0;
            while bx < x + w {
                linea(l, (bx, y - 24.0), (bx, y), 1.5, HIERRO);
                bx += 9.0;
            }
            for mx in [x + 10.0, x + w - 10.0] {
                rect(l, mx - 6.0, y - 10.0, 12.0, 10.0, ALMAGRA);
                circulo(l, mx - 3.0, y - 14.0, 4.0, GERANIO);
                circulo(l, mx + 3.0, y - 15.0, 3.5, GERANIO);
                circulo(l, mx, y - 11.0, 3.0, VERDE_CLARO);
            }
            linea(l, (x, y), (x + w, y), 3.0, TINTA);
        }
    }
}

/// El tablao mientras esta cerrado: las cancelas de hierro a los lados, que
/// dicen "de aqui no se sale", la guirnalda de farolillos de feria de punta a
/// punta y el foco sobre la cantaora.
fn tablao(p: &Paseo, l: &Layout, pulso: f32) {
    let Some(a) = p.cierre() else {
        return;
    };
    if let Some(e) = p.portero() {
        let pies = p.def.altura(e.pos.x);
        tri(
            l,
            (e.pos.x - 20.0, 0.0),
            (e.pos.x - 100.0, pies),
            (e.pos.x + 100.0, pies),
            fade(LUZ, 0.07 + 0.04 * pulso),
        );
        let s = l.to_screen(e.pos.x, pies);
        draw_ellipse(s.x, s.y, l.len(90.0), l.len(9.0), 0.0, fade(LUZ, 0.12));
    }
    // La guirnalda: farolillos de lunares colgando en curva.
    let n = 16;
    for i in 0..=n {
        let f = i as f32 / n as f32;
        let fx = a + 20.0 + (VISTA_W - 40.0) * f;
        let fy = 120.0 + 40.0 * (1.0 - (2.0 * f - 1.0).powi(2));
        if i < n {
            let g = (i + 1) as f32 / n as f32;
            let gx = a + 20.0 + (VISTA_W - 40.0) * g;
            let gy = 120.0 + 40.0 * (1.0 - (2.0 * g - 1.0).powi(2));
            linea(l, (fx, fy), (gx, gy), 1.5, TINTA);
        }
        let c = [ROJO, CAL, VERDE_CLARO][i % 3];
        circulo(l, fx, fy + 12.0, 9.0, TINTA);
        circulo(l, fx, fy + 12.0, 7.5, fade(c, 0.85 + 0.15 * pulso));
        circulo(
            l,
            fx - 2.0,
            fy + 10.0,
            1.6,
            if i % 3 == 1 { ROJO } else { CAL },
        );
        circulo(
            l,
            fx + 2.5,
            fy + 14.0,
            1.6,
            if i % 3 == 1 { ROJO } else { CAL },
        );
    }
    // Las cancelas.
    for x in [a + 12.0, a + VISTA_W - 12.0] {
        let s = p.def.altura(x);
        for dx in [-10.0, -3.0, 4.0, 11.0] {
            linea(l, (x + dx, s - 70.0), (x + dx, s), 2.0, HIERRO);
        }
        linea(l, (x - 12.0, s - 70.0), (x + 13.0, s - 70.0), 3.0, HIERRO);
        linea(l, (x - 12.0, s - 36.0), (x + 13.0, s - 36.0), 2.0, HIERRO);
        arco(l, x, s - 70.0, 12.0, 2.0, HIERRO);
    }
}

/// La puerta: la del tablao de la calle Betis, encalada, con su marco de
/// azulejos, el cartel de ceramica con el baile y dos hojas de madera que
/// **se abren** segun te acercas. Dentro, la luz y una bailaora girando.
fn puerta(p: &Paseo, l: &Layout, alpha: f32, t: f32, pulso: f32, (tinta, ropa): (Color, Color)) {
    let x = p.def.puerta;
    let cam = calle::camara(p, alpha);
    if x + 600.0 < cam || x - 400.0 > cam + VISTA_W {
        return;
    }
    let s = p.def.altura(x);
    let (cx, ancho, boca) = (x + 40.0, 130.0, s - 150.0);

    // La fachada de cal que cierra la calle, con zocalo y alero.
    let arriba = s - 300.0;
    rect(l, x - 220.0, arriba, 900.0, s - arriba, CAL);
    rect(l, x - 220.0, s - 40.0, 900.0, 40.0, ALMAGRA);
    linea(l, (x - 220.0, s - 40.0), (x + 680.0, s - 40.0), 2.0, TINTA);
    rect(l, x - 226.0, arriba - 14.0, 912.0, 14.0, TEJA);
    linea(l, (x - 226.0, arriba), (x + 686.0, arriba), 2.5, TINTA);
    linea(l, (x - 220.0, arriba), (x - 220.0, s), 4.0, TINTA);
    // Ventanas con reja y luz a los lados.
    for wx in [x - 140.0, cx + 150.0, cx + 270.0] {
        let c = fade(VENTANA, 0.85 + 0.15 * pulso);
        rect(l, wx - 26.0, s - 210.0, 52.0, 90.0, c);
        for i in 0..6 {
            let bx = wx - 22.0 + i as f32 * 9.0;
            linea(l, (bx, s - 210.0), (bx, s - 120.0), 2.0, HIERRO);
        }
        linea(
            l,
            (wx - 26.0, s - 170.0),
            (wx + 26.0, s - 170.0),
            2.0,
            HIERRO,
        );
        draw_rect_lines(l, wx - 26.0, s - 210.0, 52.0, 90.0, 3.0);
        rect(l, wx - 32.0, s - 120.0, 64.0, 6.0, CAL);
        for mx in [wx - 18.0, wx + 18.0] {
            rect(l, mx - 6.0, s - 132.0, 12.0, 12.0, ALMAGRA);
            circulo(l, mx, s - 136.0, 5.0, GERANIO);
        }
    }

    // La luz de dentro y la bailaora dando la vuelta en contraluz.
    let abierta = ((p.jugadora.render_pos(alpha).x - (x - 420.0)) / 320.0).clamp(0.0, 1.0);
    circulo(l, cx, boca, ancho * 0.5, fade(LUZ, 0.95));
    rect(l, cx - ancho * 0.5, boca, ancho, s - boca, fade(LUZ, 0.95));
    let sombra = fade(TINTA, 0.55);
    let (vs, vc) = sin_cos(t * 0.09);
    circulo(l, cx, s - 92.0, 6.0, sombra);
    rect(l, cx - 6.0, s - 84.0, 12.0, 30.0, sombra);
    tri(
        l,
        (cx, s - 60.0),
        (cx - 22.0 - 10.0 * vc, s),
        (cx + 22.0 + 10.0 * vs, s),
        sombra,
    );
    linea(l, (cx, s - 80.0), (cx + 16.0 * vc, s - 106.0), 3.0, sombra);

    // El marco de azulejos alrededor de la boca.
    let lado = 12.0;
    let mut ty = boca - ancho * 0.5 - lado;
    while ty < s - 40.0 {
        for (k, bx) in [cx - ancho * 0.5 - lado, cx + ancho * 0.5]
            .into_iter()
            .enumerate()
        {
            let c = if ((ty / lado) as i32 + k as i32) % 2 == 0 {
                AZULEJO
            } else {
                AMARILLO
            };
            rect(l, bx, ty, lado, lado, c);
            draw_rect_lines(l, bx, ty, lado, lado, 1.0);
        }
        ty += lado;
    }

    // Las hojas de madera, que giran sobre sus bisagras.
    let hoja = ancho * 0.5 * (1.0 - abierta * 0.85);
    for (x0, dir) in [(cx - ancho * 0.5, 1.0), (cx + ancho * 0.5, -1.0)] {
        let x1 = x0 + hoja * dir;
        let (a, b) = (x0.min(x1), x0.max(x1));
        rect(l, a, boca, b - a, s - boca, MADERA);
        draw_rect_lines(l, a, boca, b - a, s - boca, 2.5);
        if b - a > 12.0 {
            draw_rect_lines(l, a + 5.0, boca + 16.0, b - a - 10.0, 50.0, 1.5);
            draw_rect_lines(l, a + 5.0, boca + 80.0, b - a - 10.0, 60.0, 1.5);
        }
    }
    arco(l, cx, boca, ancho * 0.5, 5.0, TINTA);
    for dx in [-ancho * 0.5, ancho * 0.5] {
        linea(l, (cx + dx, boca), (cx + dx, s), 5.0, TINTA);
    }
    // Los faroles a los lados de la puerta.
    for fx in [cx - ancho * 0.5 - 36.0, cx + ancho * 0.5 + 36.0] {
        linea(l, (fx, s - 230.0), (fx, s - 214.0), 2.0, HIERRO);
        rect(
            l,
            fx - 8.0,
            s - 214.0,
            16.0,
            22.0,
            fade(LUZ, 0.85 + 0.15 * pulso),
        );
        draw_rect_lines(l, fx - 8.0, s - 214.0, 16.0, 22.0, 2.0);
    }

    // El cartel de ceramica, en las dos tintas del baile.
    let cy = boca - ancho * 0.5 - 60.0;
    rect(l, cx - 92.0, cy, 184.0, 40.0, TINTA);
    rect(l, cx - 88.0, cy + 4.0, 176.0, 32.0, ropa);
    rect(l, cx - 88.0, cy + 4.0, 176.0, 5.0, tinta);
    rect(l, cx - 88.0, cy + 31.0, 176.0, 5.0, tinta);
    let q = l.to_screen(cx, cy + 30.0);
    let nombre = nombre(&p.def.jefe).to_uppercase();
    fuentes::centrado(&nombre, q.x, q.y, l.len(21.0), Cara::Titulo, TINTA);

    // La luz se derrama a la calle segun se abre, y al llegar lo inunda.
    let llegada = p
        .completado
        .map_or(0.0, |d| (((p.tick - d) as f32 + alpha) / 40.0).min(1.0));
    let derrame = fade(LUZ, 0.10 + 0.25 * abierta + 0.3 * llegada);
    let (izq, der) = (cx - ancho * 0.5, cx + ancho * 0.5);
    tri(l, (izq, s), (der, s), (der + 160.0, VISTA_H), derrame);
    tri(
        l,
        (izq, s),
        (der + 160.0, VISTA_H),
        (izq - 160.0, VISTA_H),
        derrame,
    );
}

// ---------------------------------------------------------------------------
// Los enemigos
// ---------------------------------------------------------------------------

/// Los de Triana. Todos se preparan en el tiempo de antes del acento, que es
/// lo que deja ver venir el quiebro.
pub fn enemigo(
    e: &Enemigo,
    l: &Layout,
    tick: u64,
    t: f32,
    tiempo: u32,
    tinte: &dyn Fn(Color) -> Color,
) {
    match e.tipo {
        Tipo::Bailaora => bailaora(e, l, tick, tiempo, tinte),
        Tipo::Guitarrista => guitarrista(e, l, tick, tiempo, tinte),
        Tipo::Palmas => palmas(e, l, tick, tiempo, tinte),
        Tipo::Clavel => clavel(e, l, t, tinte),
        Tipo::Torito => torito(e, l, tick, t, tiempo, tinte),
        Tipo::Cantaora => cantaora(e, l, tick, tiempo, tinte),
        _ => {}
    }
}

/// Un anillo blanco que late, alrededor de lo que se parria.
fn anillo_rosa(l: &Layout, x: f32, y: f32, r: f32, t: f32) {
    let k = 0.5 + 0.5 * (t * 0.2).sin();
    let s = l.to_screen(x, y);
    draw_circle_lines(
        s.x,
        s.y,
        l.len(r + 4.0 * k),
        2.0,
        fade(WHITE, 0.4 + 0.4 * k),
    );
}

/// Una bailaora con su traje de lunares y la bata de cola. Entre dos acentos
/// la cola va arrastrando por detras y ella zapatea; antes del acento sube el
/// brazo, y en el acento patea la bata, que barre el suelo por delante hasta
/// donde dice `Enemigo::bata`: lo que se ve es lo que pega.
fn bailaora(e: &Enemigo, l: &Layout, tick: u64, tiempo: u32, tinte: &dyn Fn(Color) -> Color) {
    let (x, d, k) = (e.pos.x, e.dir, e.gesto);
    let pies = e.pos.y + 40.0;
    let sube = prepara(tick, tiempo, 12.0);
    // El zapateado: el tacon arriba y abajo, mientras no patea.
    let tacon = if k == 0.0 {
        ((tick % 10) as f32 / 10.0 * TAU).sin().abs() * 3.0
    } else {
        0.0
    };
    let (traje, lunar, piel) = (tinte(ROJO), tinte(HUESO), tinte(PIEL));
    // La cola: por detras arrastrando, o por delante en la patada.
    let estira = if k > 0.0 { (PI * (1.0 - k)).sin() } else { 0.0 };
    let punta = if k > 0.0 {
        x + d * (22.0 + BATA * estira)
    } else {
        x - d * 46.0
    };
    let bajo = (x - d * 4.0, pies - 6.0);
    let volantes: Vec<((f32, f32), f32)> = (0..6)
        .map(|i| {
            let f = i as f32 / 5.0;
            let vx = bajo.0 + (punta - bajo.0) * f;
            let vy = pies - 12.0 + 4.0 * f - 10.0 * estira * (1.0 - f) * f * 4.0;
            ((vx, vy), 12.0 - 5.0 * f)
        })
        .collect();
    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        for &((vx, vy), r) in &volantes {
            circulo(l, vx, vy, r + g, c(traje));
        }
        // La falda, del talle al suelo, y el cuerpo.
        tri(
            l,
            (x - 6.0 - g, pies - 44.0 - g),
            (x - 20.0 - g, pies - tacon + g),
            (x + 20.0 + g, pies - tacon + g),
            c(traje),
        );
        rect(
            l,
            x - 7.0 - g,
            pies - 66.0 - g,
            14.0 + 2.0 * g,
            24.0 + g,
            c(traje),
        );
        // Un brazo en jarras y el otro arriba, que sube antes del acento.
        let hombro = (x + d * 5.0, pies - 62.0);
        let codo = (x + d * 16.0, pies - 74.0 - 12.0 * sube);
        let mano = (x + d * (8.0 - 6.0 * k), pies - 92.0 - 8.0 * sube + 10.0 * k);
        linea(l, hombro, codo, 4.0 + 2.0 * g, c(piel));
        linea(l, codo, mano, 4.0 + 2.0 * g, c(piel));
        linea(
            l,
            (x - d * 5.0, pies - 62.0),
            (x - d * 15.0, pies - 52.0),
            4.0 + 2.0 * g,
            c(piel),
        );
        linea(
            l,
            (x - d * 15.0, pies - 52.0),
            (x - d * 6.0, pies - 45.0),
            4.0 + 2.0 * g,
            c(piel),
        );
        circulo(l, x, pies - 74.0, 7.0 + g, c(piel));
    });
    // Los lunares, los volantes de la falda, el mono y el clavel del pelo.
    for &((vx, vy), r) in volantes.iter().step_by(2) {
        circulo(l, vx - r * 0.3, vy - r * 0.3, 2.0, lunar);
    }
    for (dx, dy) in [(-8.0, -12.0), (6.0, -20.0), (-3.0, -30.0), (9.0, -8.0)] {
        circulo(l, x + dx, pies + dy, 2.0, lunar);
    }
    linea(
        l,
        (x - 14.0, pies - 16.0),
        (x + 14.0, pies - 16.0),
        1.5,
        TINTA,
    );
    linea(
        l,
        (x - 10.0, pies - 28.0),
        (x + 10.0, pies - 28.0),
        1.5,
        TINTA,
    );
    arco(l, x, pies - 76.0, 7.0, 3.5, TINTA);
    circulo(l, x - d * 7.0, pies - 78.0, 5.0, TINTA);
    circulo(l, x + d * 2.0, pies - 83.0, 3.6, TINTA);
    circulo(l, x + d * 2.0, pies - 83.0, 2.6, tinte(ROJO));
    circulo(l, x + d * 3.0, pies - 73.0, 1.3, TINTA);
}

/// Un guitarrista sentado en su silla de enea, con el sombrero de ala ancha
/// y la guitarra en la pierna. Antes del acento levanta la mano del rasgueo,
/// y en el acento la baja de golpe: ahi sale la falseta.
fn guitarrista(e: &Enemigo, l: &Layout, tick: u64, tiempo: u32, tinte: &dyn Fn(Color) -> Color) {
    let (x, d) = (e.pos.x, e.dir);
    let pies = e.pos.y + 34.0;
    let alza = prepara(tick, tiempo, 10.0) * (1.0 - e.gesto);
    let (traje, camisa, piel) = (tinte(NEGRO), tinte(HUESO), tinte(PIEL));
    // La silla.
    rect(l, x - d * 12.0 - 2.0, pies - 52.0, 4.0, 52.0, MADERA);
    rect(l, x - 12.0, pies - 20.0, 24.0, 5.0, MADERA_CLARA);
    linea(l, (x - 10.0, pies - 15.0), (x - 10.0, pies), 2.5, MADERA);
    linea(l, (x + 10.0, pies - 15.0), (x + 10.0, pies), 2.5, MADERA);
    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        // El muslo, la pierna y el cuerpo.
        linea(
            l,
            (x, pies - 22.0),
            (x + d * 16.0, pies - 22.0),
            7.0 + 2.0 * g,
            c(traje),
        );
        linea(
            l,
            (x + d * 16.0, pies - 22.0),
            (x + d * 16.0, pies),
            6.0 + 2.0 * g,
            c(traje),
        );
        rect(
            l,
            x - 8.0 - g,
            pies - 52.0 - g,
            16.0 + 2.0 * g,
            30.0 + g,
            c(traje),
        );
        tri(
            l,
            (x - 4.0, pies - 52.0),
            (x + 4.0, pies - 52.0),
            (x, pies - 38.0),
            c(camisa),
        );
        circulo(l, x, pies - 60.0, 7.0 + g, c(piel));
        // La guitarra: la caja en ocho, el mastil hacia ti.
        circulo(l, x + d * 12.0, pies - 28.0, 9.0 + g, c(MADERA_CLARA));
        circulo(l, x + d * 6.0, pies - 35.0, 7.0 + g, c(MADERA_CLARA));
        linea(
            l,
            (x + d * 6.0, pies - 36.0),
            (x + d * 30.0, pies - 52.0),
            3.0 + 2.0 * g,
            c(MADERA),
        );
        // La mano del rasgueo, arriba antes del acento.
        let mano = (x + d * 9.0, pies - 32.0 - 12.0 * alza);
        linea(l, (x + d * 4.0, pies - 46.0), mano, 4.0 + 2.0 * g, c(traje));
        circulo(l, mano.0, mano.1, 3.0 + g, c(piel));
    });
    circulo(l, x + d * 10.0, pies - 30.0, 2.6, TINTA);
    rect(l, x + d * 30.0 - 3.0, pies - 56.0, 6.0, 6.0, TINTA);
    // El sombrero cordobes.
    rect(l, x - 14.0, pies - 67.0, 28.0, 3.5, TINTA);
    rect(l, x - 7.0, pies - 76.0, 14.0, 9.0, TINTA);
    circulo(l, x + d * 3.0, pies - 60.0, 1.3, TINTA);
}

/// Un palmero de chaquetilla corta: abre las manos en el tiempo de antes del
/// acento y en el acento las junta, que es cuando estalla el golpe.
fn palmas(e: &Enemigo, l: &Layout, tick: u64, tiempo: u32, tinte: &dyn Fn(Color) -> Color) {
    let (x, d) = (e.pos.x, e.dir);
    let pies = e.pos.y + 38.0;
    let abre = prepara(tick, tiempo, 14.0) * (1.0 - e.gesto);
    let (traje, camisa, piel) = (tinte(NEGRO), tinte(HUESO), tinte(PIEL));
    let manos = (x + d * 14.0, pies - 52.0);
    let sep = 3.0 + 14.0 * abre;
    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        linea(
            l,
            (x - 4.0, pies - 30.0),
            (x - 6.0, pies),
            6.0 + 2.0 * g,
            c(traje),
        );
        linea(
            l,
            (x + 4.0, pies - 30.0),
            (x + 6.0, pies),
            6.0 + 2.0 * g,
            c(traje),
        );
        rect(
            l,
            x - 9.0 - g,
            pies - 62.0 - g,
            18.0 + 2.0 * g,
            34.0 + g,
            c(camisa),
        );
        rect(
            l,
            x - 10.0 - g,
            pies - 62.0 - g,
            20.0 + 2.0 * g,
            18.0 + g,
            c(traje),
        );
        for lado in [-1.0, 1.0] {
            let mano = (manos.0, manos.1 + lado * sep * 0.5);
            linea(l, (x, pies - 58.0), mano, 4.0 + 2.0 * g, c(traje));
            circulo(l, mano.0, mano.1, 4.0 + g, c(piel));
        }
        circulo(l, x, pies - 70.0, 7.5 + g, c(piel));
    });
    arco(l, x, pies - 72.0, 7.5, 4.0, TINTA);
    circulo(l, x + d * 3.0, pies - 70.0, 1.3, TINTA);
    // Las rayitas del golpe, mientras dura.
    if e.gesto > 0.0 {
        for i in 0..6 {
            let (s, co) = sin_cos(TAU * i as f32 / 6.0);
            let (r0, r1) = (8.0, 8.0 + 12.0 * e.gesto);
            linea(
                l,
                (manos.0 + co * r0, manos.1 + s * r0),
                (manos.0 + co * r1, manos.1 + s * r1),
                2.0,
                fade(AMARILLO, e.gesto),
            );
        }
    }
}

/// Un clavel reventon dando vueltas en el aire, con su tallo detras: hacia
/// donde apunta el tallo es de donde viene, y el quiebro se ve en el giro.
fn clavel(e: &Enemigo, l: &Layout, t: f32, tinte: &dyn Fn(Color) -> Color) {
    let (x, y) = (e.pos.x, e.pos.y);
    let v = hacia(e.vel.x, e.vel.y);
    let flor = if e.rosa { ROSA } else { tinte(ROJO) };
    let tallo = (x - v.x * 18.0, y - v.y * 18.0);
    linea(l, (x, y), tallo, 5.0, TINTA);
    linea(l, (x, y), tallo, 2.5, VERDE_CLARO);
    let hoja = (x - v.x * 10.0 - v.y * 6.0, y - v.y * 10.0 + v.x * 6.0);
    circulo(l, hoja.0, hoja.1, 3.0, VERDE_CLARO);
    let giro = t * 0.15 + x * 0.01;
    con_tinta(|g, es_tinta| {
        let c = if es_tinta { TINTA } else { flor };
        for i in 0..7 {
            let (s, co) = sin_cos(giro + TAU * i as f32 / 7.0);
            circulo(l, x + co * 6.0, y + s * 6.0, 4.5 + g, c);
        }
        circulo(l, x, y, 5.5 + g, c);
    });
    let oscuro = if e.rosa { WHITE } else { ROJO_OSCURO };
    for i in 0..7 {
        let (s, co) = sin_cos(giro + TAU * (i as f32 + 0.5) / 7.0);
        linea(
            l,
            (x + co * 4.0, y + s * 4.0),
            (x + co * 9.0, y + s * 9.0),
            1.2,
            fade(oscuro, 0.7),
        );
    }
    if e.rosa {
        anillo_rosa(l, x, y, 18.0, t);
    }
}

/// El torito: negro, bajo y con los cuernos blancos. Escarbando patea la
/// tierra; antes del acento baja la testuz, y embistiendo va con rayas de
/// velocidad. Al clavarse levanta una polvareda de albero.
fn torito(e: &Enemigo, l: &Layout, tick: u64, t: f32, tiempo: u32, tinte: &dyn Fn(Color) -> Color) {
    let (x, d) = (e.pos.x, e.dir);
    let pies = e.pos.y + 26.0;
    let corre = e.embiste();
    let baja = if corre {
        1.0
    } else {
        prepara(tick, tiempo, 16.0)
    };
    let negro = tinte(NEGRO);
    let paso = if corre { (t * 0.7).sin() * 8.0 } else { 0.0 };
    let escarba = if !corre && e.vel.x == 0.0 {
        (t * 0.35).sin().max(0.0) * 10.0
    } else {
        0.0
    };
    let cabeza = (x + d * 30.0, pies - 30.0 + 10.0 * baja);
    con_tinta(|g, es_tinta| {
        let c = if es_tinta { TINTA } else { negro };
        // Las patas: las de delante corren o escarban.
        for (px, dx) in [
            (-16.0, -paso),
            (-8.0, paso),
            (12.0, paso - escarba),
            (18.0, -paso),
        ] {
            linea(
                l,
                (x + d * px, pies - 24.0),
                (x + d * (px + dx), pies),
                5.0 + 2.0 * g,
                c,
            );
        }
        let s = l.to_screen(x, pies - 32.0);
        draw_ellipse(s.x, s.y, l.len(28.0 + g), l.len(15.0 + g), 0.0, c);
        circulo(l, x + d * 18.0, pies - 38.0, 12.0 + g, c);
        circulo(l, cabeza.0, cabeza.1, 10.0 + g, c);
        // El rabo, con su borla.
        linea(
            l,
            (x - d * 26.0, pies - 38.0),
            (x - d * 34.0, pies - 22.0 - paso * 0.5),
            2.0 + 2.0 * g,
            c,
        );
        circulo(l, x - d * 34.0, pies - 20.0 - paso * 0.5, 3.0 + g, c);
    });
    // Los cuernos, hacia delante y arriba, y el ojo.
    for dy in [-4.0, 2.0] {
        let base = (cabeza.0 + d * 2.0, cabeza.1 - 6.0 + dy);
        let codo = (cabeza.0 + d * 12.0, cabeza.1 - 12.0 + dy);
        let punta = (cabeza.0 + d * 20.0, cabeza.1 - 22.0 + dy * 0.5 + 8.0 * baja);
        linea(l, base, codo, 5.0, TINTA);
        linea(l, codo, punta, 3.5, TINTA);
        linea(l, base, codo, 3.0, HUESO);
        linea(l, codo, punta, 1.8, HUESO);
    }
    circulo(l, cabeza.0 + d * 4.0, cabeza.1 - 2.0, 1.8, ROJO);
    if corre {
        for (i, dy) in [-36.0, -26.0, -14.0].into_iter().enumerate() {
            let largo = 30.0 + 12.0 * i as f32;
            linea(
                l,
                (x - d * 36.0, pies + dy),
                (x - d * (36.0 + largo), pies + dy),
                2.0,
                fade(HUESO, 0.6),
            );
        }
    }
    // La polvareda del frenazo, por delante, y la tierra que escarba.
    if e.gesto > 0.0 {
        for i in 0..5 {
            let r = 8.0 + 4.0 * i as f32 + 10.0 * (1.0 - e.gesto);
            let px = x + d * (30.0 + 8.0 * i as f32);
            circulo(l, px, pies - r * 0.6, r, fade(ALBERO, 0.6 * e.gesto));
        }
    }
    if escarba > 6.0 {
        circulo(l, x + d * 4.0, pies - 6.0, 4.0, fade(ALBERO, 0.7));
        circulo(l, x - d * 2.0, pies - 12.0, 3.0, fade(ALBERO, 0.5));
    }
}

/// La cantaora, sentada en su silla en medio del tablao, con el manton de
/// flecos, la peineta y la mano al pecho. Antes del acento coge aire y abre
/// la mano; en el acento la lanza hacia ti, con la boca abierta.
fn cantaora(e: &Enemigo, l: &Layout, tick: u64, tiempo: u32, tinte: &dyn Fn(Color) -> Color) {
    let (x, d, k) = (e.pos.x, e.dir, e.gesto);
    let pies = e.pos.y + 46.0;
    let aire = prepara(tick, tiempo, 12.0) * (1.0 - k);
    let (falda, manton, piel) = (tinte(NEGRO), tinte(CAL), tinte(PIEL));
    // La silla de enea.
    rect(l, x - d * 16.0 - 2.0, pies - 64.0, 5.0, 64.0, MADERA);
    rect(l, x - 16.0, pies - 26.0, 32.0, 5.0, MADERA_CLARA);
    linea(l, (x - 14.0, pies - 21.0), (x - 14.0, pies), 3.0, MADERA);
    linea(l, (x + 14.0, pies - 21.0), (x + 14.0, pies), 3.0, MADERA);
    let mano = if k > 0.0 {
        (x + d * (26.0 + 10.0 * k), pies - 72.0 - 6.0 * k)
    } else {
        (x + d * (10.0 + 6.0 * aire), pies - 62.0 - 10.0 * aire)
    };
    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        // La falda larga hasta el suelo, con volantes.
        tri(
            l,
            (x - 4.0, pies - 34.0 - g),
            (x - 22.0 - g, pies + g),
            (x + d * 30.0 + d * g, pies + g),
            c(falda),
        );
        rect(
            l,
            x - 10.0 - g,
            pies - 70.0 - g,
            20.0 + 2.0 * g,
            42.0 + g,
            c(falda),
        );
        // El manton sobre los hombros, en pico.
        tri(
            l,
            (x - 16.0 - g, pies - 70.0 - g),
            (x + 16.0 + g, pies - 70.0 - g),
            (x, pies - 36.0 + g),
            c(manton),
        );
        linea(l, (x + d * 6.0, pies - 64.0), mano, 4.5 + 2.0 * g, c(piel));
        circulo(l, mano.0, mano.1, 3.5 + g, c(piel));
        circulo(l, x, pies - 80.0, 8.5 + g, c(piel));
    });
    // Las flores bordadas y los flecos del manton.
    for (dx, dy) in [(-7.0, -62.0), (6.0, -58.0), (0.0, -48.0)] {
        circulo(l, x + dx, pies + dy, 2.5, tinte(ROJO));
    }
    let mut fx = x - 14.0;
    while fx < x + 15.0 {
        let fy = pies - 70.0 + (34.0 * (1.0 - (fx - x).abs() / 16.0)).max(0.0);
        linea(l, (fx, fy), (fx, fy + 7.0), 1.0, TINTA);
        fx += 3.0;
    }
    // El pelo con su mono, la peineta y la boca: abierta en el acento.
    arco(l, x, pies - 82.0, 8.5, 4.5, TINTA);
    circulo(l, x - d * 8.0, pies - 84.0, 6.0, TINTA);
    let s = l.to_screen(x - d * 4.0, pies - 94.0);
    draw_arc(s.x, s.y, 16, l.len(9.0), 180.0, l.len(3.5), 180.0, MADERA);
    circulo(l, x + d * 3.5, pies - 82.0, 1.4, TINTA);
    let boca = 1.2 + 2.6 * k.max(aire * 0.4);
    circulo(l, x + d * 5.0, pies - 76.0, boca, TINTA);
}

// ---------------------------------------------------------------------------
// Lo que vuela
// ---------------------------------------------------------------------------

/// Las notas de Triana: la falseta, dorada y latiendo, que va a quebrar; la
/// quebrada, con rayas hacia donde va; los ayes de la cantaora, ondas de voz;
/// y el taconeo, la tierra que levanta a ras de tablao.
pub fn proyectil(q: &Proyectil, l: &Layout, t: f32) {
    let (x, y) = (q.pos.x, q.pos.y);
    let v = hacia(q.vel.x, q.vel.y);
    match q.forma {
        Forma::Falseta | Forma::Quebrada => {
            let c = if q.rosa { ROSA } else { AMARILLO };
            if q.forma == Forma::Quebrada {
                for (i, dy) in [-4.0, 0.0, 4.0].into_iter().enumerate() {
                    let largo = 14.0 + 6.0 * i as f32;
                    let (a, b) = (
                        (x - v.x * 10.0 - v.y * dy, y - v.y * 10.0 + v.x * dy),
                        (
                            x - v.x * (10.0 + largo) - v.y * dy,
                            y - v.y * (10.0 + largo) + v.x * dy,
                        ),
                    );
                    linea(l, a, b, 1.8, fade(HUESO, 0.7));
                }
            } else {
                let k = 0.5 + 0.5 * (t * 0.3).sin();
                circulo(l, x, y, 12.0 + 2.0 * k, fade(c, 0.18));
            }
            con_tinta(|g, es_tinta| {
                let c = if es_tinta { TINTA } else { c };
                let p = l.to_screen(x - 2.0, y + 4.0);
                draw_ellipse(p.x, p.y, l.len(6.5 + g), l.len(5.0 + g), -22.0, c);
                linea(l, (x + 3.5, y + 3.0), (x + 3.5, y - 13.0), 2.4 + 2.0 * g, c);
                linea(
                    l,
                    (x + 3.5, y - 13.0),
                    (x + 10.0, y - 7.0),
                    2.4 + 2.0 * g,
                    c,
                );
            });
        }
        Forma::Quejio => {
            // Una onda de voz: el punto y dos arcos que se abren delante.
            let c = if q.rosa { ROSA } else { CAL };
            let giro = atan2(v.y, v.x).to_degrees();
            let s = l.to_screen(x, y);
            for (r, g) in [(9.0, 4.5), (14.0, 3.0)] {
                draw_arc(
                    s.x,
                    s.y,
                    16,
                    l.len(r),
                    giro - 50.0,
                    l.len(g + 2.2),
                    100.0,
                    TINTA,
                );
                draw_arc(s.x, s.y, 16, l.len(r), giro - 50.0, l.len(g), 100.0, c);
            }
            circulo(l, x, y, 5.5, TINTA);
            circulo(l, x, y, 4.0, c);
        }
        _ => {
            // El taconeo: tierra de albero saltando a ras de suelo.
            let dx = v.x.signum();
            for i in 0..3 {
                let px = x - dx * i as f32 * 9.0;
                let alto = 16.0 - 4.0 * i as f32 + 3.0 * (t * 0.5 + i as f32).sin();
                tri(
                    l,
                    (px - 7.0, y + 12.0),
                    (px + 7.0, y + 12.0),
                    (px + dx * 3.0, y + 12.0 - alto - 2.0),
                    TINTA,
                );
                tri(
                    l,
                    (px - 5.0, y + 11.0),
                    (px + 5.0, y + 11.0),
                    (px + dx * 3.0, y + 12.0 - alto),
                    ALBERO,
                );
            }
        }
    }
    if q.rosa {
        anillo_rosa(l, x, y, 16.0, t);
    }
}

/// Los golpes de palmas: un estallido quieto alrededor de las manos, del
/// tamano exacto de lo que pega, que se apaga en lo que dura.
pub fn palmadas(p: &Paseo, l: &Layout) {
    for f in p.fugaces.iter().filter(|f| f.clase == Clase::Palmada) {
        let k = (f.ttl / VIDA_PALMADA).clamp(0.0, 1.0);
        let r = Clase::Palmada.radio();
        let s = l.to_screen(f.pos.x, f.pos.y);
        draw_circle(s.x, s.y, l.len(r), fade(LUZ, 0.18 * k));
        draw_circle_lines(s.x, s.y, l.len(r), l.len(4.5), fade(TINTA, 0.8 * k));
        draw_circle_lines(s.x, s.y, l.len(r), l.len(2.5), fade(AMARILLO, k));
        for i in 0..10 {
            let (sn, co) = sin_cos(TAU * i as f32 / 10.0);
            let (r0, r1) = (r * 0.35, r * (0.55 + 0.3 * (1.0 - k)));
            linea(
                l,
                (f.pos.x + co * r0, f.pos.y + sn * r0),
                (f.pos.x + co * r1, f.pos.y + sn * r1),
                2.5,
                fade(HUESO, k),
            );
        }
    }
}
