//! Chicago: la calle que se cruza antes del charleston.
//!
//! Es el club de `escenarios::club` sacado a la ciudad: la noche verde botella
//! del decorado, el oro de su sol Art Deco y las bombillas que corren. Chicago
//! en los anos veinte es eso visto desde fuera: rascacielos escalonados con la
//! corona dorada, el tren elevado sobre su caballete de acero, marquesinas y
//! escaleras de incendios, y el jazz saliendo por una puerta de sotano.
//!
//! Viena era una calle; esto se cruza **por arriba**. El dibujo no recibe mas
//! datos que Viena —tramos de acera y plataformas— y decide que es cada
//! plataforma por donde esta (`Pieza`): sobre la acera, un rellano de escalera
//! de incendios; sobre el vacio, una azotea con todo su edificio debajo o las
//! vias del elevado; encima de otra, el deposito de agua de su tejado. Asi el
//! RON sigue siendo el mismo formato y el nivel se afina moviendo numeros.
//!
//! **El vacio tiene que leerse como caida.** Entre dos tejados no hay canal,
//! hay un callejon que se oscurece hacia abajo y, al fondo del todo, las luces
//! diminutas de la calle de verdad. Diminutas a proposito: el tamano es lo que
//! dice que esta lejos.
//!
//! Las capas son las de Viena: cielo quieto con dos reflectores que barren (el
//! verbo del charleston es girar), la silueta de la ciudad al 15 %, las torres
//! Art Deco al 30 %, los edificios de ladrillo al 50 % y la calle al 100 %.
//! Todo late con el pulso del rag, y lo que baila, baila en la clave 3-3-2.

use macroquad::prelude::*;
use vals_core::math::{PI, TAU, sin_cos};
use vals_core::paseo::{
    self, EN_EL_SUELO, Enemigo, Forma, Paseo, Proyectil, SUELO_Y, Tipo, VISTA_H, VISTA_W,
};
use vals_core::rng::Pcg32;

use crate::calle::{HUESO, ROSA, camara, con_tinta};
use crate::draw::{Layout, fade};
use crate::escenarios::{Instrumento, arco, circulo, linea, musico, rect, tri};
use crate::fuentes::{self, Cara};
use crate::idioma::{Texto, nombre, tx};
use crate::paleta::{LUZ, ORO, TINTA};
use crate::skeleton;

// La noche del club: verde botella, no azul de Prusia. Y un horizonte que
// clarea hacia el ambar, que es lo que hace una ciudad con luz electrica.
const CIELO: [Color; 5] = [
    color_u8!(10, 20, 24, 255),
    color_u8!(13, 28, 31, 255),
    color_u8!(18, 38, 40, 255),
    color_u8!(28, 52, 50, 255),
    color_u8!(52, 70, 56, 255),
];
const LUNA: Color = color_u8!(236, 222, 170, 255);
/// La ciudad lejana, una sola tinta plana y mas oscura que el cielo.
const LEJOS: Color = color_u8!(11, 23, 26, 255);
/// Las torres Art Deco del medio.
const DECO: Color = color_u8!(19, 36, 37, 255);
const DECO_CLARO: Color = color_u8!(30, 52, 50, 255);
/// El ladrillo de las fachadas de fondo: rojizo, pero hundido en la noche.
const FONDO_LADRILLO: Color = color_u8!(40, 34, 36, 255);
/// El ladrillo de lo que esta en la calle y se pisa. Mas vivo que el del fondo:
/// lo cercano tiene que despegarse de lo lejano sin gritar.
const LADRILLO: Color = color_u8!(96, 52, 44, 255);
const LADRILLO_OSCURO: Color = color_u8!(66, 38, 34, 255);
/// La piedra de las cornisas. Es lo que se pisa en las azoteas, y por eso es
/// clara: una plataforma es informacion (lo mismo que el balcon de Viena).
const PIEDRA: Color = color_u8!(206, 190, 148, 255);
const VENTANA: Color = color_u8!(236, 188, 104, 255);
const APAGADA: Color = color_u8!(24, 44, 46, 255);
const HIERRO: Color = color_u8!(34, 40, 40, 255);
/// El verde de las vigas del elevado, el mismo de los puentes de Chicago.
const ACERO: Color = color_u8!(58, 86, 76, 255);
const ACERO_CLARO: Color = color_u8!(92, 122, 108, 255);
const RAIL: Color = color_u8!(214, 206, 180, 255);
const MADERA: Color = color_u8!(112, 72, 46, 255);
const HORMIGON: Color = color_u8!(158, 150, 128, 255);
const ASFALTO: Color = color_u8!(34, 32, 36, 255);
const ADOQUIN: Color = color_u8!(56, 42, 42, 255);
/// El fondo del vacio: casi negro, un punto verde.
const VACIO: Color = color_u8!(5, 11, 13, 255);
const PIEL: Color = color_u8!(240, 208, 176, 255);
const PAJA: Color = color_u8!(236, 214, 150, 255);
const PERLA: Color = color_u8!(246, 240, 224, 255);
const ROJO: Color = color_u8!(214, 70, 60, 255);

/// Lo que es cada plataforma, deducido de donde esta.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pieza {
    /// Sobre la acera: un rellano de escalera de incendios.
    Rellano,
    /// Sobre el vacio y alta: el tejado de un edificio que baja hasta abajo.
    Azotea,
    /// Sobre el vacio y baja: las vias del elevado.
    Via,
    /// Encima de otra plataforma y sobre el vacio: un deposito de agua.
    Deposito,
    /// La que se mueve: un vagon del tren, y lo que se pisa es su techo.
    Vagon,
}

fn pieza(p: &Paseo, i: usize) -> Pieza {
    let d = &p.def;
    let pl = d.plataformas[i];
    if pl.mueve.is_some() {
        return Pieza::Vagon;
    }
    let cx = pl.x + pl.ancho * 0.5;
    if d.hay_suelo(cx) {
        return Pieza::Rellano;
    }
    if tejado_debajo(p, i).is_some() {
        Pieza::Deposito
    } else if pl.y < 320.0 {
        Pieza::Azotea
    } else {
        Pieza::Via
    }
}

/// La superficie de la plataforma que tiene `i` debajo, si la tapa entera.
fn tejado_debajo(p: &Paseo, i: usize) -> Option<f32> {
    let pl = p.def.plataformas[i];
    p.def
        .plataformas
        .iter()
        .filter(|o| o.y > pl.y + 10.0 && o.y - pl.y < 200.0)
        .find(|o| o.x <= pl.x && o.x + o.ancho >= pl.x + pl.ancho)
        .map(|o| o.y)
}

/// Un rectangulo con su contorno de tinta.
fn caja(l: &Layout, x: f32, y: f32, w: f32, h: f32, c: Color, g: f32) {
    rect(l, x - g, y - g, w + 2.0 * g, h + 2.0 * g, TINTA);
    rect(l, x, y, w, h, c);
}

/// Una fila de bombillas que corren: una de cada tres encendida, y el patron
/// avanza solo, como en la marquesina del club.
fn bombillas(l: &Layout, a: (f32, f32), b: (f32, f32), n: usize, t: f32, r: f32) {
    let paso = (t / 9.0) as usize;
    for i in 0..n {
        let k = if n > 1 {
            i as f32 / (n - 1) as f32
        } else {
            0.5
        };
        let (x, y) = (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
        let encendida = (i + paso).is_multiple_of(3);
        let c = if encendida {
            color_u8!(255, 236, 170, 255)
        } else {
            color_u8!(110, 86, 44, 255)
        };
        if encendida {
            circulo(l, x, y, r * 2.2, fade(LUZ, 0.18));
        }
        circulo(l, x, y, r + 1.4, TINTA);
        circulo(l, x, y, r, c);
    }
}

/// Texto centrado en coordenadas de la calle.
fn rotulo(l: &Layout, s: &str, cx: f32, y: f32, tam: f32, c: Color) {
    let p = l.to_screen(cx, y);
    fuentes::centrado(s, p.x, p.y, l.len(tam), Cara::Titulo, c);
}

// ---------------------------------------------------------------------------
// El fondo
// ---------------------------------------------------------------------------

/// El fondo de Chicago, del cielo a lo que hay pegado a la acera.
pub fn fondo(p: &Paseo, l: &Layout, capa: &dyn Fn(f32) -> Layout, cam: f32, pulso: f32) {
    let t = p.tick as f32;
    cielo(l, t, pulso);
    silueta(&capa(0.15), cam * 0.15, pulso);
    torres(&capa(0.30), cam * 0.30, t, pulso);
    ladrillo(&capa(0.50), cam * 0.50, pulso);
    let calle = capa(1.0);
    vacio(p, &calle, cam, t);
    pegado_a_la_acera(p, &calle, cam, t, pulso);
}

/// El cielo, la luna y los reflectores. Los reflectores son de estreno de
/// cine, y giran: es el verbo del charleston puesto en el cielo.
fn cielo(l: &Layout, t: f32, pulso: f32) {
    let alto = VISTA_H / CIELO.len() as f32;
    for (i, c) in CIELO.into_iter().enumerate() {
        rect(l, 0.0, alto * i as f32, VISTA_W, alto + 1.0, c);
    }
    let mut rng = Pcg32::new(0x0C41_CA60);
    for _ in 0..26 {
        let (x, y) = (rng.next_f32() * VISTA_W, rng.next_f32() * 200.0);
        let brillo = 0.3 + 0.5 * rng.next_f32();
        circulo(l, x, y, 1.1, fade(HUESO, brillo * (0.6 + 0.4 * pulso)));
    }
    circulo(l, 170.0, 92.0, 52.0, fade(LUNA, 0.07));
    circulo(l, 170.0, 92.0, 32.0, LUNA);
    // La luna de un cartel: una media sombra, no crateres.
    circulo(
        l,
        182.0,
        86.0,
        27.0,
        fade(color_u8!(210, 190, 132, 255), 0.5),
    );

    for (k, bx) in [(0.0_f32, 300.0_f32), (1.0, 700.0)] {
        let a = -PI * 0.5 + 0.5 * (t * 0.011 + k * 2.3).sin();
        let (s0, c0) = sin_cos(a - 0.035);
        let (s1, c1) = sin_cos(a + 0.035);
        let (x0, y0) = (bx, 330.0);
        let r = 700.0;
        tri(
            l,
            (x0, y0),
            (x0 + c0 * r, y0 + s0 * r),
            (x0 + c1 * r, y0 + s1 * r),
            fade(LUZ, 0.07 + 0.03 * pulso),
        );
    }
}

/// Un rascacielos escalonado: cada cuerpo, mas estrecho que el de debajo.
/// Devuelve donde acaba por arriba.
fn escalonado(
    l: &Layout,
    cx: f32,
    base: f32,
    ancho: f32,
    alto: f32,
    cuerpos: usize,
    c: Color,
) -> f32 {
    let h = alto / cuerpos as f32;
    let (mut w, mut y) = (ancho, base);
    for i in 0..cuerpos {
        // El de abajo, mas alto: los retranqueos de la ley de 1923 empiezan
        // arriba, no en la acera.
        let hi = if i == 0 { h * 1.6 } else { h * 0.7 };
        rect(l, cx - w * 0.5, y - hi, w, hi + 1.0, c);
        y -= hi;
        w *= 0.74;
    }
    y
}

/// Un sol Art Deco: rayos alternos en abanico, la corona de medio Chicago.
fn sol(l: &Layout, cx: f32, cy: f32, r: f32, a: Color, b: Color) {
    let rayos = 10;
    for i in 0..rayos {
        let a0 = PI + PI * i as f32 / rayos as f32;
        let a1 = PI + PI * (i + 1) as f32 / rayos as f32;
        let (s0, c0) = sin_cos(a0);
        let (s1, c1) = sin_cos(a1);
        tri(
            l,
            (cx, cy),
            (cx + c0 * r, cy + s0 * r),
            (cx + c1 * r, cy + s1 * r),
            if i % 2 == 0 { a } else { b },
        );
    }
}

/// Chicago a lo lejos, en una tinta: bloques bajos y tres torres que la dicen
/// sin escribirla. La del Carbide & Carbon, verde oscuro con la corona de pan
/// de oro, que son justo las dos tintas del charleston; la Tribune, con su
/// corona gotica de contrafuertes; y la Wrigley, con su reloj.
fn silueta(l: &Layout, desde: f32, pulso: f32) {
    const PERIODO: f32 = 1800.0;
    let base = 330.0;
    let primera = (desde / PERIODO).floor() as i32;
    for k in primera..=primera + 1 {
        let x0 = k as f32 * PERIODO;
        let mut rng = Pcg32::new(0xC4_1CA6 + k as u64);
        let mut x = x0;
        while x < x0 + PERIODO {
            let w = 50.0 + rng.next_f32() * 80.0;
            let h = 40.0 + rng.next_f32() * 90.0;
            rect(l, x, base - h, w + 1.0, h + 120.0, LEJOS);
            for _ in 0..3 {
                if rng.next_f32() < 0.6 {
                    let vx = x + 6.0 + rng.next_f32() * (w - 12.0);
                    let vy = base - h + 8.0 + rng.next_f32() * (h - 10.0);
                    rect(l, vx, vy, 3.0, 4.0, fade(VENTANA, 0.45));
                }
            }
            x += w;
        }
        // El Carbide & Carbon: esbelto, escalonado y con la aguja de oro.
        let cx = x0 + 330.0;
        let arriba = escalonado(l, cx, base, 80.0, 250.0, 4, LEJOS);
        let oro = fade(ORO, 0.8 + 0.2 * pulso);
        rect(l, cx - 10.0, arriba - 10.0, 20.0, 10.0, oro);
        tri(
            l,
            (cx - 6.0, arriba - 10.0),
            (cx, arriba - 54.0),
            (cx + 6.0, arriba - 10.0),
            oro,
        );
        circulo(l, cx, arriba - 16.0, 16.0, fade(ORO, 0.05 + 0.04 * pulso));
        // La Tribune: torre y corona con contrafuertes.
        let tx = x0 + 860.0;
        rect(l, tx - 36.0, base - 210.0, 72.0, 210.0, LEJOS);
        rect(l, tx - 26.0, base - 250.0, 52.0, 40.0, LEJOS);
        for i in 0..5 {
            let dx = -26.0 + 13.0 * i as f32;
            tri(
                l,
                (dx + tx - 4.0, base - 250.0),
                (dx + tx, base - 276.0),
                (dx + tx + 4.0, base - 250.0),
                LEJOS,
            );
        }
        // La Wrigley: cuerpo, torre del reloj y su templete.
        let wx = x0 + 1380.0;
        rect(l, wx - 60.0, base - 150.0, 120.0, 150.0, LEJOS);
        rect(l, wx - 22.0, base - 220.0, 44.0, 70.0, LEJOS);
        rect(l, wx - 14.0, base - 250.0, 28.0, 30.0, LEJOS);
        tri(
            l,
            (wx - 10.0, base - 250.0),
            (wx, base - 282.0),
            (wx + 10.0, base - 250.0),
            LEJOS,
        );
        circulo(l, wx, base - 196.0, 11.0, fade(VENTANA, 0.55 + 0.2 * pulso));
        circulo(l, wx, base - 196.0, 11.0 * 0.15, LEJOS);
    }
}

/// Las torres Art Deco del medio: escalonadas, con franjas de ventanas en
/// vertical, un sol de oro en la corona y la luz roja de aviso latiendo.
fn torres(l: &Layout, desde: f32, t: f32, pulso: f32) {
    const PASO: f32 = 420.0;
    let primero = (desde / PASO).floor() as i32 - 1;
    for k in primero..primero + (VISTA_W / PASO) as i32 + 3 {
        let mut rng = Pcg32::new(0xDEC0 + k as u64);
        let cx = k as f32 * PASO + 80.0 + rng.next_f32() * 220.0;
        let ancho = 90.0 + rng.next_f32() * 60.0;
        let alto = 250.0 + rng.next_f32() * 110.0;
        let arriba = escalonado(l, cx, SUELO_Y, ancho, alto, 3, DECO);
        // Las pilastras: la verticalidad es todo el estilo.
        let hombro = SUELO_Y - alto / 3.0 * 1.6;
        for i in 0..4 {
            let x = cx - ancho * 0.5 + ancho * (i as f32 + 0.5) / 4.0;
            linea(l, (x, hombro + 6.0), (x, SUELO_Y), 2.0, fade(TINTA, 0.5));
            let mut y = hombro + 14.0;
            while y < SUELO_Y - 20.0 {
                if rng.next_f32() < 0.35 {
                    rect(l, x - 8.0, y, 5.0, 7.0, fade(VENTANA, 0.35 + 0.25 * pulso));
                }
                y += 18.0;
            }
        }
        match k.rem_euclid(3) {
            0 => {
                sol(
                    l,
                    cx,
                    arriba,
                    26.0,
                    fade(color_u8!(186, 142, 58, 255), 0.8),
                    fade(color_u8!(132, 98, 40, 255), 0.8),
                );
            }
            1 => {
                tri(
                    l,
                    (cx - 8.0, arriba),
                    (cx, arriba - 56.0),
                    (cx + 8.0, arriba),
                    DECO_CLARO,
                );
            }
            _ => {
                rect(l, cx - 10.0, arriba - 22.0, 20.0, 22.0, DECO_CLARO);
                arco(l, cx, arriba - 22.0, 10.0, 3.0, DECO_CLARO);
            }
        }
        // La luz de aviso, que parpadea a su aire, no con el compas: una
        // ciudad no baila entera.
        let on = ((t / 40.0) as i32 + k).rem_euclid(3) != 0;
        if on {
            circulo(l, cx, arriba - 4.0, 7.0, fade(ROJO, 0.2));
            circulo(l, cx, arriba - 4.0, 2.6, ROJO);
        }
    }
}

/// Los edificios de ladrillo de detras de la calle: cornisa, ventanas y en
/// algunos tejados el deposito de agua, que es la silueta de Chicago de cerca.
/// Alguno lleva un rotulo vertical con bombillas.
fn ladrillo(l: &Layout, desde: f32, pulso: f32) {
    const VANO: f32 = 230.0;
    let primero = (desde / VANO).floor() as i32 - 1;
    for k in primero..primero + (VISTA_W / VANO) as i32 + 3 {
        let x0 = k as f32 * VANO;
        let mut rng = Pcg32::new(0x1AD_2111 + k as u64);
        let techo = 200.0 + rng.next_f32() * 90.0;
        let w = VANO - 18.0 - rng.next_f32() * 30.0;
        rect(l, x0, techo, w, SUELO_Y - techo, FONDO_LADRILLO);
        rect(l, x0 - 4.0, techo - 8.0, w + 8.0, 8.0, DECO_CLARO);
        linea(
            l,
            (x0 - 4.0, techo - 8.0),
            (x0 + w + 4.0, techo - 8.0),
            2.0,
            TINTA,
        );
        linea(l, (x0, techo), (x0, SUELO_Y), 2.0, fade(TINTA, 0.7));
        linea(l, (x0 + w, techo), (x0 + w, SUELO_Y), 2.0, fade(TINTA, 0.7));
        let mut y = techo + 18.0;
        while y < SUELO_Y - 30.0 {
            let mut x = x0 + 14.0;
            while x + 18.0 < x0 + w - 8.0 {
                let c = if rng.next_f32() < 0.3 {
                    fade(VENTANA, 0.55 + 0.3 * pulso)
                } else {
                    APAGADA
                };
                rect(l, x, y, 16.0, 22.0, c);
                x += 30.0;
            }
            y += 40.0;
        }
        // El deposito de agua: barril sobre patas y sombrero de cono.
        if rng.next_f32() < 0.5 {
            let dx = x0 + 30.0 + rng.next_f32() * (w - 80.0);
            let (ta, tb) = (techo - 70.0, techo - 30.0);
            linea(l, (dx + 4.0, tb), (dx, techo), 2.5, DECO);
            linea(l, (dx + 36.0, tb), (dx + 40.0, techo), 2.5, DECO);
            linea(l, (dx + 4.0, tb), (dx + 36.0, techo), 1.5, DECO);
            rect(l, dx, ta, 40.0, 40.0, DECO);
            tri(
                l,
                (dx - 4.0, ta),
                (dx + 20.0, ta - 18.0),
                (dx + 44.0, ta),
                DECO,
            );
        }
        // Un rotulo vertical, uno de cada cuatro.
        if k.rem_euclid(4) == 1 {
            let (sx, sy) = (x0 + w - 6.0, techo + 30.0);
            caja(l, sx, sy, 22.0, 110.0, color_u8!(46, 64, 56, 255), 2.0);
            let letras = ["H", "O", "T", "E", "L"];
            for (i, c) in letras.iter().enumerate() {
                rotulo(
                    l,
                    c,
                    sx + 11.0,
                    sy + 22.0 + i as f32 * 20.0,
                    16.0,
                    fade(VENTANA, 0.6 + 0.4 * pulso),
                );
            }
        }
    }
}

/// El vacio entre los tramos de acera: un callejon que se oscurece hacia
/// abajo y, al fondo, las luces diminutas de la calle de verdad. Es lo que
/// dice "caida" sin agua ni pinchos.
fn vacio(p: &Paseo, l: &Layout, cam: f32, t: f32) {
    let fondo = VISTA_H + 10.0;
    let (izq, der) = (cam - 20.0, cam + VISTA_W + 20.0);
    let mut huecos = Vec::with_capacity(4);
    let mut desde = f32::MIN;
    for &(a, b) in &p.def.suelo {
        huecos.push((desde, a));
        desde = b;
    }
    huecos.push((desde, f32::MAX));
    for (a, b) in huecos {
        let (a, b) = (a.max(izq), b.min(der));
        if b <= a {
            continue;
        }
        // El callejon: capas que se van oscureciendo hacia abajo.
        for k in 0..10 {
            let y = 220.0 + k as f32 * 28.0;
            rect(l, a, y, b - a, fondo - y, fade(VACIO, 0.2));
        }
        rect(l, a, SUELO_Y + 10.0, b - a, fondo - SUELO_Y, VACIO);
        // Las farolas de abajo, en una fila, y algun coche con los faros.
        let mut x = (a / 64.0).floor() * 64.0;
        while x < b {
            let n = (x / 64.0) as i64;
            if n % 3 != 0 {
                circulo(l, x, 514.0, 1.6, fade(VENTANA, 0.8));
                circulo(l, x, 514.0, 4.0, fade(VENTANA, 0.12));
            }
            x += 64.0;
        }
        for i in 0..3 {
            let cx = a + ((t * (0.6 + 0.3 * i as f32) + 311.0 * i as f32) % (b - a).max(1.0));
            let y = 526.0 - i as f32 * 3.0;
            circulo(l, cx, y, 1.3, fade(LUZ, 0.9));
            circulo(l, cx + 4.0, y, 1.3, fade(LUZ, 0.9));
        }
    }
}

// ---------------------------------------------------------------------------
// Lo que esta pegado a la acera
// ---------------------------------------------------------------------------

/// Lo que va en el plano de la calle pero detras de la jugadora: el teatro del
/// principio, la puerta del sotano con la banda, los bloques de las escaleras
/// de incendios, farolas y algun Ford aparcado.
fn pegado_a_la_acera(p: &Paseo, l: &Layout, cam: f32, t: f32, pulso: f32) {
    let ve = |a: f32, b: f32| b > cam - 40.0 && a < cam + VISTA_W + 40.0;
    if ve(280.0, 800.0) {
        teatro(l, 300.0, t, pulso);
    }
    if ve(840.0, 1110.0) {
        sotano(l, 860.0, t, pulso);
    }
    for (a, b, arriba) in bloques(p) {
        if ve(a, b) {
            bloque(l, a, b, arriba, pulso);
        }
    }
    for x in [1440.0, 6520.0] {
        if ve(x - 60.0, x + 60.0) && p.def.hay_suelo(x) {
            ford(l, x, pulso);
        }
    }
    farolas(p, l, cam, pulso);
}

/// Los bloques de detras de cada escalera de incendios: uno por grupo de
/// rellanos que se tocan, del mas alto a la acera.
fn bloques(p: &Paseo) -> Vec<(f32, f32, f32)> {
    let mut v: Vec<(f32, f32, f32)> = Vec::new();
    for i in 0..p.def.plataformas.len() {
        if pieza(p, i) != Pieza::Rellano {
            continue;
        }
        let pl = p.def.plataformas[i];
        let (a, b) = (pl.x - 40.0, pl.x + pl.ancho + 40.0);
        match v.iter_mut().find(|g| a < g.1 + 60.0 && b > g.0 - 60.0) {
            Some(g) => *g = (g.0.min(a), g.1.max(b), g.2.min(pl.y)),
            None => v.push((a, b, pl.y)),
        }
    }
    v
}

/// Un bloque de viviendas de ladrillo, con la cornisa arriba y una ventana
/// encendida detras de cada rellano.
fn bloque(l: &Layout, a: f32, b: f32, arriba: f32, pulso: f32) {
    let techo = arriba - 90.0;
    rect(l, a, techo, b - a, SUELO_Y - techo, LADRILLO_OSCURO);
    // Hiladas de ladrillo, finas: textura, no dibujo.
    let mut y = techo + 14.0;
    while y < SUELO_Y {
        linea(l, (a, y), (b, y), 1.0, fade(TINTA, 0.25));
        y += 14.0;
    }
    let mut y = techo + 30.0;
    let mut n = 0;
    while y < SUELO_Y - 40.0 {
        let mut x = a + 22.0;
        while x + 30.0 < b - 10.0 {
            n += 1;
            let c = if n % 3 != 0 {
                fade(VENTANA, 0.75 + 0.25 * pulso)
            } else {
                APAGADA
            };
            caja(l, x, y, 28.0, 40.0, c, 2.0);
            linea(l, (x, y + 18.0), (x + 28.0, y + 18.0), 2.0, TINTA);
            x += 58.0;
        }
        y += 70.0;
    }
    caja(l, a - 8.0, techo - 12.0, b - a + 16.0, 12.0, PIEDRA, 2.5);
    linea(l, (a, techo), (a, SUELO_Y), 3.0, TINTA);
    linea(l, (b, techo), (b, SUELO_Y), 3.0, TINTA);
}

/// El teatro del principio: fachada de ladrillo, rotulo vertical de JAZZ con
/// bombillas y la marquesina con el cartel de esta noche. Es lo primero que se
/// ve, y dice donde estas antes de que pase nada.
fn teatro(l: &Layout, x: f32, t: f32, pulso: f32) {
    let (w, techo) = (480.0, 110.0);
    rect(l, x, techo, w, SUELO_Y - techo, LADRILLO);
    // Pilastras Deco en oro viejo.
    for i in 0..5 {
        let px = x + 20.0 + i as f32 * (w - 40.0) / 4.0;
        rect(l, px - 6.0, techo, 12.0, 220.0, LADRILLO_OSCURO);
        linea(l, (px - 6.0, techo), (px - 6.0, techo + 220.0), 1.5, TINTA);
        linea(l, (px + 6.0, techo), (px + 6.0, techo + 220.0), 1.5, TINTA);
    }
    for i in 0..4 {
        let vx = x + 50.0 + i as f32 * (w - 40.0) / 4.0;
        caja(
            l,
            vx,
            techo + 50.0,
            50.0,
            90.0,
            fade(VENTANA, 0.7 + 0.3 * pulso),
            2.5,
        );
        linea(
            l,
            (vx + 25.0, techo + 50.0),
            (vx + 25.0, techo + 140.0),
            2.0,
            TINTA,
        );
    }
    // El remate: un sol a lo ancho, como el del club.
    sol(
        l,
        x + w * 0.5,
        techo,
        70.0,
        color_u8!(196, 150, 62, 255),
        color_u8!(140, 104, 44, 255),
    );
    arco(l, x + w * 0.5, techo, 70.0, 3.0, TINTA);
    caja(l, x - 6.0, techo - 4.0, w + 12.0, 10.0, PIEDRA, 2.5);

    // El rotulo vertical, saliendo de la fachada.
    let (rx, ry) = (x + 40.0, techo + 10.0);
    caja(l, rx, ry, 44.0, 190.0, color_u8!(58, 86, 70, 255), 3.0);
    for (i, c) in ["J", "A", "Z", "Z"].iter().enumerate() {
        rotulo(
            l,
            c,
            rx + 22.0,
            ry + 44.0 + i as f32 * 42.0,
            34.0,
            fade(color_u8!(255, 236, 170, 255), 0.65 + 0.35 * pulso),
        );
    }
    bombillas(l, (rx - 2.0, ry + 4.0), (rx - 2.0, ry + 186.0), 12, t, 2.6);
    bombillas(
        l,
        (rx + 46.0, ry + 4.0),
        (rx + 46.0, ry + 186.0),
        12,
        t + 9.0,
        2.6,
    );

    // La marquesina: el cartel en la cara, bombillas arriba y abajo.
    let (mx, my, mw, mh) = (x + 110.0, 300.0, 300.0, 50.0);
    caja(l, mx, my, mw, mh, color_u8!(246, 232, 196, 255), 3.0);
    rotulo(
        l,
        tx(Texto::EstaNoche),
        mx + mw * 0.5,
        my + 21.0,
        15.0,
        TINTA,
    );
    rotulo(l, "CHARLESTON", mx + mw * 0.5, my + 43.0, 20.0, ROJO);
    bombillas(l, (mx, my - 6.0), (mx + mw, my - 6.0), 22, t, 3.0);
    bombillas(
        l,
        (mx, my + mh + 6.0),
        (mx + mw, my + mh + 6.0),
        22,
        t + 18.0,
        3.0,
    );
    // Las puertas, con la luz de dentro.
    for i in 0..3 {
        let dx = mx + 40.0 + i as f32 * 80.0;
        caja(
            l,
            dx,
            380.0,
            60.0,
            SUELO_Y - 380.0,
            fade(LUZ, 0.55 + 0.25 * pulso),
            3.0,
        );
        linea(l, (dx + 30.0, 380.0), (dx + 30.0, SUELO_Y), 2.0, TINTA);
    }
    linea(l, (x, techo), (x, SUELO_Y), 3.5, TINTA);
    linea(l, (x + w, techo), (x + w, SUELO_Y), 3.5, TINTA);
}

/// La puerta del sotano: unos escalones que bajan, la puerta abierta y dentro
/// la banda tocando. El jazz se sale a la calle, y de aqui salen las notas.
fn sotano(l: &Layout, x: f32, t: f32, pulso: f32) {
    let (w, techo) = (230.0, 230.0);
    rect(l, x, techo, w, SUELO_Y - techo, LADRILLO_OSCURO);
    let mut y = techo + 14.0;
    while y < SUELO_Y {
        linea(l, (x, y), (x + w, y), 1.0, fade(TINTA, 0.25));
        y += 14.0;
    }
    caja(l, x - 6.0, techo - 10.0, w + 12.0, 10.0, PIEDRA, 2.5);
    // Un toldo a rayas, en las dos tintas del charleston.
    let (ta, tb) = (x + 30.0, x + 200.0);
    for i in 0..8 {
        let a = ta + (tb - ta) * i as f32 / 8.0;
        let c = if i % 2 == 0 {
            color_u8!(226, 178, 66, 255)
        } else {
            color_u8!(78, 116, 92, 255)
        };
        tri(l, (a, 330.0), (a + 21.25, 330.0), (a + 30.0, 356.0), c);
        tri(l, (a, 330.0), (a + 30.0, 356.0), (a + 8.75, 356.0), c);
    }
    linea(l, (ta, 330.0), (tb, 330.0), 3.0, TINTA);
    linea(l, (ta + 8.0, 356.0), (tb + 8.0, 356.0), 3.0, TINTA);
    // La puerta, abierta: luz, y la banda en contraluz.
    let (px, pw) = (x + 60.0, 110.0);
    rect(
        l,
        px,
        364.0,
        pw,
        SUELO_Y - 364.0,
        fade(LUZ, 0.85 + 0.15 * pulso),
    );
    musico(l, px + 24.0, SUELO_Y, Instrumento::Trompeta, pulso);
    musico(l, px + 74.0, SUELO_Y, Instrumento::Contrabajo, pulso);
    linea(l, (px, 364.0), (px, SUELO_Y), 4.0, TINTA);
    linea(l, (px + pw, 364.0), (px + pw, SUELO_Y), 4.0, TINTA);
    linea(l, (px, 364.0), (px + pw, 364.0), 4.0, TINTA);
    // Las notas que se escapan, subiendo con el compas.
    for i in 0..3 {
        let k = ((t / 60.0) + i as f32 / 3.0).fract();
        let nx = px + pw * 0.5 + 30.0 * k + 10.0 * (t * 0.05 + i as f32).sin();
        let ny = 350.0 - 90.0 * k;
        let c = fade(PAJA, 1.0 - k);
        circulo(l, nx, ny, 4.0, c);
        linea(l, (nx + 3.5, ny), (nx + 3.5, ny - 14.0), 1.5, c);
    }
}

/// Un Ford T aparcado, en tinta, con los faros encendidos.
fn ford(l: &Layout, x: f32, pulso: f32) {
    let y = SUELO_Y;
    rect(l, x - 50.0, y - 34.0, 100.0, 18.0, TINTA);
    rect(l, x - 30.0, y - 62.0, 52.0, 30.0, TINTA);
    rect(l, x - 24.0, y - 56.0, 18.0, 16.0, fade(VENTANA, 0.35));
    rect(l, x - 2.0, y - 56.0, 18.0, 16.0, fade(VENTANA, 0.35));
    for dx in [-30.0, 30.0] {
        circulo(l, x + dx, y - 12.0, 13.0, TINTA);
        circulo(l, x + dx, y - 12.0, 6.0, color_u8!(70, 60, 50, 255));
    }
    circulo(l, x + 52.0, y - 30.0, 5.0, TINTA);
    circulo(l, x + 52.0, y - 30.0, 3.4, fade(LUZ, 0.8 + 0.2 * pulso));
    tri(
        l,
        (x + 54.0, y - 30.0),
        (x + 150.0, y - 50.0),
        (x + 150.0, y - 6.0),
        fade(LUZ, 0.08),
    );
}

/// Las farolas de Chicago: poste de hierro y dos globos, con su charco de luz.
fn farolas(p: &Paseo, l: &Layout, cam: f32, pulso: f32) {
    const CADA: f32 = 460.0;
    let mut x = (cam / CADA).floor() * CADA + 230.0 - CADA;
    while x < cam + VISTA_W + CADA {
        // Ni delante del teatro y del sotano, ni debajo de algo que se pisa:
        // un globo asomando por encima de una via parece algo en la via.
        let libre = !(280.0..800.0).contains(&x)
            && !(840.0..1110.0).contains(&x)
            && !p
                .def
                .plataformas
                .iter()
                .any(|pl| x > pl.x - 40.0 && x < pl.x + pl.ancho + 40.0);
        if p.def.hay_suelo(x) && libre && x < p.def.puerta - 300.0 {
            let arriba = 330.0;
            tri(
                l,
                (x, arriba),
                (x - 80.0, SUELO_Y),
                (x + 80.0, SUELO_Y),
                fade(LUZ, 0.05 + 0.05 * pulso),
            );
            rect(l, x - 3.0, arriba, 6.0, SUELO_Y - arriba, TINTA);
            rect(l, x - 8.0, SUELO_Y - 16.0, 16.0, 16.0, TINTA);
            linea(
                l,
                (x - 22.0, arriba + 8.0),
                (x + 22.0, arriba + 8.0),
                3.0,
                TINTA,
            );
            for dx in [-22.0, 22.0, 0.0] {
                let gy = if dx == 0.0 {
                    arriba - 14.0
                } else {
                    arriba - 2.0
                };
                circulo(l, x + dx, gy, 9.0, TINTA);
                circulo(l, x + dx, gy, 7.0, fade(LUZ, 0.8 + 0.2 * pulso));
            }
        }
        x += CADA;
    }
}

// ---------------------------------------------------------------------------
// La calle y lo que se pisa
// ---------------------------------------------------------------------------

/// Acera, tejados, vias, escaleras y la puerta del club: todo lo que se pisa.
pub fn calle(p: &Paseo, l: &Layout, alpha: f32, cam: f32, t: f32, pulso: f32) {
    acera(p, l, cam, t);
    let n = p.def.plataformas.len();
    // La via del tren va por debajo de los vagones, una vez por hueco.
    let mut vias: Vec<(f32, f32)> = Vec::new();
    for i in 0..n {
        if pieza(p, i) == Pieza::Vagon {
            let h = hueco_del_tren(p, i);
            if !vias.contains(&h) {
                vias.push(h);
            }
        }
    }
    for (a, b) in vias {
        if b > cam - 40.0 && a < cam + VISTA_W + 40.0 {
            via_del_tren(l, a, b, pulso);
        }
    }
    // Primero los edificios, luego lo que va encima de ellos.
    for fase in [
        Pieza::Azotea,
        Pieza::Via,
        Pieza::Vagon,
        Pieza::Deposito,
        Pieza::Rellano,
    ] {
        for i in 0..n {
            let pl = p.def.plataformas[i];
            if pl.x > cam + VISTA_W + 40.0 || pl.x + pl.ancho < cam - 40.0 || pieza(p, i) != fase {
                continue;
            }
            match fase {
                Pieza::Azotea => azotea(l, pl.x, pl.y, pl.ancho, t, pulso),
                Pieza::Via => via(l, pl.x, pl.y, pl.ancho, pulso),
                Pieza::Deposito => {
                    let abajo = tejado_debajo(p, i).unwrap_or(pl.y + 80.0);
                    deposito(l, pl.x, pl.y, pl.ancho, abajo);
                }
                Pieza::Rellano => rellano(p, l, i),
                Pieza::Vagon => vagon(l, pl.x, pl.y, pl.ancho, t, pulso),
            }
        }
    }
    puerta(p, l, alpha, t, pulso);
}

/// La acera de Chicago: hormigon con sus juntas encima, adoquin de ladrillo
/// debajo y, donde se acaba al borde del vacio, una valla de obras con su
/// farol rojo. El borde lleva el canto de tinta gordo, como en Viena.
fn acera(p: &Paseo, l: &Layout, cam: f32, t: f32) {
    let fondo = VISTA_H + 10.0;
    for &(a, b) in &p.def.suelo {
        if b < cam - 20.0 || a > cam + VISTA_W + 20.0 {
            continue;
        }
        let (a0, b0) = (a.max(cam - 20.0), b.min(cam + VISTA_W + 20.0));
        rect(l, a0, SUELO_Y, b0 - a0, fondo - SUELO_Y, ASFALTO);
        for fila in 0..4 {
            let y = SUELO_Y + 20.0 + fila as f32 * 13.0;
            let desfase = if fila % 2 == 0 { 0.0 } else { 18.0 };
            let mut x = ((a0 - desfase) / 36.0).floor() * 36.0 + desfase;
            while x < b0 {
                if x >= a && x + 33.0 <= b {
                    rect(l, x + 2.0, y, 32.0, 10.0, ADOQUIN);
                }
                x += 36.0;
            }
        }
        rect(l, a0, SUELO_Y, b0 - a0, 14.0, HORMIGON);
        let mut x = (a0 / 90.0).floor() * 90.0;
        while x < b0 {
            if x > a && x < b {
                linea(
                    l,
                    (x, SUELO_Y + 2.0),
                    (x, SUELO_Y + 14.0),
                    1.2,
                    fade(TINTA, 0.5),
                );
            }
            x += 90.0;
        }
        linea(l, (a0, SUELO_Y), (b0, SUELO_Y), 3.5, TINTA);
        linea(l, (a0, SUELO_Y + 14.0), (b0, SUELO_Y + 14.0), 1.5, TINTA);
        for (x, hacia) in [(a, 1.0), (b, -1.0)] {
            if x <= 0.0 || x >= p.def.largo || x < cam - 60.0 || x > cam + VISTA_W + 60.0 {
                continue;
            }
            linea(l, (x, SUELO_Y), (x, fondo), 4.0, TINTA);
            valla(l, x + hacia * 34.0, t);
        }
    }
}

/// Una valla de obras al borde de la acera: dos caballetes, el tablon a rayas
/// y un farol rojo que late despacio.
fn valla(l: &Layout, x: f32, t: f32) {
    let y = SUELO_Y;
    for dx in [-20.0, 20.0] {
        linea(l, (x + dx - 6.0, y), (x + dx, y - 34.0), 3.0, TINTA);
        linea(l, (x + dx + 6.0, y), (x + dx, y - 34.0), 3.0, TINTA);
    }
    caja(l, x - 30.0, y - 40.0, 60.0, 10.0, HUESO, 2.0);
    for i in 0..4 {
        let a = x - 28.0 + i as f32 * 15.0;
        tri(
            l,
            (a, y - 40.0),
            (a + 7.0, y - 40.0),
            (a + 1.0, y - 30.0),
            TINTA,
        );
        tri(
            l,
            (a + 7.0, y - 40.0),
            (a + 8.0, y - 30.0),
            (a + 1.0, y - 30.0),
            TINTA,
        );
    }
    let k = 0.5 + 0.5 * (t * 0.08).sin();
    circulo(l, x, y - 48.0, 12.0, fade(ROJO, 0.15 * k));
    circulo(l, x, y - 48.0, 5.0, TINTA);
    circulo(l, x, y - 48.0, 3.6, fade(ROJO, 0.6 + 0.4 * k));
}

/// Una azotea: el tejado es la cornisa, clara y con su canto de tinta, y
/// debajo baja el edificio entero hasta perderse. Las ventanas, encendidas
/// casi todas, y el ladrillo oscureciendose hacia abajo: la calle esta lejos.
fn azotea(l: &Layout, x: f32, y: f32, w: f32, t: f32, pulso: f32) {
    let fondo = VISTA_H + 10.0;
    let mut rng = Pcg32::new(0xA2_07E + x as u64);
    // Una chimenea al fondo del tejado, que un tejado liso es un suelo.
    let cx = x + 20.0 + rng.next_f32() * 30.0;
    caja(l, cx, y - 34.0, 24.0, 34.0, LADRILLO_OSCURO, 2.0);
    rect(l, cx - 4.0, y - 38.0, 32.0, 6.0, TINTA);

    rect(l, x, y, w, fondo - y, LADRILLO);
    let mut hy = y + 16.0;
    while hy < fondo {
        linea(l, (x, hy), (x + w, hy), 1.0, fade(TINTA, 0.22));
        hy += 12.0;
    }
    // Las ventanas: filas de dos hojas, con alguna silueta.
    let mut vy = y + 36.0;
    while vy < fondo {
        let mut vx = x + 26.0;
        while vx + 34.0 < x + w - 16.0 {
            let encendida = rng.next_f32() < 0.6;
            let c = if encendida {
                fade(VENTANA, 0.72 + 0.28 * pulso)
            } else {
                APAGADA
            };
            caja(l, vx, vy, 30.0, 44.0, c, 2.0);
            linea(l, (vx + 15.0, vy), (vx + 15.0, vy + 44.0), 1.5, TINTA);
            if encendida && rng.next_f32() < 0.25 {
                // Una pareja bailando detras del cristal, en sombra.
                let b = (t * 0.1 + vx).sin() * 2.0;
                circulo(l, vx + 11.0 + b, vy + 20.0, 4.0, fade(TINTA, 0.6));
                circulo(l, vx + 19.0 - b, vy + 21.0, 4.0, fade(TINTA, 0.6));
                rect(l, vx + 7.0, vy + 25.0, 16.0, 19.0, fade(TINTA, 0.6));
            }
            vx += 56.0;
        }
        vy += 78.0;
    }
    // Hacia abajo se va a la noche.
    for k in 0..6 {
        let sy = y + 150.0 + k as f32 * 40.0;
        if sy < fondo {
            rect(l, x, sy, w, fondo - sy, fade(VACIO, 0.14));
        }
    }
    linea(l, (x, y), (x, fondo), 4.0, TINTA);
    linea(l, (x + w, y), (x + w, fondo), 4.0, TINTA);
    // La cornisa, que es lo que se pisa.
    caja(l, x - 6.0, y, w + 12.0, 12.0, PIEDRA, 2.0);
    linea(l, (x - 8.0, y), (x + w + 8.0, y), 4.0, TINTA);
    let mut d = x;
    while d < x + w - 4.0 {
        rect(l, d + 2.0, y + 14.0, 6.0, 6.0, PIEDRA);
        d += 14.0;
    }
    // En la mas ancha, un anuncio con bombillas en el tejado.
    if w >= 400.0 {
        let (ax, ay) = (x + w - 190.0, y - 120.0);
        linea(l, (ax + 30.0, ay + 60.0), (ax + 30.0, y), 3.0, TINTA);
        linea(l, (ax + 130.0, ay + 60.0), (ax + 130.0, y), 3.0, TINTA);
        linea(
            l,
            (ax + 30.0, y - 10.0),
            (ax + 130.0, ay + 64.0),
            2.0,
            TINTA,
        );
        caja(l, ax, ay, 160.0, 60.0, color_u8!(40, 62, 54, 255), 3.0);
        rotulo(
            l,
            "HOT JAZZ",
            ax + 80.0,
            ay + 40.0,
            24.0,
            fade(VENTANA, 0.7 + 0.3 * pulso),
        );
        bombillas(l, (ax, ay - 5.0), (ax + 160.0, ay - 5.0), 14, t, 2.6);
    }
}

/// Las vias del elevado: carriles claros encima (lo que se pisa), traviesas,
/// la viga de celosia remachada y los caballetes de acero hasta abajo.
fn via(l: &Layout, x: f32, y: f32, w: f32, pulso: f32) {
    let fondo = VISTA_H + 10.0;
    // Los caballetes: dos patas abiertas y su cruz.
    let n = ((w / 150.0) as i32).max(1);
    for i in 0..=n {
        let cx = x + 20.0 + (w - 40.0) * i as f32 / n as f32;
        let top = y + 48.0;
        for dx in [-14.0, 14.0] {
            linea(l, (cx + dx * 0.6, top), (cx + dx, fondo), 9.0, TINTA);
            linea(l, (cx + dx * 0.6, top), (cx + dx, fondo), 5.5, ACERO);
        }
        let mut cy = top + 10.0;
        while cy < fondo {
            linea(l, (cx - 10.0, cy), (cx + 11.0, cy + 36.0), 1.5, TINTA);
            linea(l, (cx + 10.0, cy), (cx - 11.0, cy + 36.0), 1.5, TINTA);
            cy += 40.0;
        }
    }
    // La viga, con la celosia en X y los remaches.
    caja(l, x, y + 10.0, w, 38.0, ACERO, 2.5);
    rect(l, x, y + 10.0, w, 5.0, ACERO_CLARO);
    rect(l, x, y + 43.0, w, 5.0, ACERO_CLARO);
    let mut cx = x;
    while cx < x + w - 1.0 {
        let b = (cx + 36.0).min(x + w);
        linea(l, (cx, y + 15.0), (b, y + 43.0), 1.5, fade(TINTA, 0.7));
        linea(l, (cx, y + 43.0), (b, y + 15.0), 1.5, fade(TINTA, 0.7));
        circulo(l, cx + 4.0, y + 12.5, 1.4, TINTA);
        circulo(l, cx + 4.0, y + 45.5, 1.4, TINTA);
        cx += 36.0;
    }
    // Traviesas y carril: el carril es claro y lleva canto de tinta.
    rect(l, x - 4.0, y + 2.0, w + 8.0, 8.0, TINTA);
    let mut tx = x;
    while tx < x + w {
        rect(l, tx, y + 3.0, 10.0, 6.0, MADERA);
        tx += 16.0;
    }
    rect(l, x - 4.0, y - 3.0, w + 8.0, 5.0, RAIL);
    linea(l, (x - 4.0, y - 3.0), (x + w + 4.0, y - 3.0), 2.5, TINTA);
    // El semaforo al final de cada tramo: rojo, que la via esta cortada.
    for sx in [x + 6.0, x + w - 6.0] {
        linea(l, (sx, y - 3.0), (sx, y - 40.0), 2.5, TINTA);
        caja(l, sx - 5.0, y - 56.0, 10.0, 16.0, HIERRO, 1.5);
        circulo(l, sx, y - 48.0, 3.2, fade(ROJO, 0.6 + 0.4 * pulso));
    }
}

/// Un deposito de agua: barril de duelas con sus aros, la tapa plana (que se
/// pisa) y las patas de hierro hasta el tejado.
fn deposito(l: &Layout, x: f32, y: f32, w: f32, abajo: f32) {
    let barril = (abajo - y - 28.0).clamp(30.0, 60.0);
    let pie = y + barril;
    for (a, b) in [
        (x + 8.0, x + 2.0),
        (x + w - 8.0, x + w - 2.0),
        (x + w * 0.5, x + w * 0.5),
    ] {
        linea(l, (a, pie), (b, abajo), 4.0, TINTA);
    }
    linea(l, (x + 8.0, pie + 4.0), (x + w - 2.0, abajo), 1.5, TINTA);
    linea(l, (x + w - 8.0, pie + 4.0), (x + 2.0, abajo), 1.5, TINTA);
    caja(l, x, y, w, barril, MADERA, 2.5);
    let mut dx = x + 8.0;
    while dx < x + w {
        linea(l, (dx, y), (dx, pie), 1.0, fade(TINTA, 0.5));
        dx += 9.0;
    }
    for k in [0.3, 0.7] {
        linea(l, (x, y + barril * k), (x + w, y + barril * k), 2.5, TINTA);
    }
    // La tapa.
    caja(l, x - 5.0, y - 1.0, w + 10.0, 7.0, PIEDRA, 2.0);
    linea(l, (x - 7.0, y - 1.0), (x + w + 7.0, y - 1.0), 3.5, TINTA);
}

/// Un rellano de escalera de incendios: la reja de hierro con su borde claro,
/// la barandilla, las mensulas y la escalera al rellano de abajo (o, el mas
/// bajo, la escalerilla que cuelga hacia la calle).
fn rellano(p: &Paseo, l: &Layout, i: usize) {
    let pl = p.def.plataformas[i];
    let (x, y, w) = (pl.x, pl.y, pl.ancho);
    // La escalera hasta el rellano de debajo de este bloque, si lo hay; si no,
    // la escalerilla colgante.
    let debajo = p
        .def
        .plataformas
        .iter()
        .enumerate()
        .filter(|&(k, o)| {
            k != i && o.y > y && o.y - y < 120.0 && o.x < x + w + 60.0 && o.x + o.ancho > x - 60.0
        })
        .map(|(_, o)| *o)
        .min_by(|a, b| a.y.total_cmp(&b.y));
    match debajo {
        Some(o) => {
            // En diagonal y empinada, que es como bajan las de verdad: del
            // extremo de este que queda encima del otro a su extremo contrario.
            let (a, b) = if o.x < x {
                ((x + 12.0, y), (o.x + o.ancho - 12.0, o.y))
            } else {
                ((x + w - 12.0, y), (o.x + 12.0, o.y))
            };
            linea(l, a, b, 6.0, TINTA);
            linea(l, (a.0, a.1 + 8.0), (b.0, b.1 + 8.0), 3.0, TINTA);
            for k in 1..8 {
                let f = k as f32 / 8.0;
                let (sx, sy) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
                linea(l, (sx, sy), (sx + 6.0, sy + 4.0), 2.0, TINTA);
            }
        }
        None => {
            let lx = x + w - 20.0;
            let fin = (SUELO_Y - 60.0).max(y + 20.0);
            linea(l, (lx - 7.0, y), (lx - 7.0, fin), 2.0, TINTA);
            linea(l, (lx + 7.0, y), (lx + 7.0, fin), 2.0, TINTA);
            let mut ly = y + 8.0;
            while ly < fin {
                linea(l, (lx - 7.0, ly), (lx + 7.0, ly), 1.5, TINTA);
                ly += 9.0;
            }
        }
    }
    // Las mensulas contra la pared.
    for mx in [x + 14.0, x + w - 14.0] {
        tri(
            l,
            (mx - 8.0, y + 6.0),
            (mx + 8.0, y + 6.0),
            (mx, y + 26.0),
            TINTA,
        );
    }
    // La barandilla: delgada, que no tape a quien pasa por detras.
    linea(l, (x, y - 30.0), (x + w, y - 30.0), 2.5, HIERRO);
    let mut bx = x;
    while bx <= x + w {
        linea(l, (bx, y - 30.0), (bx, y), 1.4, HIERRO);
        bx += 13.0;
    }
    // La reja, con el borde claro: lo que se pisa se lee.
    caja(l, x, y, w, 6.0, HIERRO, 2.0);
    rect(l, x, y - 1.0, w, 3.0, RAIL);
    linea(l, (x - 2.0, y - 1.5), (x + w + 2.0, y - 1.5), 2.5, TINTA);
}

/// La puerta del club, al final: una fachada Art Deco verde botella, con su
/// sol de oro encima de la puerta, la marquesina con el nombre del baile y
/// bombillas corriendo. La puerta se abre segun te acercas y dentro esta la
/// banda. Es la meta y tiene que verse desde lejos.
fn puerta(p: &Paseo, l: &Layout, alpha: f32, t: f32, pulso: f32) {
    let x = p.def.puerta;
    let cam = camara(p, alpha);
    if x + 400.0 < cam || x - 400.0 > cam + VISTA_W {
        return;
    }
    let (mostaza, verde) = paleta_charleston();
    let (cx, ancho, arriba) = (x + 40.0, 130.0, 300.0);

    // La fachada, escalonada.
    let fachada = color_u8!(40, 70, 58, 255);
    for (a, b, top) in [
        (x - 240.0, x + 700.0, 150.0),
        (x - 140.0, x + 220.0, 90.0),
        (x - 60.0, x + 140.0, 50.0),
    ] {
        caja(l, a, top, b - a, SUELO_Y - top, fachada, 3.0);
        // El remate de cada escalon, en oro con su filete de tinta.
        rect(l, a, top, b - a, 7.0, mostaza);
        linea(l, (a, top + 7.0), (b, top + 7.0), 2.0, TINTA);
    }
    // Las aletas verticales en oro.
    for i in 0..9 {
        let fx = x - 220.0 + i as f32 * 110.0;
        if (fx - cx).abs() < 110.0 {
            continue;
        }
        rect(l, fx - 4.0, 160.0, 8.0, SUELO_Y - 160.0, fade(mostaza, 0.8));
        linea(l, (fx - 4.0, 160.0), (fx - 4.0, SUELO_Y), 1.5, TINTA);
        linea(l, (fx + 4.0, 160.0), (fx + 4.0, SUELO_Y), 1.5, TINTA);
    }
    // Ventanales encendidos a los lados: el club esta lleno.
    for wx in [x - 170.0, x + 250.0, x + 400.0, x + 550.0] {
        caja(
            l,
            wx,
            200.0,
            60.0,
            110.0,
            fade(VENTANA, 0.8 + 0.2 * pulso),
            3.0,
        );
        linea(l, (wx + 30.0, 200.0), (wx + 30.0, 310.0), 2.0, TINTA);
        linea(l, (wx, 250.0), (wx + 60.0, 250.0), 2.0, TINTA);
    }
    // El sol encima de todo, latiendo, con su abanico de bombillas.
    let (sx, sy) = (cx, 150.0);
    sol(
        l,
        sx,
        sy,
        90.0,
        fade(color_u8!(214, 166, 64, 255), 0.85 + 0.15 * pulso),
        fade(color_u8!(150, 110, 44, 255), 0.85 + 0.15 * pulso),
    );
    arco(l, sx, sy, 90.0, 4.0, TINTA);
    for i in 0..13 {
        let a = PI + PI * (i as f32 + 0.5) / 13.0;
        let (s, c) = sin_cos(a);
        let encendida = (i + (t / 9.0) as usize).is_multiple_of(3);
        circulo(l, sx + c * 100.0, sy + s * 100.0, 5.0, TINTA);
        circulo(
            l,
            sx + c * 100.0,
            sy + s * 100.0,
            3.6,
            if encendida {
                color_u8!(255, 236, 170, 255)
            } else {
                color_u8!(110, 86, 44, 255)
            },
        );
    }

    // La luz de dentro, la banda y dos que bailan el charleston.
    let abierta = ((p.jugadora.render_pos(alpha).x - (x - 420.0)) / 320.0).clamp(0.0, 1.0);
    rect(
        l,
        cx - ancho * 0.5,
        arriba,
        ancho,
        SUELO_Y - arriba,
        fade(LUZ, 0.95),
    );
    musico(l, cx - 34.0, SUELO_Y, Instrumento::Trompeta, pulso);
    musico(l, cx + 30.0, SUELO_Y, Instrumento::Bateria, pulso);
    // Las hojas giran sobre sus bisagras y se estrechan, como en Viena.
    let hoja = ancho * 0.5 * (1.0 - abierta * 0.85);
    for (x0, dir) in [(cx - ancho * 0.5, 1.0), (cx + ancho * 0.5, -1.0)] {
        let x1 = x0 + hoja * dir;
        let (a, b) = (x0.min(x1), x0.max(x1));
        caja(l, a, arriba, b - a, SUELO_Y - arriba, verde, 1.5);
        if b - a > 14.0 {
            // El cristal con un medio sol grabado.
            let (gx, gw) = (a + 6.0, b - a - 12.0);
            rect(l, gx, arriba + 20.0, gw, 60.0, fade(ORO, 0.5));
            linea(
                l,
                (gx + gw * 0.5, arriba + 80.0),
                (gx, arriba + 20.0),
                1.5,
                TINTA,
            );
            linea(
                l,
                (gx + gw * 0.5, arriba + 80.0),
                (gx + gw, arriba + 20.0),
                1.5,
                TINTA,
            );
        }
    }
    linea(
        l,
        (cx - ancho * 0.5, arriba),
        (cx - ancho * 0.5, SUELO_Y),
        5.0,
        TINTA,
    );
    linea(
        l,
        (cx + ancho * 0.5, arriba),
        (cx + ancho * 0.5, SUELO_Y),
        5.0,
        TINTA,
    );

    // La marquesina con el nombre del baile.
    let (mx, my, mw, mh) = (cx - 130.0, 236.0, 260.0, 46.0);
    caja(l, mx, my, mw, mh, color_u8!(246, 232, 196, 255), 3.0);
    let nombre = nombre(&p.def.jefe).to_uppercase();
    rotulo(l, &nombre, cx, my + 33.0, 24.0, TINTA);
    bombillas(l, (mx, my - 6.0), (mx + mw, my - 6.0), 20, t, 3.0);
    bombillas(
        l,
        (mx, my + mh + 6.0),
        (mx + mw, my + mh + 6.0),
        20,
        t + 18.0,
        3.0,
    );

    // La luz se derrama por la acera segun se abre, y al llegar lo inunda.
    let llegada = p
        .completado
        .map_or(0.0, |d| (((p.tick - d) as f32 + alpha) / 40.0).min(1.0));
    let derrame = fade(LUZ, 0.10 + 0.25 * abierta + 0.3 * llegada);
    let (izq, der) = (cx - ancho * 0.5, cx + ancho * 0.5);
    tri(
        l,
        (izq, SUELO_Y),
        (der, SUELO_Y),
        (der + 160.0, VISTA_H),
        derrame,
    );
    tri(
        l,
        (izq, SUELO_Y),
        (der + 160.0, VISTA_H),
        (izq - 160.0, VISTA_H),
        derrame,
    );
}

fn paleta_charleston() -> (Color, Color) {
    crate::paleta::del_baile(2)
}

// ---------------------------------------------------------------------------
// Los enemigos
// ---------------------------------------------------------------------------

/// Los enemigos de Chicago. `tiempo` es el del paseo: la flapper tiene que
/// pintar el collar donde el nucleo dice que esta.
pub fn enemigo(
    e: &Enemigo,
    l: &Layout,
    tick: u64,
    t: f32,
    tiempo: u32,
    tintas: (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    match e.tipo {
        Tipo::Flapper => flapper(e, l, t, tiempo, tintas, tinte),
        Tipo::Saxo => saxo(e, l, tick, tiempo, tintas, tinte),
        Tipo::Saltarin => saltarin(e, l, tick, t, tiempo, tintas, tinte),
        Tipo::Canotier => canotier(e, l, tick, t, tiempo, tintas, tinte),
        Tipo::Fila => de_la_fila(e, l, tick, t, tiempo, tintas, tinte),
        Tipo::Contrabajista => contrabajista(e, l, tick, t, tiempo, tintas, tinte),
        _ => {}
    }
}

/// Lo que queda de un golpe de clave: 1 en el golpe y cae a 0 en seis ticks.
/// Es el cabeceo de los que bailan y tocan.
fn golpe_de_clave(tick: u64, tiempo: u32) -> f32 {
    let (_, dentro, _) = paseo::clave(tick, tiempo);
    (1.0 - dentro as f32 / 6.0).max(0.0)
}

/// Una flapper: melena a lo garcon con cinta y pluma, vestido de talle bajo
/// con flecos, y el collar dando vueltas alrededor del cuello. Las tres perlas
/// gordas del collar son por donde van a salir las siguientes: mirarla es
/// saber donde no ponerse.
fn flapper(
    e: &Enemigo,
    l: &Layout,
    t: f32,
    tiempo: u32,
    (mostaza, verde): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, y) = (e.pos.x, e.pos.y);
    let pies = y + 38.0;
    // El paso del charleston: rodillas juntas y talones fuera, alternando cada
    // tiempo.
    let (s, _) = sin_cos(TAU * t / (2 * tiempo.max(1)) as f32);
    let rodilla = (x + s * 3.0, pies - 16.0);
    let vestido = tinte(mostaza);
    let piel = tinte(PIEL);
    con_tinta(|g, es_tinta| {
        let (v, pi) = if es_tinta {
            (TINTA, TINTA)
        } else {
            (vestido, piel)
        };
        for lado in [-1.0, 1.0] {
            let talon = (x + lado * (9.0 + 7.0 * s * lado), pies);
            linea(l, (x + lado * 4.0, y + 12.0), rodilla, 4.5 + 2.0 * g, pi);
            linea(l, rodilla, talon, 4.0 + 2.0 * g, pi);
        }
        // Talle bajo: el cuerpo recto y la falda de flecos.
        rect(l, x - 11.0 - g, y - 22.0 - g, 22.0 + 2.0 * g, 30.0 + g, v);
        rect(l, x - 14.0 - g, y + 4.0, 28.0 + 2.0 * g, 12.0 + g, v);
        circulo(l, x + e.dir * 2.0, y - 32.0, 9.0 + g, pi);
        // El brazo de la cadera.
        linea(
            l,
            (x - e.dir * 10.0, y - 18.0),
            (x - e.dir * 17.0, y - 6.0),
            3.5 + 2.0 * g,
            pi,
        );
        linea(
            l,
            (x - e.dir * 17.0, y - 6.0),
            (x - e.dir * 10.0, y + 2.0),
            3.5 + 2.0 * g,
            pi,
        );
    });
    for lado in [-1.0, 1.0] {
        let talon = (x + lado * (9.0 + 7.0 * s * lado), pies);
        rect(l, talon.0 - 4.0, pies - 3.0, 8.0, 4.0, TINTA);
    }
    // Los flecos, que se mecen al reves que las rodillas.
    for i in 0..7 {
        let fx = x - 12.0 + i as f32 * 4.0;
        linea(l, (fx, y + 14.0), (fx - s * 4.0, y + 22.0), 1.5, TINTA);
    }
    linea(l, (x - 13.0, y + 4.0), (x + 13.0, y + 4.0), 1.5, TINTA);
    // La melena: un casco de tinta con el pico en la mejilla, la cinta y la
    // pluma.
    let d = e.dir;
    circulo(l, x - d * 1.5, y - 35.0, 9.5, TINTA);
    tri(
        l,
        (x + d * 8.0, y - 34.0),
        (x + d * 2.0, y - 26.0),
        (x - d * 6.0, y - 30.0),
        TINTA,
    );
    circulo(l, x + d * 4.5, y - 31.0, 5.0, piel);
    linea(
        l,
        (x - 9.0, y - 38.0),
        (x + 9.0, y - 38.0),
        2.5,
        tinte(verde),
    );
    // La pluma, curvada hacia atras.
    let (px, py) = (x - d * 6.0, y - 39.0);
    for (g, c) in [(2.0, TINTA), (0.0, tinte(mostaza))] {
        tri(
            l,
            (px - 2.5 - g, py + g),
            (px + 2.5 + g, py + g),
            (px - d * 8.0, py - 12.0 - g),
            c,
        );
        tri(
            l,
            (px - d * 8.0 - 3.0 - g, py - 11.0),
            (px - d * 8.0 + 3.0 + g, py - 11.0),
            (px - d * 16.0, py - 20.0 - g),
            c,
        );
    }
    circulo(l, x + d * 7.0, y - 29.0, 1.2, ROJO);

    // El collar: un aro de perlas girando alrededor del cuello, y tres gordas
    // donde apunta el giro.
    let giro = paseo::giro_collar(t, tiempo);
    let cuello = (x, y - 12.0);
    for i in 0..15 {
        let a = giro + TAU * i as f32 / 15.0;
        let (sa, ca) = sin_cos(a);
        let (px, py) = (cuello.0 + ca * 20.0, cuello.1 + sa * 20.0);
        circulo(l, px, py, 2.6, TINTA);
        circulo(l, px, py, 1.8, tinte(PERLA));
    }
    for k in 0..paseo::PERLAS_POR_GOLPE {
        let a = giro + TAU * k as f32 / paseo::PERLAS_POR_GOLPE as f32;
        let (sa, ca) = sin_cos(a);
        let (px, py) = (cuello.0 + ca * 24.0, cuello.1 + sa * 24.0);
        linea(l, cuello, (px, py), 1.2, fade(PERLA, 0.6));
        circulo(l, px, py, 5.2, TINTA);
        circulo(l, px, py, 3.8, tinte(PERLA));
    }
}

/// El saxofonista: traje verde botella, sombrero de fieltro y el saxo de oro.
/// En cada golpe de clave echa el cuerpo atras y sopla; la campana esta donde
/// el nucleo suelta las notas.
fn saxo(
    e: &Enemigo,
    l: &Layout,
    tick: u64,
    tiempo: u32,
    (mostaza, verde): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, y, d) = (e.pos.x, e.pos.y, e.dir);
    let pies = y + 36.0;
    let atras = golpe_de_clave(tick, tiempo) * 5.0;
    let (hx, hy) = (x - d * atras * 0.6, y - 34.0 + atras * 0.3);
    // Chaqueta blanca de orquesta y pantalon verde botella: contra la noche
    // verde, lo blanco es lo que le despega del fondo.
    let chaqueta = tinte(color_u8!(236, 226, 204, 255));
    let pantalon = tinte(verde);
    let piel = tinte(PIEL);
    con_tinta(|g, es_tinta| {
        let (tr, pa, pi) = if es_tinta {
            (TINTA, TINTA, TINTA)
        } else {
            (chaqueta, pantalon, piel)
        };
        rect(l, x - 9.0 - g, y + 6.0 - g, 7.0 + 2.0 * g, 30.0 + g, pa);
        rect(l, x + 2.0 - g, y + 6.0 - g, 7.0 + 2.0 * g, 30.0 + g, pa);
        tri(
            l,
            (hx - 13.0 - g, y - 24.0 - g),
            (hx + 13.0 + g, y - 24.0 - g),
            (x + 11.0 + g, y + 10.0 + g),
            tr,
        );
        tri(
            l,
            (hx - 13.0 - g, y - 24.0 - g),
            (x + 11.0 + g, y + 10.0 + g),
            (x - 11.0 - g, y + 10.0 + g),
            tr,
        );
        circulo(l, hx, hy, 8.5 + g, pi);
    });
    // Solapas verdes, pajarita, zapatos y el sombrero.
    tri(
        l,
        (hx - 5.0, y - 24.0),
        (hx + 5.0, y - 24.0),
        (hx, y - 8.0),
        pantalon,
    );
    tri(
        l,
        (hx - 4.0, y - 23.0),
        (hx + 4.0, y - 23.0),
        (hx, y - 13.0),
        tinte(HUESO),
    );
    tri(
        l,
        (hx - 5.0, y - 25.0),
        (hx, y - 22.0),
        (hx - 5.0, y - 19.0),
        TINTA,
    );
    tri(
        l,
        (hx + 5.0, y - 25.0),
        (hx, y - 22.0),
        (hx + 5.0, y - 19.0),
        TINTA,
    );
    rect(l, x - 11.0, pies - 4.0, 10.0, 4.0, TINTA);
    rect(l, x + 1.0, pies - 4.0, 10.0, 4.0, TINTA);
    // El fieltro: ala ancha y copa baja con su hendidura.
    rect(l, hx - 14.0, hy - 8.0, 28.0, 4.0, TINTA);
    tri(
        l,
        (hx - 9.0, hy - 7.0),
        (hx - 7.0, hy - 16.0),
        (hx + 9.0, hy - 7.0),
        TINTA,
    );
    tri(
        l,
        (hx + 9.0, hy - 7.0),
        (hx - 7.0, hy - 16.0),
        (hx + 7.0, hy - 16.0),
        TINTA,
    );
    rect(l, hx - 8.5, hy - 10.0, 17.0, 2.5, tinte(mostaza));
    // El saxo: boquilla en la boca, cuerpo que baja y campana hacia delante.
    let oro = tinte(ORO);
    let boca = (hx + d * 6.0, hy + 4.0);
    let codo = (x + d * 4.0, y + 16.0);
    let campana = (x + d * 20.0, y + 6.0);
    for (g, c) in [(4.0, TINTA), (0.0, oro)] {
        linea(l, boca, (x + d * 2.0, y - 6.0), 3.0 + g, c);
        linea(l, (x + d * 2.0, y - 6.0), codo, 6.0 + g, c);
        linea(l, codo, (x + d * 14.0, y + 14.0), 7.0 + g, c);
        linea(l, (x + d * 14.0, y + 14.0), campana, 8.0 + g, c);
    }
    // La campana, abierta hacia delante: un aro de oro con la boca negra.
    circulo(l, campana.0 + d * 3.0, campana.1 - 2.0, 7.5, TINTA);
    circulo(l, campana.0 + d * 3.0, campana.1 - 2.0, 5.5, oro);
    circulo(l, campana.0 + d * 4.5, campana.1 - 2.5, 3.2, TINTA);
    // Las manos en las llaves.
    circulo(l, x + d * 1.0, y - 2.0, 3.2, piel);
    circulo(l, x + d * 6.0, y + 10.0, 3.2, piel);
}

/// El saltarin: canotier de paja, americana a rayas en las dos tintas,
/// pantalon blanco y botines. En el aire, la patada del charleston; en el
/// suelo, agachado para el siguiente brinco, que es cuando se le puede saltar.
fn saltarin(
    e: &Enemigo,
    l: &Layout,
    tick: u64,
    t: f32,
    tiempo: u32,
    (mostaza, verde): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, y, d) = (e.pos.x, e.pos.y, e.dir);
    let (_, dentro, largo) = paseo::clave(tick, tiempo);
    let en_suelo = dentro + EN_EL_SUELO >= largo;
    let agacha = if en_suelo { 6.0 } else { 0.0 };
    let pies = y + 36.0;
    let cadera = (x, y + 10.0 + agacha);
    let americana = tinte(mostaza);
    let blanco = tinte(HUESO);
    let piel = tinte(PIEL);
    // Las piernas: agachado, dobladas; en el aire, una patada adelante y la
    // otra recogida, alternando cada brinco.
    let lado = if (tick / u64::from((tiempo / 2).max(1))).is_multiple_of(2) {
        1.0
    } else {
        -1.0
    };
    let (pa, pb) = if en_suelo {
        ((x - 12.0, pies), (x + 12.0, pies))
    } else {
        (
            (x + lado * d * 22.0, pies - 10.0),
            (x - lado * d * 6.0, pies - 4.0),
        )
    };
    let rod = |p: (f32, f32)| {
        (
            (cadera.0 + p.0) * 0.5 + d * 4.0,
            (cadera.1 + p.1) * 0.5 - 2.0,
        )
    };
    con_tinta(|g, es_tinta| {
        let (am, bl, pi) = if es_tinta {
            (TINTA, TINTA, TINTA)
        } else {
            (americana, blanco, piel)
        };
        for p in [pa, pb] {
            let r = rod(p);
            linea(l, cadera, r, 6.0 + 2.0 * g, bl);
            linea(l, r, p, 5.0 + 2.0 * g, bl);
        }
        rect(
            l,
            x - 12.0 - g,
            y - 20.0 + agacha - g,
            24.0 + 2.0 * g,
            32.0 + g,
            am,
        );
        // Los brazos, abiertos en el aire y en jarras en el suelo.
        let brazo = if en_suelo { 8.0 } else { -16.0 };
        for s in [-1.0, 1.0] {
            let codo = (x + s * 18.0, y - 12.0 + agacha);
            linea(
                l,
                (x + s * 10.0, y - 16.0 + agacha),
                codo,
                5.0 + 2.0 * g,
                am,
            );
            linea(
                l,
                codo,
                (x + s * 24.0, y + brazo + agacha),
                4.0 + 2.0 * g,
                am,
            );
        }
        circulo(l, x + d * 2.0, y - 30.0 + agacha, 9.0 + g, pi);
    });
    // Las rayas verdes de la americana.
    for i in 0..4 {
        let sx = x - 9.0 + i as f32 * 6.0;
        linea(
            l,
            (sx, y - 19.0 + agacha),
            (sx, y + 11.0 + agacha),
            2.0,
            tinte(verde),
        );
    }
    // Botines de dos colores.
    for p in [pa, pb] {
        rect(l, p.0 - 5.0, p.1 - 5.0, 10.0, 5.0, TINTA);
        rect(l, p.0 - 3.0, p.1 - 4.0, 5.0, 3.0, blanco);
    }
    // El canotier, un poco ladeado, y la sonrisa.
    let inclina = (t * 0.2).sin() * 1.5;
    let hy = y - 38.0 + agacha;
    rect(l, x - 15.0, hy + inclina, 30.0, 4.0, TINTA);
    rect(l, x - 13.5, hy + 0.8 + inclina, 27.0, 2.4, tinte(PAJA));
    rect(l, x - 9.0, hy - 8.0 + inclina, 18.0, 9.0, TINTA);
    rect(l, x - 7.5, hy - 6.5 + inclina, 15.0, 6.0, tinte(PAJA));
    rect(l, x - 7.5, hy - 3.0 + inclina, 15.0, 2.0, TINTA);
    linea(
        l,
        (x + d * 1.0, y - 26.0 + agacha),
        (x + d * 7.0, y - 27.0 + agacha),
        1.6,
        TINTA,
    );
}

/// Lo que vuela en Chicago: perlas del collar, corcheas del saxo y lo del
/// contrabajista. Las rosas llevan el anillo que late, que es como se dice
/// "parry" en este juego.
pub fn proyectil(q: &Proyectil, l: &Layout, t: f32) {
    let (x, y) = (q.pos.x, q.pos.y);
    let (mostaza, _) = paleta_charleston();
    match q.forma {
        Forma::Negra => negra(q, l, mostaza),
        Forma::Onda => onda(q, l, t),

        Forma::Perla => {
            let c = if q.rosa { ROSA } else { PERLA };
            circulo(l, x, y, 8.4, TINTA);
            circulo(l, x, y, 6.4, c);
            circulo(l, x - 2.0, y - 2.2, 1.8, fade(WHITE, 0.9));
        }
        _ => {
            let c = if q.rosa { ROSA } else { mostaza };
            con_tinta(|g, es_tinta| {
                let c = if es_tinta { TINTA } else { c };
                let p = l.to_screen(x - 2.0, y + 5.0);
                draw_ellipse(p.x, p.y, l.len(6.5 + g), l.len(5.0 + g), -22.0, c);
                linea(l, (x + 3.5, y + 4.0), (x + 3.5, y - 13.0), 2.4 + 2.0 * g, c);
                tri(
                    l,
                    (x + 3.5 - g, y - 14.0 - g),
                    (x + 11.0 + g, y - 5.0),
                    (x + 3.5 - g, y - 7.0 + g),
                    c,
                );
            });
        }
    }
    if q.rosa {
        let k = 0.5 + 0.5 * (t * 0.2).sin();
        let p = l.to_screen(x, y);
        draw_circle_lines(
            p.x,
            p.y,
            l.len(16.0 + 4.0 * k),
            2.0,
            fade(WHITE, 0.4 + 0.4 * k),
        );
    }
}

// ---------------------------------------------------------------------------
// El tren
// ---------------------------------------------------------------------------

/// Donde van los carriles del tren: por debajo del techo de los vagones, a
/// la altura de un tercer piso. Lejos de los pies a proposito: lo que se pisa
/// es el techo, y unos carriles a la altura de un tejado se querrian pisar.
const RIEL: f32 = 485.0;
/// El granate de los vagones, que contra la noche verde se despega solo.
const GRANATE: Color = color_u8!(128, 42, 40, 255);
const GRANATE_OSCURO: Color = color_u8!(84, 28, 30, 255);

/// El hueco por el que va el tren de la plataforma `i`: de la pared del
/// tejado de su izquierda a la del de su derecha.
fn hueco_del_tren(p: &Paseo, i: usize) -> (f32, f32) {
    let pl = p.def.plataformas[i];
    let fijas = || p.def.plataformas.iter().filter(|o| o.mueve.is_none());
    let a = fijas()
        .map(|o| o.x + o.ancho)
        .filter(|&b| b <= pl.x + 1.0)
        .fold(f32::MIN, f32::max);
    let b = fijas()
        .map(|o| o.x)
        .filter(|&a| a >= pl.x + pl.ancho - 1.0)
        .fold(f32::MAX, f32::min);
    (
        if a == f32::MIN { pl.x - 20.0 } else { a },
        if b == f32::MAX {
            pl.x + pl.ancho + 20.0
        } else {
            b
        },
    )
}

/// La via del elevado entre dos edificios: carriles con sus traviesas, la
/// viga de celosia remachada y las columnas que bajan al callejon, con las
/// mensulas clavadas en las dos paredes. Es el hierro de Chicago.
fn via_del_tren(l: &Layout, a: f32, b: f32, pulso: f32) {
    let fondo = VISTA_H + 10.0;
    let (viga, canto) = (RIEL + 6.0, 34.0);
    let bajo = viga + canto;
    // Las columnas, con su cartela contra la viga.
    let n = ((b - a) / 200.0).max(1.0) as i32;
    for i in 1..n {
        let cx = a + (b - a) * i as f32 / n as f32;
        caja(l, cx - 8.0, bajo, 16.0, fondo - bajo, ACERO, 2.0);
        rect(l, cx - 8.0, bajo, 4.0, fondo - bajo, ACERO_CLARO);
        for lado in [-1.0, 1.0] {
            tri(
                l,
                (cx + lado * 8.0, bajo),
                (cx + lado * 30.0, bajo),
                (cx + lado * 8.0, bajo + 16.0),
                TINTA,
            );
        }
    }
    // Las mensulas de las paredes: la via sale de un edificio y entra en otro.
    for (x, hacia) in [(a, 1.0), (b, -1.0)] {
        tri(
            l,
            (x, viga),
            (x + hacia * 40.0, bajo),
            (x, bajo + 30.0),
            TINTA,
        );
        tri(
            l,
            (x + hacia * 3.0, viga + 6.0),
            (x + hacia * 30.0, bajo),
            (x + hacia * 3.0, bajo + 20.0),
            ACERO,
        );
    }
    // La viga, en celosia y con remaches.
    caja(l, a, viga, b - a, canto, ACERO, 2.5);
    rect(l, a, viga, b - a, 5.0, ACERO_CLARO);
    rect(l, a, bajo - 5.0, b - a, 5.0, ACERO_CLARO);
    let mut x = a;
    while x < b - 1.0 {
        let x1 = (x + 30.0).min(b);
        linea(l, (x, viga + 5.0), (x1, bajo - 5.0), 1.6, fade(TINTA, 0.7));
        linea(l, (x, bajo - 5.0), (x1, viga + 5.0), 1.6, fade(TINTA, 0.7));
        circulo(l, x + 3.0, viga + 2.5, 1.4, TINTA);
        circulo(l, x + 3.0, bajo - 2.5, 1.4, TINTA);
        x += 30.0;
    }
    // Traviesas, el tercer carril con su luz y el carril.
    rect(l, a, RIEL, b - a, 6.0, TINTA);
    let mut x = a;
    while x < b {
        rect(l, x + 1.0, RIEL + 1.0, 9.0, 4.0, MADERA);
        x += 14.0;
    }
    linea(
        l,
        (a, RIEL + 3.0),
        (b, RIEL + 3.0),
        1.2,
        fade(LUZ, 0.25 + 0.2 * pulso),
    );
    rect(l, a, RIEL - 3.0, b - a, 3.0, RAIL);
    linea(l, (a, RIEL - 3.0), (b, RIEL - 3.0), 1.5, TINTA);
}

/// Un vagon del elevado de los anos veinte: caja granate, plataformas
/// abiertas en las puntas con su barandilla, ventanillas encendidas con
/// gente dentro, el letrero de la linea y dos bojes con las ruedas girando.
/// El techo, claro y con canto de tinta, es lo que se pisa.
fn vagon(l: &Layout, x: f32, y: f32, w: f32, t: f32, pulso: f32) {
    // Los vagones se solapan para que no haya rendija: se pinta mas estrecho.
    let (a, b) = (x + 3.0, x + w - 3.0);
    let suelo = RIEL - 16.0;
    let techo = y + 8.0;
    // Los bojes, con las ruedas girando segun avanza: el angulo sale de x.
    let (s, c) = sin_cos(x / 9.0);
    for bx in [a + 40.0, b - 40.0] {
        caja(l, bx - 28.0, suelo, 56.0, 6.0, HIERRO, 1.5);
        for dx in [-16.0, 16.0] {
            let (cx, cy) = (bx + dx, RIEL - 9.0);
            circulo(l, cx, cy, 9.5, TINTA);
            circulo(l, cx, cy, 7.5, HIERRO);
            let r = 7.0;
            linea(
                l,
                (cx - c * r, cy - s * r),
                (cx + c * r, cy + s * r),
                1.5,
                TINTA,
            );
            linea(
                l,
                (cx + s * r, cy - c * r),
                (cx - s * r, cy + c * r),
                1.5,
                TINTA,
            );
            circulo(l, cx, cy, 2.0, RAIL);
        }
    }
    // El enganche, que tapa el solape con el siguiente.
    rect(l, b - 2.0, suelo - 20.0, 10.0, 6.0, TINTA);
    // La caja, con la franja de debajo de las ventanillas y el filete de oro.
    let (pa, pb) = (a + 22.0, b - 22.0);
    caja(l, pa, techo, pb - pa, suelo - techo, GRANATE, 2.5);
    rect(l, pa, suelo - 34.0, pb - pa, 34.0, GRANATE_OSCURO);
    linea(l, (pa, suelo - 34.0), (pb, suelo - 34.0), 2.0, TINTA);
    rect(l, pa, techo + 12.0, pb - pa, 4.0, fade(ORO, 0.7));
    // Las ventanillas, con gente que cabecea con el rag.
    let ventanas = ((pb - pa - 20.0) / 26.0) as i32;
    let mut rng = Pcg32::new(0x7AE4);
    for k in 0..ventanas {
        let vx = pa + 12.0 + k as f32 * 26.0;
        let vy = techo + 30.0;
        caja(l, vx, vy, 18.0, 34.0, fade(VENTANA, 0.8 + 0.2 * pulso), 1.8);
        if rng.next_f32() < 0.6 {
            let bote = pulso * 1.5 + (t * 0.05 + k as f32).sin() * 0.8;
            let sombra = fade(TINTA, 0.75);
            circulo(l, vx + 9.0, vy + 20.0 + bote, 4.5, sombra);
            rect(l, vx + 3.0, vy + 25.0 + bote, 12.0, 9.0, sombra);
            rect(l, vx + 3.5, vy + 15.0 + bote, 11.0, 2.0, sombra);
        }
    }
    // El letrero de la linea, debajo de las ventanillas del medio.
    let cx = (pa + pb) * 0.5;
    caja(
        l,
        cx - 26.0,
        techo + 72.0,
        52.0,
        16.0,
        color_u8!(246, 232, 196, 255),
        1.5,
    );
    rotulo(l, "LOOP", cx, techo + 85.0, 13.0, TINTA);
    // Las plataformas de las puntas: suelo, barandilla y el farol.
    for (p0, p1, farol) in [(a, pa, a + 4.0), (pb, b, b - 4.0)] {
        caja(l, p0, suelo - 8.0, p1 - p0, 8.0, HIERRO, 1.5);
        linea(l, (p0, suelo - 36.0), (p1, suelo - 36.0), 2.0, HIERRO);
        for bx in [p0 + 2.0, (p0 + p1) * 0.5, p1 - 2.0] {
            linea(l, (bx, suelo - 36.0), (bx, suelo - 8.0), 1.5, HIERRO);
        }
        linea(l, (farol, techo), (farol, suelo - 36.0), 2.0, TINTA);
        circulo(l, farol, techo + 20.0, 5.0, TINTA);
        circulo(l, farol, techo + 20.0, 3.5, fade(LUZ, 0.8 + 0.2 * pulso));
    }
    // El techo, de punta a punta: es lo que se pisa.
    caja(l, x + 1.0, y, w - 2.0, 8.0, PIEDRA, 2.0);
    linea(l, (x, y), (x + w, y), 3.5, TINTA);
    for vx in [a + 50.0, b - 50.0] {
        caja(l, vx - 10.0, y + 9.0, 20.0, 5.0, HIERRO, 1.0);
    }
}

// ---------------------------------------------------------------------------
// Los numeros de Chicago: el canotier, la fila y el contrabajista
// ---------------------------------------------------------------------------

/// Un trazo curvo de `a` a `b`, como los brazos de manguera de los jefes: la
/// Bezier de `skeleton::trazo` hecha de tramos rectos.
fn manga(l: &Layout, a: (f32, f32), b: (f32, f32), arqueo: f32, g: f32, c: Color) {
    const PASOS: usize = 6;
    let (va, vb) = (vec2(a.0, a.1), vec2(b.0, b.1));
    let mut antes = va;
    for i in 1..=PASOS {
        let p = skeleton::trazo(va, vb, arqueo, i as f32 / PASOS as f32);
        linea(l, (antes.x, antes.y), (p.x, p.y), g, c);
        circulo(l, p.x, p.y, g * 0.5, c);
        antes = p;
    }
}

/// Un canotier volando: ala de paja, copa baja y la cinta verde. Se ve de
/// plano o de canto segun gira, y en el tiron deja estela.
fn canotier(
    e: &Enemigo,
    l: &Layout,
    tick: u64,
    t: f32,
    tiempo: u32,
    (_, verde): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, y) = (e.pos.x, e.pos.y);
    let giro = paseo::giro_canotier(tick, tiempo);
    let (_, dentro, _) = paseo::clave(tick, tiempo);
    // La estela del tiron: donde estaba hace un poco, en la misma vuelta.
    if dentro < paseo::TIRON_CANOTIER {
        let (rx, ry) = paseo::VUELTA_CANOTIER;
        let sitio = |a: f32| {
            let (s, c) = sin_cos(a);
            (-rx * s, -ry * c)
        };
        let ahora = sitio(giro);
        for k in 1..4 {
            let (dx, dy) = sitio(giro - 0.22 * k as f32);
            let f = k as f32;
            circulo(
                l,
                x - ahora.0 + dx,
                y - ahora.1 + dy,
                9.0 - f * 2.0,
                fade(PAJA, 0.35 - 0.08 * f),
            );
        }
    }
    let (s, c) = sin_cos(giro * 1.5);
    let canto = 0.35 + 0.65 * s.abs();
    let inclina = 22.0 * c;
    let paja = tinte(PAJA);
    let cinta = tinte(if e.rosa { ROSA } else { verde });
    let p = l.to_screen(x, y + 3.0);
    // El ala, con su canto de tinta, y la copa encima.
    draw_ellipse(
        p.x,
        p.y,
        l.len(19.0),
        l.len(3.5 + 4.0 * canto),
        inclina,
        TINTA,
    );
    draw_ellipse(
        p.x,
        p.y,
        l.len(17.0),
        l.len(2.0 + 4.0 * canto),
        inclina,
        paja,
    );
    let (si, ci) = sin_cos(inclina * PI / 180.0);
    let rota = |dx: f32, dy: f32| (x + dx * ci - dy * si, y + 3.0 + dx * si + dy * ci);
    con_tinta(|g, es_tinta| {
        let (cp, cc) = if es_tinta {
            (TINTA, TINTA)
        } else {
            (paja, cinta)
        };
        let (a, b, c, d) = (
            rota(-10.0 - g, 0.0),
            rota(10.0 + g, 0.0),
            rota(9.0 + g, -12.0 - g),
            rota(-9.0 - g, -12.0 - g),
        );
        tri(l, a, b, c, cp);
        tri(l, a, c, d, cp);
        let (a, b, c, d) = (
            rota(-10.0, -2.0),
            rota(10.0, -2.0),
            rota(9.5, -6.0),
            rota(-9.5, -6.0),
        );
        tri(l, a, b, c, cc);
        tri(l, a, c, d, cc);
    });
    // El trenzado de la paja: una raya de tinta en el ala.
    linea(l, rota(-14.0, 1.0), rota(14.0, 1.0), 1.0, fade(TINTA, 0.5));
    if e.rosa {
        let k = 0.5 + 0.5 * (t * 0.2).sin();
        let q = l.to_screen(x, y);
        draw_circle_lines(
            q.x,
            q.y,
            l.len(22.0 + 4.0 * k),
            2.0,
            fade(WHITE, 0.4 + 0.4 * k),
        );
    }
}

/// Una chica de la fila: flapper de talle bajo, de dos tintas alternas, con
/// los brazos enlazados con las de su lado. Patean todas a la vez en cada
/// golpe de clave, y las dos del hueco levantan el brazo de fuera cuando se
/// abre: es el aviso de por donde se pasa.
fn de_la_fila(
    e: &Enemigo,
    l: &Layout,
    tick: u64,
    t: f32,
    tiempo: u32,
    (mostaza, verde): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, y) = (e.pos.x, e.pos.y);
    let pies = y + 38.0;
    let k = e.en_la_fila();
    let (golpe, _, _) = paseo::clave(tick, tiempo);
    let patada = golpe_de_clave(tick, tiempo);
    let (s, _) = sin_cos(TAU * t / (2 * tiempo.max(1)) as f32);
    let hueco = paseo::hueco_fila(tick, tiempo);
    let vestido = tinte(if k.is_multiple_of(2) { mostaza } else { verde });
    let cinta = tinte(if k.is_multiple_of(2) { verde } else { mostaza });
    let piel = tinte(PIEL);
    // La pierna que patea va cambiando con el golpe.
    let lado_patada = if golpe.is_multiple_of(2) { -1.0 } else { 1.0 };
    let cadera = (x, y + 12.0);
    // Hacia que lado esta el hueco, si es una de las del medio.
    let al_hueco = if k + 1 == paseo::FILA / 2 {
        Some(1.0)
    } else if k == paseo::FILA / 2 {
        Some(-1.0)
    } else {
        None
    };
    con_tinta(|g, es_tinta| {
        let (v, pi) = if es_tinta {
            (TINTA, TINTA)
        } else {
            (vestido, piel)
        };
        for lado in [-1.0, 1.0] {
            let (rodilla, talon) = if lado == lado_patada && patada > 0.0 {
                (
                    (x + lado * (6.0 + 10.0 * patada), pies - 14.0 - 8.0 * patada),
                    (x + lado * (8.0 + 20.0 * patada), pies - 6.0 - 20.0 * patada),
                )
            } else {
                (
                    (x + s * 3.0, pies - 16.0),
                    (x + lado * (9.0 + 7.0 * s * lado), pies),
                )
            };
            let arranque = (cadera.0 + lado * 4.0, cadera.1);
            linea(l, arranque, rodilla, 4.5 + 2.0 * g, pi);
            linea(l, rodilla, talon, 4.0 + 2.0 * g, pi);
        }
        rect(l, x - 11.0 - g, y - 22.0 - g, 22.0 + 2.0 * g, 30.0 + g, v);
        rect(l, x - 14.0 - g, y + 4.0, 28.0 + 2.0 * g, 12.0 + g, v);
        circulo(l, x - 2.0, y - 32.0, 9.0 + g, pi);
        // Los brazos: enlazados con la de al lado, salvo el del hueco, que
        // sube con la mano abierta cuando se abre.
        for lado in [-1.0, 1.0] {
            let hombro = (x + lado * 9.0, y - 18.0);
            let mano = if al_hueco == Some(lado) {
                (x + lado * (22.0 - 6.0 * hueco), y - 14.0 - 34.0 * hueco)
            } else {
                (x + lado * paseo::ESPACIO_FILA * 0.5, y - 10.0)
            };
            manga(l, hombro, mano, lado * 6.0, 3.5 + 2.0 * g, pi);
        }
    });
    // Los flecos, la melena con la cinta y la pluma.
    for i in 0..7 {
        let fx = x - 12.0 + i as f32 * 4.0;
        linea(l, (fx, y + 14.0), (fx - s * 4.0, y + 22.0), 1.5, TINTA);
    }
    circulo(l, x - 3.5, y - 35.0, 9.5, TINTA);
    tri(
        l,
        (x - 10.0, y - 34.0),
        (x - 4.0, y - 26.0),
        (x + 4.0, y - 30.0),
        TINTA,
    );
    circulo(l, x - 6.5, y - 31.0, 5.0, piel);
    linea(l, (x - 11.0, y - 38.0), (x + 7.0, y - 38.0), 2.5, cinta);
    tri(
        l,
        (x + 3.0, y - 39.0),
        (x + 7.0, y - 39.0),
        (x + 10.0, y - 56.0 + patada * 3.0),
        cinta,
    );
    if hueco > 0.5
        && let Some(lado) = al_hueco
    {
        // La mano abierta, en jazz hands.
        let m = (x + lado * (22.0 - 6.0 * hueco), y - 14.0 - 34.0 * hueco);
        for d in [-4.0, 0.0, 4.0] {
            linea(l, m, (m.0 + d, m.1 - 7.0), 1.5, TINTA);
        }
    }
}

/// El contrabajista: bombin, bigote, chaleco a rayas en las dos tintas y el
/// contrabajo delante, mas alto que el. Una mano en el mastil y la otra
/// pellizcando las cuerdas; en el slap la echa atras y el contrabajo se
/// mece, y en la ultima fase lo hace girar sobre la pica, que es lo que se
/// hacia en los clubs.
fn contrabajista(
    e: &Enemigo,
    l: &Layout,
    tick: u64,
    t: f32,
    tiempo: u32,
    (mostaza, verde): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, y, d) = (e.pos.x, e.pos.y, e.dir);
    let pies = y + 48.0;
    let cabeceo = golpe_de_clave(tick, tiempo) * 3.0;
    let slap = e.gesto;
    let negro = tinte(color_u8!(30, 28, 34, 255));
    let camisa = tinte(HUESO);
    let chaleco = tinte(mostaza);
    let piel = tinte(PIEL);
    let madera = tinte(color_u8!(150, 84, 44, 255));
    let cadera = (x - d * 12.0, pies - 52.0);
    let hombros = (x - d * 12.0 + d * cabeceo * 0.3, pies - 92.0 + cabeceo);
    let cabeza = (hombros.0 + d * 2.0, hombros.1 - 17.0);

    // El, detras del contrabajo.
    con_tinta(|g, es_tinta| {
        let (ne, ca, ch, pi) = if es_tinta {
            (TINTA, TINTA, TINTA, TINTA)
        } else {
            (negro, camisa, chaleco, piel)
        };
        for lado in [-1.0, 1.0] {
            let pie = (cadera.0 + lado * 11.0, pies - 4.0);
            linea(
                l,
                (cadera.0 + lado * 6.0, cadera.1),
                pie,
                10.0 + 2.0 * g,
                ne,
            );
        }
        // La camisa y el chaleco encima.
        tri(
            l,
            (hombros.0 - 18.0 - g, hombros.1 - g),
            (hombros.0 + 18.0 + g, hombros.1 - g),
            (cadera.0 + 15.0 + g, cadera.1 + g),
            ca,
        );
        tri(
            l,
            (hombros.0 - 18.0 - g, hombros.1 - g),
            (cadera.0 + 15.0 + g, cadera.1 + g),
            (cadera.0 - 15.0 - g, cadera.1 + g),
            ca,
        );
        rect(
            l,
            cadera.0 - 14.0 - g,
            hombros.1 + 6.0 - g,
            28.0 + 2.0 * g,
            cadera.1 - hombros.1 - 4.0 + g,
            ch,
        );
        circulo(l, cabeza.0, cabeza.1, 12.0 + g, pi);
    });
    // Las rayas del chaleco, la pajarita, los zapatos y la cara.
    for i in 0..5 {
        let sx = cadera.0 - 11.0 + i as f32 * 5.5;
        linea(
            l,
            (sx, hombros.1 + 7.0),
            (sx, cadera.1 + 1.0),
            2.0,
            tinte(verde),
        );
    }
    for lado in [-1.0, 1.0] {
        tri(
            l,
            (hombros.0 + lado * 6.0, hombros.1 - 1.0),
            (hombros.0, hombros.1 + 2.0),
            (hombros.0 + lado * 6.0, hombros.1 + 5.0),
            TINTA,
        );
        rect(
            l,
            cadera.0 + lado * 11.0 - 8.0 + d * 3.0,
            pies - 5.0,
            14.0,
            5.0,
            TINTA,
        );
    }
    linea(
        l,
        (cabeza.0 + d * 3.0, cabeza.1 + 5.0),
        (cabeza.0 + d * 12.0, cabeza.1 + 6.0),
        3.0,
        TINTA,
    );
    circulo(l, cabeza.0 + d * 6.0, cabeza.1 - 2.0, 1.6, TINTA);
    // El bombin: ala corta y copa redonda, con la cinta verde.
    rect(l, cabeza.0 - 16.0, cabeza.1 - 11.0, 32.0, 4.0, TINTA);
    circulo(l, cabeza.0, cabeza.1 - 13.0, 11.0, TINTA);
    rect(l, cabeza.0 - 11.0, cabeza.1 - 13.0, 22.0, 2.5, tinte(verde));

    // El contrabajo, inclinado hacia el, sobre la pica. Se mece en el slap y
    // en la ultima fase gira: se estrecha y enseguida vuelve a abrirse.
    let pica = (x + d * 22.0, pies);
    let mece = -d * (0.12 + 0.12 * slap);
    let ancho = if e.fase() == 2 {
        0.25 + 0.75 * (t * 0.12).cos().abs()
    } else {
        1.0
    };
    let (sm, cm) = sin_cos(mece);
    let en = |dx: f32, dy: f32| {
        let dx = dx * ancho;
        (pica.0 + dx * cm - dy * sm, pica.1 + dx * sm + dy * cm)
    };
    con_tinta(|g, es_tinta| {
        let (m, ne) = if es_tinta {
            (TINTA, TINTA)
        } else {
            (madera, negro)
        };
        for (dy, r) in [(-30.0, 22.0), (-62.0, 17.0)] {
            let c = en(0.0, dy);
            let q = l.to_screen(c.0, c.1);
            draw_ellipse(
                q.x,
                q.y,
                l.len(r * ancho + g),
                l.len(r + g),
                mece * 180.0 / PI,
                m,
            );
        }
        linea(l, en(0.0, -12.0), en(0.0, -76.0), 20.0 * ancho + 2.0 * g, m);
        // El mastil y la voluta, por encima de su cabeza, y la pica.
        linea(l, en(0.0, -76.0), en(0.0, -128.0), 5.0 + 2.0 * g, ne);
        let v = en(0.0, -132.0);
        circulo(l, v.0, v.1, 5.0 + g, m);
        linea(l, en(0.0, -8.0), pica, 3.0 + g, ne);
    });
    // Las efes, el puente y las cuerdas, que tiemblan tras el slap.
    if ancho > 0.5 {
        for lado in [-1.0, 1.0] {
            linea(l, en(lado * 9.0, -40.0), en(lado * 8.0, -26.0), 1.5, TINTA);
        }
        let puente = en(0.0, -34.0);
        rect(l, puente.0 - 6.0, puente.1 - 1.0, 12.0, 3.0, TINTA);
        for k in [-3.0, -1.0, 1.0, 3.0] {
            let vibra = slap * 1.5 * (t * 1.7 + k).sin();
            linea(
                l,
                en(k * 0.8, -20.0),
                en(k * 0.5 + vibra, -126.0),
                0.8,
                fade(HUESO, 0.8),
            );
        }
    }

    // Los brazos, delante: el de atras en el mastil y el de delante en las
    // cuerdas, que en el slap se echa hacia fuera.
    let mastil = en(0.0, -100.0);
    let cuerdas = if slap > 0.0 {
        (pica.0 + d * (16.0 + 22.0 * slap), pies - 54.0 - 10.0 * slap)
    } else {
        let c = en(0.0, -48.0);
        (c.0 + d * cabeceo, c.1)
    };
    let brazos = [
        ((hombros.0 - d * 10.0, hombros.1 + 4.0), mastil, -d * 10.0),
        ((hombros.0 + d * 10.0, hombros.1 + 4.0), cuerdas, d * 8.0),
    ];
    con_tinta(|g, es_tinta| {
        let (ca, pi) = if es_tinta {
            (TINTA, TINTA)
        } else {
            (camisa, piel)
        };
        for (hombro, mano, arqueo) in brazos {
            manga(l, hombro, mano, arqueo, 6.0 + 2.0 * g, ca);
            circulo(l, mano.0, mano.1, 4.5 + g, pi);
        }
    });
    // Las rayas del golpe del slap.
    if slap > 0.2 {
        for k in [-1.0, 0.0, 1.0] {
            let a = (pica.0 + d * 30.0, pies - 54.0 + k * 10.0);
            let b = (a.0 + d * 16.0 * slap, a.1 + k * 6.0 * slap);
            linea(l, a, b, 2.0, fade(TINTA, slap));
        }
    }
}

/// Una negra del contrabajista: cabeza llena y plica, sin corchete, con un
/// rastro de dos puntos por donde viene, que es lo que dice que se tuerce.
fn negra(q: &Proyectil, l: &Layout, mostaza: Color) {
    let (x, y) = (q.pos.x, q.pos.y);
    let v = q.vel.normalize_or_zero();
    for k in 1..3 {
        let f = k as f32;
        circulo(
            l,
            x - v.x * 12.0 * f,
            y - v.y * 12.0 * f,
            4.0 - f,
            fade(PAJA, 0.5 - 0.15 * f),
        );
    }
    let c = if q.rosa { ROSA } else { mostaza };
    con_tinta(|g, es_tinta| {
        let c = if es_tinta { TINTA } else { c };
        let p = l.to_screen(x - 2.0, y + 5.0);
        draw_ellipse(p.x, p.y, l.len(7.5 + g), l.len(5.5 + g), -22.0, c);
        linea(l, (x + 4.5, y + 4.0), (x + 4.5, y - 15.0), 2.4 + 2.0 * g, c);
    });
}

/// La onda del slap: tres arcos que corren por la acera hacia donde va, con
/// polvo levantado detras. Es baja: se salta.
fn onda(q: &Proyectil, l: &Layout, t: f32) {
    let (x, y) = (q.pos.x, q.pos.y);
    let d = q.vel.x.signum();
    // El centro va a un radio por encima de la acera.
    let pies = y + 12.0;
    for k in 0..3 {
        let cx = x - d * 9.0 * k as f32;
        let r = 20.0 - 4.0 * k as f32;
        let mut antes = (cx, pies - r);
        for i in 1..=6 {
            let a = PI * 0.5 - PI * 0.5 * i as f32 / 6.0;
            let (s, c) = sin_cos(a);
            let p = (cx + d * c * r * 0.7, pies - s * r);
            linea(l, antes, p, 5.5, TINTA);
            linea(l, antes, p, 3.0, fade(LUZ, 0.95 - 0.25 * k as f32));
            antes = p;
        }
    }
    for i in 0..3 {
        let fx = x - d * (20.0 + 8.0 * i as f32);
        let fy = pies - 3.0 - (t * 0.3 + i as f32).sin().abs() * 4.0;
        circulo(l, fx, fy, 2.2, fade(HORMIGON, 0.6));
    }
}
