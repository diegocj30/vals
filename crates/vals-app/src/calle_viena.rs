//! Lo que Viena trae aparte de su calle: las hojas del Prater, el husar, el
//! fiacre en su barca y el templete del director, que es su portero.
//!
//! El fondo y la acera de Viena siguen en `calle.rs`, que fue la primera calle
//! y la que hace de plantilla. Aqui va lo que gira, que es el verbo del vals:
//! **el giro es un aplastamiento**, el mismo truco de la pareja y del jefe. Lo
//! que da vueltas se estrecha y se ensancha, y el que esta delante se pinta el
//! ultimo.

use macroquad::prelude::*;
use vals_core::math::{PI, TAU};
use vals_core::paseo::{
    BATUTA, Enemigo, Forma, Paseo, Plataforma, Proyectil, SUELO_Y, VISTA_W, batuta, giro_batuta,
};

use crate::calle::{HUESO, ROSA, con_tinta};
use crate::draw::{Layout, fade};
use crate::escenarios::{arco, circulo, linea, rect, tri};
use crate::paleta::{LUZ, ORO, TINTA};

/// El otono del Prater: castano y ocre. Las rosas van en el rosa del parry.
const CASTANO: Color = color_u8!(206, 116, 52, 255);
const OCRE: Color = color_u8!(232, 170, 72, 255);
const PIEL: Color = color_u8!(242, 214, 188, 255);
const NEGRO: Color = color_u8!(34, 30, 40, 255);
/// El rojo de los pantalones del husar y la piel de su pelliza.
const GRANA: Color = color_u8!(176, 40, 52, 255);
const PELLIZA: Color = color_u8!(226, 214, 192, 255);
const MADERA: Color = color_u8!(98, 50, 40, 255);
const MADERA_CLARA: Color = color_u8!(150, 92, 60, 255);
const CABALLO: Color = color_u8!(120, 82, 60, 255);
const CONCHA: Color = color_u8!(214, 204, 186, 255);
const CONCHA_SOMBRA: Color = color_u8!(150, 140, 136, 255);
const HIERRO: Color = color_u8!(44, 42, 56, 255);

/// Medio circulo relleno, la mitad de arriba: la concha del templete.
fn media_luna(l: &Layout, x: f32, y: f32, r: f32, c: Color) {
    const N: usize = 32;
    for i in 0..N {
        let (a, b) = (PI * i as f32 / N as f32, PI * (i + 1) as f32 / N as f32);
        let p = |ang: f32| (x + ang.cos() * r, y - ang.sin() * r);
        tri(l, (x, y), p(a), p(b), c);
    }
}

/// Un punto girado alrededor de `c` y aplastado en x por `k`: lo que gira se
/// ve de lado.
fn girar(c: (f32, f32), p: (f32, f32), k: f32) -> (f32, f32) {
    (c.0 + (p.0 - c.0) * k, p.1)
}

// ---------------------------------------------------------------------------
// Los enemigos
// ---------------------------------------------------------------------------

/// Una hoja de castano de Indias: cinco foliolos en abanico desde el rabillo.
/// Da una vuelta por compas, igual que en el core, y al girar se aplasta y se
/// ladea: se ve de canto, de cara y del reves.
pub(crate) fn hoja(
    e: &Enemigo,
    l: &Layout,
    edad: f32,
    tiempo: u32,
    t: f32,
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, y) = (e.pos.x, e.pos.y);
    let giro = TAU * edad / (3 * tiempo.max(1)) as f32;
    let k = giro.cos();
    let k = k.signum() * k.abs().max(0.3);
    let ladeo = 0.45 * giro.sin();
    // El reves de la hoja es mas claro: se nota que ha dado la vuelta.
    let cara = if e.rosa {
        ROSA
    } else if k > 0.0 {
        CASTANO
    } else {
        OCRE
    };
    let color = tinte(cara);
    let base = (x, y + 11.0);
    let foliolos = [
        (-1.1, 14.0),
        (-0.55, 20.0),
        (0.0, 24.0),
        (0.55, 20.0),
        (1.1, 14.0),
    ];
    con_tinta(|g, es_tinta| {
        let c = if es_tinta { TINTA } else { color };
        for (a, largo) in foliolos {
            let a = a + ladeo;
            let (s, co) = (a.sin(), a.cos());
            let punta = (base.0 + s * (largo + g), base.1 - co * (largo + g));
            let medio = (base.0 + s * largo * 0.55, base.1 - co * largo * 0.55);
            let ancho = largo * 0.28 + g;
            let lado_a = (medio.0 + co * ancho, medio.1 + s * ancho);
            let lado_b = (medio.0 - co * ancho, medio.1 - s * ancho);
            let (b, p, la, lb) = (
                girar((x, y), base, k),
                girar((x, y), punta, k),
                girar((x, y), lado_a, k),
                girar((x, y), lado_b, k),
            );
            tri(l, b, la, p, c);
            tri(l, b, p, lb, c);
        }
        // El rabillo.
        let r = girar((x, y), (base.0 - ladeo * 6.0, base.1 + 8.0), k);
        linea(l, girar((x, y), base, k), r, 2.0 + 2.0 * g, c);
    });
    // Los nervios, en tinta, solo de cara: de canto no se ven.
    if k.abs() > 0.5 {
        for (a, largo) in foliolos {
            let a = a + ladeo;
            let punta = (
                base.0 + a.sin() * largo * 0.8,
                base.1 - a.cos() * largo * 0.8,
            );
            linea(
                l,
                girar((x, y), base, k),
                girar((x, y), punta, k),
                1.0,
                fade(TINTA, 0.7),
            );
        }
    }
    if e.rosa {
        let s = 0.5 + 0.5 * (t * 0.2).sin();
        let p = l.to_screen(x, y);
        draw_circle_lines(
            p.x,
            p.y,
            l.len(22.0 + 4.0 * s),
            2.0,
            fade(WHITE, 0.4 + 0.4 * s),
        );
    }
}

/// Un husar de gala: chaco con penacho, dolman en el azul del vals con los
/// alamares de oro, pantalon grana, botas y la pelliza colgada de un hombro.
/// Viene valsando: gira sobre si mismo (se aplasta) y en el paso largo del uno
/// se echa adelante y la pelliza vuela hacia atras.
pub(crate) fn husar(
    e: &Enemigo,
    l: &Layout,
    edad: f32,
    tiempo: u32,
    (azul, _): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, pies, d) = (e.pos.x, e.pos.y + 38.0, e.dir);
    let compas = (3 * tiempo.max(1)) as f32;
    let dentro = edad % compas;
    // El uno: de 1 a 0 a lo largo del primer tiempo.
    let carga = (1.0 - dentro / tiempo.max(1) as f32).max(0.0);
    let giro = edad * 0.14;
    let (s, c) = (giro.sin(), giro.cos());
    let ancho = 0.55 + 0.45 * c.abs();
    let inclina = d * 9.0 * carga;
    let dolman = tinte(azul);
    let grana = tinte(GRANA);
    let piel = tinte(PIEL);
    let pelliza = tinte(PELLIZA);
    let cadera = (x, pies - 30.0);
    let hombros = (x + inclina, pies - 60.0);
    let cabeza = (hombros.0 + d * 1.5, hombros.1 - 10.0);
    // Zancada en el paso largo, pies juntos en los pasitos.
    let abre = 6.0 + 12.0 * carga;
    let (delante, detras) = (x + d * abre * ancho, x - d * abre * ancho);

    // La pelliza, detras del cuerpo: vuela hacia atras y hacia el lado del
    // giro. Va antes que el para quedar detras.
    let vuelo = 12.0 + 22.0 * carga;
    let hombro = (hombros.0 - d * 6.0 * ancho, hombros.1 + 2.0);
    let bajo = (
        hombro.0 - d * vuelo + s * 8.0,
        hombro.1 + 30.0 - 8.0 * carga,
    );
    con_tinta(|g, es_tinta| {
        let col = if es_tinta { TINTA } else { pelliza };
        tri(
            l,
            (hombro.0 - g, hombro.1 - g),
            (bajo.0 - d * g, bajo.1 + g),
            (hombro.0 + d * (8.0 + g), hombro.1 + 26.0 + g),
            col,
        );
    });
    // El ribete de piel del bajo de la pelliza.
    linea(
        l,
        bajo,
        (hombro.0 + d * 8.0, hombro.1 + 26.0),
        3.0,
        fade(TINTA, 0.8),
    );

    con_tinta(|g, es_tinta| {
        let col = |v: Color| if es_tinta { TINTA } else { v };
        // Piernas grana y botas negras.
        for px in [delante, detras] {
            linea(l, cadera, (px, pies - 12.0), 7.0 + 2.0 * g, col(grana));
            linea(
                l,
                (px, pies - 14.0),
                (px + d * 2.0, pies - 2.0),
                8.0 + 2.0 * g,
                col(NEGRO),
            );
            rect(
                l,
                px - 4.0 - g,
                pies - 4.0 - g,
                10.0 + 2.0 * g,
                4.0 + 2.0 * g,
                col(NEGRO),
            );
        }
        // El dolman, ceñido, y los brazos: uno a la cadera y otro en alto,
        // que es como se lleva a la pareja que no esta.
        let w = |v: f32| v * ancho + g;
        tri(
            l,
            (hombros.0 - w(11.0), hombros.1),
            (hombros.0 + w(11.0), hombros.1),
            (cadera.0, cadera.1 + 4.0 + g),
            col(dolman),
        );
        rect(
            l,
            hombros.0 - w(10.0),
            hombros.1 - g,
            2.0 * w(10.0),
            16.0 + g,
            col(dolman),
        );
        let alto = (hombros.0 + d * 16.0 * ancho, hombros.1 - 16.0);
        linea(
            l,
            (hombros.0, hombros.1 + 3.0),
            alto,
            6.0 + 2.0 * g,
            col(dolman),
        );
        linea(
            l,
            (hombros.0, hombros.1 + 3.0),
            (hombros.0 - d * 9.0 * ancho, cadera.1 + 2.0),
            6.0 + 2.0 * g,
            col(dolman),
        );
        circulo(l, alto.0, alto.1, 3.5 + g, col(piel));
        circulo(l, cabeza.0, cabeza.1, 7.5 + g, col(piel));
        // El chaco, un poco ladeado hacia delante.
        let (sx, sy) = (cabeza.0 + d * 1.0, cabeza.1 - 6.0);
        rect(
            l,
            sx - w(7.0),
            sy - 16.0 - g,
            2.0 * w(7.0),
            16.0 + 2.0 * g,
            col(NEGRO),
        );
        rect(
            l,
            sx - w(8.5),
            sy - 2.0 - g,
            2.0 * w(8.5),
            3.0 + 2.0 * g,
            col(NEGRO),
        );
    });
    // Los alamares de oro cruzando el pecho, mas juntos cuanto mas de lado.
    for i in 0..4 {
        let y = hombros.1 + 3.0 + i as f32 * 4.0;
        let m = 7.0 * ancho - i as f32 * 0.8;
        linea(l, (hombros.0 - m, y), (hombros.0 + m, y), 1.6, ORO);
    }
    // El penacho del chaco, que tiembla con el paso, y la escarapela.
    let (sx, sy) = (cabeza.0 + d * 1.0, cabeza.1 - 22.0);
    tri(
        l,
        (sx - 3.0, sy),
        (sx + 3.0, sy),
        (sx - d * (4.0 + 6.0 * carga), sy - 16.0),
        TINTA,
    );
    tri(
        l,
        (sx - 2.0, sy - 1.0),
        (sx + 2.0, sy - 1.0),
        (sx - d * (3.0 + 6.0 * carga), sy - 14.0),
        HUESO,
    );
    circulo(l, sx, sy + 4.0, 2.4, ORO);
    // El bigote de husar, que va de oreja a oreja.
    linea(
        l,
        (cabeza.0 - 5.0 * ancho, cabeza.1 + 2.5),
        (cabeza.0 + 5.0 * ancho + d * 3.0, cabeza.1 + 1.5),
        2.2,
        TINTA,
    );
    // El sable a la cadera, oscilando con el giro.
    linea(
        l,
        (cadera.0 - d * 6.0, cadera.1),
        (cadera.0 - d * 20.0 + s * 4.0, pies - 6.0),
        2.5,
        TINTA,
    );
}

/// El director de orquesta, en frac y con la melena blanca al aire. La batuta
/// da vueltas por donde dice `giro_batuta`, asi que las notas salen de la
/// punta que se ve. Los avisos van en el dibujo:
///
/// - en el dos, la punta va cogiendo una nota rosa: en el tres la suelta;
/// - el compas antes del remolino lo pasa quieto con los dos brazos arriba y
///   un anillo que se cierra a su alrededor;
/// - por fases, cada vez mas desatado: en la segunda vuelan los faldones, en
///   la tercera gira sobre si mismo como un bailarin.
pub(crate) fn director(
    e: &Enemigo,
    l: &Layout,
    tick: u64,
    t: f32,
    tiempo: u32,
    (azul, _): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, pies, d) = (e.pos.x, e.pos.y + 48.0, e.dir);
    let fase = e.fase();
    let (golpe, dentro, prepara) = batuta(tick, tiempo, fase);
    let tiempo_f = tiempo.max(1) as f32;
    let en_tiempo = (dentro as f32 + (t - tick as f32)) / tiempo_f;
    let giro = if fase == 2 { t * 0.09 } else { 0.0 };
    let ancho = 0.6 + 0.4 * giro.cos().abs();
    let frac = tinte(NEGRO);
    let camisa = tinte(HUESO);
    let piel = tinte(PIEL);
    let pelo = tinte(color_u8!(246, 244, 236, 255));
    // El latido del tiempo: se agacha un poco en cada uno, mas en el uno.
    let latido = (1.0 - en_tiempo).max(0.0) * if golpe == 0 { 5.0 } else { 3.0 };
    let cadera = (x, pies - 36.0 + latido * 0.4);
    let hombros = (x, pies - 74.0 + latido);
    let cabeza = (x + d * 2.0 * ancho, hombros.1 - 14.0);

    // El aviso del remolino: un anillo que se cierra durante el compas quieto.
    if prepara {
        let compas = 3.0 * tiempo_f;
        let k = ((t % compas) / compas).clamp(0.0, 1.0);
        let r = 120.0 - 80.0 * k;
        let p = l.to_screen(x, e.pos.y);
        draw_circle(p.x, p.y, l.len(r), fade(azul, 0.08 + 0.10 * k));
        draw_circle_lines(p.x, p.y, l.len(r), 2.5, fade(WHITE, 0.3 + 0.5 * k));
    }

    // Los faldones del frac, detras: quietos en la primera fase, al aire
    // despues.
    let vuelo = match fase {
        0 => 4.0,
        1 => 10.0 + 6.0 * (t * 0.2).sin(),
        _ => 16.0 + 6.0 * (t * 0.3).sin(),
    };
    con_tinta(|g, es_tinta| {
        let col = if es_tinta { TINTA } else { frac };
        tri(
            l,
            (cadera.0 - 9.0 * ancho, cadera.1 - 6.0),
            (cadera.0 + 9.0 * ancho, cadera.1 - 6.0),
            (cadera.0 - d * vuelo, cadera.1 + 30.0 + g),
            col,
        );
    });

    // Los brazos: la mano de la batuta da vueltas, la otra marca el tiempo o
    // te senala en el tres. Preparando, los dos arriba.
    let mano = e.mano();
    let mano = if prepara {
        (x + d * 18.0 * ancho, hombros.1 - 34.0)
    } else {
        (mano.x, mano.y + latido)
    };
    let otra = if prepara {
        (x - d * 18.0 * ancho, hombros.1 - 34.0)
    } else if golpe == 1 {
        // En el dos te mira: la rosa va a salir.
        (x + d * 26.0 * ancho, hombros.1 + 6.0)
    } else {
        (
            x - d * 20.0 * ancho,
            hombros.1 + 10.0 - 10.0 * (1.0 - en_tiempo),
        )
    };
    let punta = if prepara {
        (mano.0, mano.1 - BATUTA)
    } else {
        let a = giro_batuta(t, tiempo);
        (mano.0 + a.cos() * BATUTA, mano.1 + a.sin() * BATUTA)
    };

    con_tinta(|g, es_tinta| {
        let col = |v: Color| if es_tinta { TINTA } else { v };
        // Las piernas, juntas en la tarima.
        for dx in [-5.0, 5.0] {
            let px = x + dx * ancho;
            linea(l, cadera, (px, pies - 3.0), 7.0 + 2.0 * g, col(frac));
            rect(
                l,
                px - 5.0 - g,
                pies - 5.0 - g,
                10.0 + 2.0 * g,
                5.0 + 2.0 * g,
                col(NEGRO),
            );
        }
        // El frac y la pechera.
        let w = |v: f32| v * ancho + g;
        rect(
            l,
            x - w(13.0),
            hombros.1 - g,
            2.0 * w(13.0),
            cadera.1 - hombros.1 + 4.0 + g,
            col(frac),
        );
        tri(
            l,
            (x - w(6.0), hombros.1),
            (x + w(6.0), hombros.1),
            (x, cadera.1 - 4.0),
            col(camisa),
        );
        for m in [mano, otra] {
            linea(l, (x, hombros.1 + 4.0), m, 7.0 + 2.0 * g, col(frac));
        }
        circulo(l, mano.0, mano.1, 4.0 + g, col(camisa));
        circulo(l, otra.0, otra.1, 4.0 + g, col(camisa));
        // La melena blanca, detras de la cabeza, cada fase mas revuelta.
        let revuelta = 1.0 + fase as f32 * 0.35;
        for (dx, dy, r) in [(7.0, -2.0, 8.0), (10.0, 5.0, 6.5), (4.0, 9.0, 5.0)] {
            circulo(
                l,
                cabeza.0 - d * dx * revuelta * ancho,
                cabeza.1 + dy * revuelta,
                r * revuelta.sqrt() + g,
                col(pelo),
            );
        }
        circulo(l, cabeza.0, cabeza.1, 9.0 + g, col(piel));
    });
    // La pajarita, el bigote y la ceja fruncida.
    let (cx, cy) = cabeza;
    tri(
        l,
        (x - 5.0, hombros.1 + 1.0),
        (x, hombros.1 + 4.0),
        (x - 5.0, hombros.1 + 7.0),
        TINTA,
    );
    tri(
        l,
        (x + 5.0, hombros.1 + 1.0),
        (x, hombros.1 + 4.0),
        (x + 5.0, hombros.1 + 7.0),
        TINTA,
    );
    linea(
        l,
        (cx + d * 1.0, cy + 4.0),
        (cx + d * 8.0, cy + 2.5),
        2.4,
        TINTA,
    );
    linea(
        l,
        (cx + d * 1.0, cy - 3.0),
        (cx + d * 7.0, cy - 1.0 - fase as f32),
        1.8,
        TINTA,
    );
    circulo(l, cx + d * 4.5, cy + 0.5, 1.4, TINTA);

    // La batuta: tinta y hueso, con la estela de la vuelta que da.
    if !prepara {
        let a = giro_batuta(t, tiempo);
        for i in 1..6 {
            let b = a - i as f32 * 0.12;
            let p = (mano.0 + b.cos() * BATUTA, mano.1 + b.sin() * BATUTA);
            circulo(l, p.0, p.1, 2.4, fade(HUESO, 0.35 - i as f32 * 0.05));
        }
    }
    linea(l, mano, punta, 4.0, TINTA);
    linea(l, mano, punta, 2.0, HUESO);
    // En el dos se va cargando la rosa en la punta; en el uno y el tres, el
    // destello del golpe.
    if golpe == 1 && !prepara {
        let k = en_tiempo.clamp(0.0, 1.0);
        circulo(l, punta.0, punta.1, 3.0 + 6.0 * k, TINTA);
        circulo(l, punta.0, punta.1, 2.0 + 5.0 * k, ROSA);
    } else if e.gesto > 0.0 {
        circulo(
            l,
            punta.0,
            punta.1,
            4.0 + 8.0 * e.gesto,
            fade(LUZ, 0.6 * e.gesto),
        );
    }
}

/// Lo que suelta la batuta. Las de la espiral, corcheas en hueso; la del tres,
/// mas gorda, rosa y con el anillo del parry; las del remolino, en el azul
/// del vals y con la estela de la curva que hacen.
pub(crate) fn nota_de_batuta(q: &Proyectil, l: &Layout, t: f32, azul: Color) {
    let (x, y) = (q.pos.x, q.pos.y);
    if q.forma == Forma::Remolino {
        let atras = -q.vel.normalize_or_zero();
        for i in 1..4 {
            let p = q.pos + atras * (i as f32 * 7.0);
            circulo(
                l,
                p.x,
                p.y,
                5.5 - i as f32,
                fade(azul, 0.5 - i as f32 * 0.12),
            );
        }
        circulo(l, x, y, 8.6, TINTA);
        circulo(l, x, y, 6.4, azul);
        circulo(l, x - 2.0, y - 2.0, 1.8, fade(WHITE, 0.8));
        return;
    }
    let (color, k) = if q.rosa { (ROSA, 1.35) } else { (HUESO, 1.0) };
    con_tinta(|g, es_tinta| {
        let c = if es_tinta { TINTA } else { color };
        let p = l.to_screen(x - 2.0 * k, y + 5.0 * k);
        draw_ellipse(p.x, p.y, l.len(6.5 * k + g), l.len(5.0 * k + g), -22.0, c);
        linea(
            l,
            (x + 3.5 * k, y + 4.0 * k),
            (x + 3.5 * k, y - 13.0 * k),
            2.4 + 2.0 * g,
            c,
        );
        tri(
            l,
            (x + 3.5 * k - g, y - 14.0 * k - g),
            (x + 11.0 * k + g, y - 5.0 * k),
            (x + 3.5 * k - g, y - 7.0 * k + g),
            c,
        );
    });
    if q.rosa {
        let s = 0.5 + 0.5 * (t * 0.2).sin();
        let p = l.to_screen(x, y);
        draw_circle_lines(
            p.x,
            p.y,
            l.len(18.0 + 4.0 * s),
            2.0,
            fade(WHITE, 0.4 + 0.4 * s),
        );
    }
}

// ---------------------------------------------------------------------------
// La calle
// ---------------------------------------------------------------------------

/// El fiacre en su barca: una gabarra de tablones a ras de muelle y, encima,
/// el coche de punto con su capota y el caballo, que marca el vals con la
/// cabeza mientras cruzan. La barca es lo que se pisa; el casco solo se pinta
/// sobre el agua, que la parte que entra bajo el muelle no se ve.
pub(crate) fn fiacre(p: &Paseo, pl: &Plataforma, l: &Layout, pulso: f32) {
    let (x, y, w) = (pl.x, pl.y, pl.ancho);
    let centro = x + w * 0.5;
    // El canal en el que esta: entre el muelle de antes y el de despues.
    let (a, b) = p
        .def
        .suelo
        .windows(2)
        .map(|par| (par[0].1, par[1].0))
        .find(|&(a, b)| centro > a && centro < b)
        .unwrap_or((x, x + w));
    let (x0, x1) = (x.max(a), (x + w).min(b));
    if x1 > x0 {
        rect(l, x0, y - 1.0, x1 - x0, 18.0, TINTA);
        rect(l, x0, y + 1.0, x1 - x0, 12.0, MADERA);
        for i in 0..((x1 - x0) / 28.0) as i32 {
            let tx = x0 + 14.0 + i as f32 * 28.0;
            linea(l, (tx, y + 1.0), (tx, y + 13.0), 1.2, fade(TINTA, 0.6));
        }
        // La cubierta, clara, que es lo que se pisa.
        rect(l, x0, y - 1.0, x1 - x0, 4.0, MADERA_CLARA);
        linea(l, (x0, y - 1.0), (x1, y - 1.0), 2.5, TINTA);
        // La estela en el agua, a los lados.
        for (lado, dx) in [(x0, -1.0), (x1, 1.0)] {
            if lado > a + 2.0 && lado < b - 2.0 {
                linea(
                    l,
                    (lado, y + 18.0),
                    (lado + dx * 18.0, y + 22.0),
                    1.5,
                    fade(LUZ, 0.3),
                );
            }
        }
    }

    // El coche: caja negra con filete de oro, capota de cuero y dos ruedas.
    let (cx, suelo) = (x + w * 0.42, y - 2.0);
    for rx in [cx - 34.0, cx + 30.0] {
        let r = if rx < cx { 15.0 } else { 12.0 };
        circulo(l, rx, suelo - r, r + 2.0, TINTA);
        circulo(l, rx, suelo - r, r - 1.5, MADERA_CLARA);
        circulo(l, rx, suelo - r, r - 4.0, TINTA);
        for i in 0..6 {
            let ang = i as f32 * TAU / 12.0;
            let (s, c) = (ang.sin() * (r - 3.0), ang.cos() * (r - 3.0));
            linea(
                l,
                (rx - c, suelo - r - s),
                (rx + c, suelo - r + s),
                1.5,
                MADERA_CLARA,
            );
        }
        circulo(l, rx, suelo - r, 3.0, ORO);
    }
    con_tinta(|g, es_tinta| {
        let col = |v: Color| if es_tinta { TINTA } else { v };
        rect(
            l,
            cx - 40.0 - g,
            suelo - 52.0 - g,
            66.0 + 2.0 * g,
            26.0 + 2.0 * g,
            col(NEGRO),
        );
        // La capota plegada, curvada hacia atras.
        tri(
            l,
            (cx - 42.0 - g, suelo - 52.0),
            (cx - 4.0 + g, suelo - 52.0),
            (cx - 36.0, suelo - 80.0 - g),
            col(NEGRO),
        );
        // El pescante del cochero.
        rect(
            l,
            cx + 22.0 - g,
            suelo - 60.0 - g,
            16.0 + 2.0 * g,
            6.0 + 2.0 * g,
            col(NEGRO),
        );
    });
    linea(
        l,
        (cx - 38.0, suelo - 46.0),
        (cx + 24.0, suelo - 46.0),
        1.6,
        ORO,
    );
    rect(l, cx - 24.0, suelo - 50.0, 14.0, 10.0, fade(LUZ, 0.5));
    // Un farol a cada lado del pescante, encendido.
    circulo(l, cx + 28.0, suelo - 66.0, 3.5, TINTA);
    circulo(
        l,
        cx + 28.0,
        suelo - 66.0,
        2.2,
        fade(LUZ, 0.7 + 0.3 * pulso),
    );

    // El caballo, delante, enganchado con las varas.
    let hx = x + w - 34.0;
    let cabeceo = 3.0 * pulso;
    linea(
        l,
        (cx + 30.0, suelo - 34.0),
        (hx - 8.0, suelo - 36.0),
        2.0,
        TINTA,
    );
    con_tinta(|g, es_tinta| {
        let col = |v: Color| if es_tinta { TINTA } else { v };
        // Patas: dos plantadas y una levantada al compas.
        for (dx, alza) in [
            (-14.0, 0.0),
            (-8.0, 6.0 * pulso),
            (10.0, 0.0),
            (16.0, 6.0 * pulso),
        ] {
            linea(
                l,
                (hx + dx, suelo - 30.0),
                (hx + dx + alza * 0.4, suelo - 2.0 - alza),
                4.0 + 2.0 * g,
                col(CABALLO),
            );
        }
        let p = l.to_screen(hx, suelo - 38.0);
        draw_ellipse(
            p.x,
            p.y,
            l.len(22.0 + g),
            l.len(11.0 + g),
            0.0,
            col(CABALLO),
        );
        // Cuello y cabeza, que cabecean con el pulso del vals.
        linea(
            l,
            (hx + 14.0, suelo - 42.0),
            (hx + 22.0, suelo - 60.0 + cabeceo),
            9.0 + 2.0 * g,
            col(CABALLO),
        );
        let q = l.to_screen(hx + 27.0, suelo - 58.0 + cabeceo);
        draw_ellipse(q.x, q.y, l.len(9.0 + g), l.len(5.0 + g), 30.0, col(CABALLO));
    });
    // Crin, cola y el penacho del arnes.
    linea(
        l,
        (hx + 14.0, suelo - 50.0),
        (hx + 20.0, suelo - 64.0 + cabeceo),
        3.0,
        TINTA,
    );
    linea(
        l,
        (hx - 21.0, suelo - 40.0),
        (hx - 27.0, suelo - 22.0),
        3.0,
        TINTA,
    );
    circulo(l, hx + 21.0, suelo - 68.0 + cabeceo, 3.0, GRANA);
}

/// El templete del Stadtpark donde toca el director: la concha de la orquesta
/// detras, con sus costillas y el filete de oro, la tarima y el atril. Cuando
/// la arena esta cerrada, las verjas del parque cierran los dos lados.
pub(crate) fn templete(p: &Paseo, l: &Layout, pulso: f32) {
    let Some(po) = p.def.porteria else {
        return;
    };
    let Some(a) = p.def.enemigos.iter().find(|a| a.portero) else {
        return;
    };
    let (cx, tarima) = (a.x, if a.y != 0.0 { a.y } else { SUELO_Y });
    let base = SUELO_Y;
    // La concha: medio circulo grande, con las costillas en abanico.
    let r = 190.0;
    media_luna(l, cx, base, r + 6.0, TINTA);
    media_luna(l, cx, base, r, CONCHA);
    for i in 1..8 {
        let ang = PI * i as f32 / 8.0;
        let (s, c) = (ang.sin(), ang.cos());
        linea(
            l,
            (cx + c * 40.0, base - s * 40.0),
            (cx + c * r, base - s * r),
            2.0,
            CONCHA_SOMBRA,
        );
    }
    for rr in [r * 0.72, r * 0.42] {
        arco(l, cx, base, rr, 2.0, CONCHA_SOMBRA);
    }
    arco(l, cx, base, r - 6.0, 3.0, fade(ORO, 0.8 + 0.2 * pulso));
    // El hueco de dentro, en sombra, donde estaria la orquesta: unas sillas.
    media_luna(l, cx, base, 40.0, fade(TINTA, 0.25));
    for dx in [-120.0, -80.0, 80.0, 120.0] {
        let sx = cx + dx;
        rect(l, sx - 6.0, base - 22.0, 12.0, 3.0, fade(TINTA, 0.6));
        linea(
            l,
            (sx - 5.0, base - 22.0),
            (sx - 5.0, base),
            1.5,
            fade(TINTA, 0.6),
        );
        linea(
            l,
            (sx + 5.0, base - 40.0),
            (sx + 5.0, base),
            1.5,
            fade(TINTA, 0.6),
        );
    }
    // La tarima, de madera con su canto de tinta, y el atril.
    let alto = base - tarima;
    rect(l, cx - 52.0, tarima - 2.0, 104.0, alto + 2.0, TINTA);
    rect(l, cx - 50.0, tarima, 100.0, alto, MADERA_CLARA);
    linea(l, (cx - 52.0, tarima), (cx + 52.0, tarima), 2.5, TINTA);
    let ax = cx - 40.0;
    linea(l, (ax, tarima), (ax, tarima - 46.0), 2.5, HIERRO);
    tri(
        l,
        (ax - 14.0, tarima - 44.0),
        (ax + 14.0, tarima - 52.0),
        (ax + 14.0, tarima - 44.0),
        HIERRO,
    );
    rect(l, ax - 10.0, tarima - 62.0, 20.0, 14.0, HUESO);
    for i in 0..3 {
        let y = tarima - 59.0 + i as f32 * 4.0;
        linea(l, (ax - 8.0, y), (ax + 8.0, y), 0.8, fade(TINTA, 0.7));
    }

    // Las verjas, mientras la arena esta cerrada: hierro forjado con puntas
    // de oro que bajan a los dos bordes de la vista.
    if p.cierre().is_some() {
        for x in [po.desde, po.desde + VISTA_W] {
            let dentro = if x > po.desde { -1.0 } else { 1.0 };
            let x0 = x + dentro * 4.0;
            rect(l, x0 - 4.0, 300.0, 8.0, base - 300.0, HIERRO);
            for i in 0..4 {
                let bx = x0 + dentro * (10.0 + i as f32 * 12.0);
                let arriba = 330.0 + i as f32 * 10.0;
                linea(l, (bx, arriba), (bx, base), 3.0, HIERRO);
                tri(
                    l,
                    (bx - 4.0, arriba),
                    (bx + 4.0, arriba),
                    (bx, arriba - 10.0),
                    ORO,
                );
            }
            linea(l, (x0, 360.0), (x0 + dentro * 50.0, 380.0), 3.0, HIERRO);
            linea(l, (x0, 430.0), (x0 + dentro * 50.0, 430.0), 3.0, HIERRO);
        }
    }
}
