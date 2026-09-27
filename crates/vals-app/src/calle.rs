//! La calle: el dibujo del paseo que se anda antes de cada jefe.
//!
//! Es el mismo cartel que el salon —tinta, papel, dos tintas por baile— puesto
//! de lado. La lamina es **apaisada** (960 x 540, `Layout::para`) y la camara
//! lleva a la jugadora al 38 % de la vista: en un run-and-gun importa lo que
//! viene por delante, no lo que queda atras.
//!
//! El fondo va en capas que se mueven mas despacio cuanto mas lejos estan: el
//! cielo y la luna quietos, la silueta de Viena al 15 %, las fachadas al 45 %,
//! la balaustrada al 75 % y la calle al 100 %. **Una capa de parallax es un
//! `Layout` desplazado**, igual que la sacudida y la perspectiva de la pista:
//! ninguna funcion de dibujo se entera de que hay una camara.
//!
//! La luz es la de `escenarios::viena` —noche azul de Prusia, ventanas de arco,
//! pilastras— sacada a la calle, y los ventanales **encendidos**: desde fuera
//! un baile es eso, luz caliente en ventanas altas. Todo late con el compas
//! del vals que suena, como la sala.

use macroquad::prelude::*;
use vals_core::math::{PI, sin_cos};
use vals_core::paseo::{CADENCIA_CAMARERO, Enemigo, PIES, Paseo, SUELO_Y, Tipo, VISTA_H, VISTA_W};
use vals_core::player::{PARRY_RADIUS, SUPER_TICKS};
use vals_core::rng::Pcg32;

use crate::draw::{self, Layout, fade};
use crate::escenarios::{Instrumento, arco, circulo, linea, musico, rect, tri};
use crate::fuentes::{self, Cara};
use crate::paleta::{self, LUZ, ORO, PAPEL, TINTA};
use crate::protagonista;
use crate::skeleton;

/// Donde lleva la camara a la jugadora, como fraccion de la vista.
const ENCUADRE: f32 = 0.38;
/// La protagonista, mas grande que en el combate: alli comparte pantalla con
/// mil balas y aqui con una docena de cosas, y en una calle de 960 de ancho a
/// 1.4 seria un muneco de 40 unidades.
const TALLA: f32 = 2.0;

// La noche de Viena, de `escenarios::viena`, y lo que hace falta para la calle.
const CIELO_ALTO: Color = color_u8!(14, 16, 36, 255);
const CIELO: Color = color_u8!(20, 24, 50, 255);
const CIELO_MEDIO: Color = color_u8!(30, 38, 74, 255);
/// El horizonte, aclarado por las luces de la ciudad: es lo que deja ver la
/// silueta de Viena **mas oscura** que el cielo, que es como se ve una ciudad
/// de noche.
const CIELO_BAJO: Color = color_u8!(52, 64, 112, 255);
const LEJOS: Color = color_u8!(22, 26, 52, 255);
const PIEDRA: Color = color_u8!(38, 42, 72, 255);
const PIEDRA_CLARA: Color = color_u8!(52, 56, 92, 255);
/// Lo que se pisa va claro: una plataforma es informacion, igual que el borde
/// de la acera, y sobre un fondo oscuro tiene que saltar a la vista.
const BALCON: Color = color_u8!(172, 162, 170, 255);
const NOCHE: Color = color_u8!(36, 54, 104, 255);
const VENTANA: Color = color_u8!(236, 188, 104, 255);
const ACERA: Color = color_u8!(84, 82, 108, 255);
const ADOQUIN: Color = color_u8!(52, 48, 66, 255);
const ADOQUIN_LUZ: Color = color_u8!(66, 62, 82, 255);
const AGUA: Color = color_u8!(14, 20, 42, 255);
const MARMOL: Color = color_u8!(226, 216, 198, 255);
const MADERA: Color = color_u8!(98, 50, 40, 255);
const HUESO: Color = color_u8!(240, 232, 212, 255);
const ROSA: Color = color_u8!(238, 112, 158, 255);
const DISPARO: Color = color_u8!(248, 238, 206, 255);
const DEFEAT: Color = color_u8!(214, 78, 84, 255);
const TEXTO: Color = color_u8!(216, 200, 170, 255);
const TEXTO_TENUE: Color = color_u8!(142, 124, 102, 255);
const VELO: Color = color_u8!(20, 13, 15, 205);

/// Donde esta la camara: el borde izquierdo de la vista, en x de la calle.
pub fn camara(p: &Paseo, alpha: f32) -> f32 {
    let x = p.jugadora.render_pos(alpha).x;
    (x - VISTA_W * ENCUADRE).clamp(0.0, (p.def.largo - VISTA_W).max(0.0))
}

/// El paseo entero. `pulso` es el latido del compas, de 0 a 1, y `baile` el
/// baile al que lleva: sus dos tintas visten a las parejas, la vajilla y la
/// puerta, igual que visten a su jefe.
pub fn dibujar(p: &Paseo, alpha: f32, l: &Layout, pulso: f32, baile: usize) {
    let tintas = paleta::del_baile(baile);
    clear_background(PAPEL);
    let cam = camara(p, alpha);
    let t = p.tick as f32 + alpha;
    // Una capa de parallax es la lamina desplazada: `f` es lo que se mueve
    // con la camara, de 0 (el cielo) a 1 (la calle).
    let capa = |f: f32| l.sacudido(vec2(-cam * f, 0.0));
    let calle = capa(1.0);

    cielo(l, pulso);
    silueta_de_viena(&capa(0.15), cam * 0.15);
    fachadas(&capa(0.45), cam * 0.45, pulso);
    balaustrada(&capa(0.75), cam * 0.75);

    farolas(p, &calle, cam, pulso);
    acera(p, &calle, cam, t);
    plataformas(p, &calle);
    puerta(p, &calle, alpha, t, tintas);
    for e in &p.enemigos {
        enemigo(e, &calle, p.tick, t, tintas);
    }
    platos(p, &calle, t, tintas.0);
    disparos(p, &calle);
    dibujar_jugadora(p, alpha, t, &calle);
    parry_y_super(p, alpha, &calle);
}

/// Lo que va encima de la calle y dentro de la lamina: el papel de alrededor,
/// las cartas, el camino a la puerta y el KO. Va aparte de `dibujar` para que
/// las chispas queden entre la calle y el papel, como en el combate.
pub fn encima(p: &Paseo, l: &Layout) {
    draw::paspartu_de(l, vec2(VISTA_W, VISTA_H));
    let o = l.to_screen(0.0, 0.0);
    let w = l.len(VISTA_W);
    draw::cartas(
        l.to_screen(0.0, VISTA_H),
        p.vidas,
        p.terminado(),
        &p.jugadora,
    );
    camino(p, l);

    // El nombre del paseo arriba a la derecha, con sombra de tinta, como el
    // del baile en el combate.
    for (texto, y, tam, cara) in [
        (p.def.nombre.as_str(), 34.0, 19.0, Cara::Titulo),
        (p.def.subtitulo.as_str(), 52.0, 14.0, Cara::Cuerpo),
    ] {
        fuentes::derecha(
            texto,
            o.x + w - 8.0 + 1.6,
            o.y + y + 1.6,
            tam,
            cara,
            fade(TINTA, 0.85),
        );
        fuentes::derecha(texto, o.x + w - 8.0, o.y + y, tam, cara, TEXTO);
    }

    // Plantarse es lo unico nuevo que pide la calle, y una mecanica que no se
    // explica no existe. Se dice los primeros segundos.
    let k = 1.0 - ((p.tick as f32 - 420.0) / 60.0).clamp(0.0, 1.0);
    if k > 0.0 {
        let aviso = "SHIFT te planta  -  plantada, ARRIBA apunta arriba";
        let y = l.to_screen(0.0, VISTA_H).y - 16.0;
        fuentes::derecha(aviso, o.x + w - 12.0, y, 16.0, Cara::Cuerpo, fade(TEXTO, k));
    }

    if p.derrota {
        ko(p, l);
    }
}

// ---------------------------------------------------------------------------
// El fondo
// ---------------------------------------------------------------------------

/// El cielo, la luna y las estrellas. No se mueven con la camara: estan tan
/// lejos que moverlos seria mentir.
fn cielo(l: &Layout, pulso: f32) {
    let franjas = [CIELO_ALTO, CIELO, CIELO_MEDIO, CIELO_BAJO];
    let alto = VISTA_H / franjas.len() as f32;
    for (i, c) in franjas.into_iter().enumerate() {
        rect(l, 0.0, alto * i as f32, VISTA_W, alto + 1.0, c);
    }
    let mut rng = Pcg32::new(0x0E57_2E11A);
    for _ in 0..40 {
        let (x, y) = (rng.next_f32() * VISTA_W, rng.next_f32() * 260.0);
        let brillo = 0.4 + 0.6 * rng.next_f32();
        circulo(l, x, y, 1.2, fade(HUESO, brillo * (0.6 + 0.4 * pulso)));
    }
    let luna = color_u8!(238, 228, 196, 255);
    circulo(l, 800.0, 84.0, 46.0, fade(luna, 0.08));
    circulo(l, 800.0, 84.0, 28.0, luna);
}

/// Viena a lo lejos, en una sola tinta plana: la aguja de San Esteban, la
/// cupula de la Karlskirche con sus dos columnas, tejados y la noria del
/// Prater. Es lo que dice en que ciudad estas sin escribirlo.
///
/// `desde` es donde empieza la vista en las unidades de esta capa: el motivo
/// se repite cada `PERIODO`, y solo se pintan las copias que se ven.
fn silueta_de_viena(l: &Layout, desde: f32) {
    const PERIODO: f32 = 1700.0;
    // La base va alta para que la aguja, la cupula y la noria asomen por
    // encima de los tejados de delante; lo de abajo solo se ve por las
    // bocacalles.
    let base = 300.0;
    let primera = (desde / PERIODO).floor() as i32;
    for k in primera..=primera + 1 {
        let x0 = k as f32 * PERIODO;
        // Tejados bajos corridos, con alguna ventana diminuta.
        let mut rng = Pcg32::new(0x7E_7AD0 + k as u64);
        let mut x = x0;
        while x < x0 + PERIODO {
            let w = 60.0 + rng.next_f32() * 90.0;
            let h = 30.0 + rng.next_f32() * 50.0;
            rect(l, x, base - h, w + 1.0, h + 90.0, LEJOS);
            tri(
                l,
                (x, base - h),
                (x + w * 0.5, base - h - 16.0),
                (x + w, base - h),
                LEJOS,
            );
            if rng.next_f32() < 0.5 {
                rect(
                    l,
                    x + w * 0.4,
                    base - h + 12.0,
                    4.0,
                    5.0,
                    fade(VENTANA, 0.5),
                );
            }
            x += w;
        }
        // San Esteban: torre y aguja.
        let sx = x0 + 320.0;
        rect(l, sx - 18.0, base - 150.0, 36.0, 150.0, LEJOS);
        tri(
            l,
            (sx - 18.0, base - 150.0),
            (sx, base - 290.0),
            (sx + 18.0, base - 150.0),
            LEJOS,
        );
        // La Karlskirche: cupula sobre tambor, entre dos columnas.
        let kx = x0 + 900.0;
        rect(l, kx - 70.0, base - 90.0, 140.0, 90.0, LEJOS);
        rect(l, kx - 40.0, base - 140.0, 80.0, 50.0, LEJOS);
        circulo(l, kx, base - 140.0, 40.0, LEJOS);
        rect(l, kx - 2.0, base - 200.0, 4.0, 24.0, LEJOS);
        for dx in [-100.0, 100.0] {
            rect(l, kx + dx - 9.0, base - 170.0, 18.0, 170.0, LEJOS);
            rect(l, kx + dx - 12.0, base - 176.0, 24.0, 8.0, LEJOS);
        }
        // La noria del Prater: aro, radios y sus cabinas.
        let (nx, ny, r) = (x0 + 1380.0, base - 120.0, 90.0);
        let aro = fade(LEJOS, 1.0);
        for i in 0..16 {
            let a = i as f32 / 16.0 * 2.0 * PI;
            let (s, c) = sin_cos(a);
            linea(l, (nx, ny), (nx + c * r, ny + s * r), 1.5, aro);
            circulo(l, nx + c * r, ny + s * r, 4.0, aro);
        }
        arco_completo(l, nx, ny, r, 3.0, aro);
        tri(l, (nx, ny), (nx - 50.0, base), (nx + 50.0, base), aro);
    }
}

/// Un aro entero, que `escenarios::arco` solo hace la mitad de arriba.
fn arco_completo(l: &Layout, x: f32, y: f32, r: f32, g: f32, c: Color) {
    let p = l.to_screen(x, y);
    draw_arc(p.x, p.y, 48, l.len(r), 0.0, l.len(g).max(1.0), 360.0, c);
}

/// Los palacios del Ring: pilastras, ventanales de arco encendidos en el piso
/// noble, ventanas bajas y una cornisa. Cada cinco vanos, una bocacalle por la
/// que se ve la ciudad de fondo.
///
/// Van bajos a proposito, con el tejado a un cuarto de la vista: la primera
/// version los subia hasta arriba y la calle parecia un pasillo, sin cielo ni
/// ciudad detras. Y oscuros, porque el fondo tiene que quedarse detras de lo
/// que se juega.
fn fachadas(l: &Layout, desde: f32, pulso: f32) {
    const VANO: f32 = 190.0;
    const TECHO: f32 = 150.0;
    let primero = (desde / VANO).floor() as i32 - 1;
    for k in primero..primero + (VISTA_W / VANO) as i32 + 3 {
        if k.rem_euclid(5) == 4 {
            continue;
        }
        let x0 = k as f32 * VANO;
        let mut rng = Pcg32::new(0xFA_C4DE + k as u64);
        // Alguna chimenea, que un tejado recto de punta a punta es un muro.
        if rng.next_f32() < 0.4 {
            let cx = x0 + 30.0 + rng.next_f32() * (VANO - 60.0);
            rect(l, cx - 8.0, TECHO - 44.0, 16.0, 32.0, PIEDRA);
            rect(l, cx - 10.0, TECHO - 48.0, 20.0, 6.0, PIEDRA_CLARA);
        }
        // El muro, con atico y cornisa.
        rect(l, x0, TECHO, VANO + 1.0, SUELO_Y - TECHO, PIEDRA);
        rect(l, x0, TECHO - 12.0, VANO + 1.0, 12.0, PIEDRA_CLARA);
        linea(l, (x0, TECHO - 12.0), (x0 + VANO, TECHO - 12.0), 2.5, TINTA);
        rect(l, x0 - 4.0, TECHO + 28.0, VANO + 8.0, 8.0, PIEDRA_CLARA);
        linea(
            l,
            (x0 - 4.0, TECHO + 36.0),
            (x0 + VANO + 4.0, TECHO + 36.0),
            2.0,
            TINTA,
        );
        // La pilastra, con su capitel.
        rect(l, x0 - 11.0, TECHO, 22.0, SUELO_Y - TECHO, PIEDRA_CLARA);
        rect(l, x0 - 15.0, TECHO + 36.0, 30.0, 7.0, PIEDRA_CLARA);
        linea(l, (x0 - 11.0, TECHO), (x0 - 11.0, SUELO_Y), 2.0, TINTA);
        linea(l, (x0 + 11.0, TECHO), (x0 + 11.0, SUELO_Y), 2.0, TINTA);

        // El ventanal del piso noble: encendido casi siempre, y lo encendido
        // late con el vals que suena dentro.
        let (cx, ancho, arriba, abajo) = (x0 + VANO * 0.5, 70.0, 250.0, 340.0);
        let encendida = rng.next_f32() < 0.75;
        let cristal = if encendida {
            fade(VENTANA, 0.8 + 0.2 * pulso)
        } else {
            NOCHE
        };
        circulo(l, cx, arriba, ancho * 0.5, cristal);
        rect(l, cx - ancho * 0.5, arriba, ancho, abajo - arriba, cristal);
        if encendida && rng.next_f32() < 0.5 {
            // Una pareja bailando al otro lado del cristal, en sombra.
            let bx = cx - 10.0 + rng.next_f32() * 20.0;
            circulo(l, bx - 6.0, abajo - 42.0, 5.0, fade(TINTA, 0.55));
            circulo(l, bx + 6.0, abajo - 40.0, 5.0, fade(TINTA, 0.55));
            tri(
                l,
                (bx, abajo - 36.0),
                (bx - 15.0, abajo),
                (bx + 15.0, abajo),
                fade(TINTA, 0.55),
            );
        }
        linea(l, (cx, arriba - ancho * 0.5), (cx, abajo), 2.0, TINTA);
        linea(
            l,
            (cx - ancho * 0.5, arriba + 30.0),
            (cx + ancho * 0.5, arriba + 30.0),
            2.0,
            TINTA,
        );
        arco(l, cx, arriba, ancho * 0.5, 3.0, TINTA);
        linea(
            l,
            (cx - ancho * 0.5, arriba),
            (cx - ancho * 0.5, abajo),
            3.0,
            TINTA,
        );
        linea(
            l,
            (cx + ancho * 0.5, arriba),
            (cx + ancho * 0.5, abajo),
            3.0,
            TINTA,
        );
        // El balconcillo del ventanal.
        rect(l, cx - 48.0, abajo, 96.0, 7.0, PIEDRA_CLARA);
        linea(
            l,
            (cx - 48.0, abajo + 7.0),
            (cx + 48.0, abajo + 7.0),
            2.0,
            TINTA,
        );

        // Las ventanas bajas, pequenas y casi todas a oscuras. Asoman por
        // encima de la balaustrada.
        let baja = if rng.next_f32() < 0.3 {
            fade(VENTANA, 0.6)
        } else {
            NOCHE
        };
        rect(l, cx - 20.0, 372.0, 40.0, 50.0, baja);
        draw_rect_lines(l, cx - 20.0, 372.0, 40.0, 50.0, 2.5);
    }
}

fn draw_rect_lines(l: &Layout, x: f32, y: f32, w: f32, h: f32, g: f32) {
    let p = l.to_screen(x, y);
    draw_rectangle_lines(p.x, p.y, l.len(w), l.len(h), l.len(g).max(1.0), TINTA);
}

/// La balaustrada del paseo, entre las fachadas y la acera: una fila de
/// balaustres barrocos. Separa el fondo de la calle, que es donde se juega.
fn balaustrada(l: &Layout, desde: f32) {
    let (arriba, abajo) = (420.0, 470.0);
    let x0 = desde - 20.0;
    rect(l, x0, arriba, VISTA_W + 40.0, 8.0, PIEDRA_CLARA);
    rect(l, x0, abajo - 8.0, VISTA_W + 40.0, 8.0, PIEDRA_CLARA);
    linea(l, (x0, arriba), (x0 + VISTA_W + 40.0, arriba), 2.5, TINTA);
    linea(
        l,
        (x0, arriba + 8.0),
        (x0 + VISTA_W + 40.0, arriba + 8.0),
        1.5,
        TINTA,
    );
    const PASO: f32 = 22.0;
    let mut x = (desde / PASO).floor() * PASO;
    while x < desde + VISTA_W + PASO {
        // Un balaustre: panza abajo, cuello arriba.
        let cx = x + PASO * 0.5;
        circulo(l, cx, abajo - 18.0, 7.0, PIEDRA);
        rect(
            l,
            cx - 3.0,
            arriba + 8.0,
            6.0,
            abajo - arriba - 16.0,
            PIEDRA,
        );
        linea(
            l,
            (cx - 7.0, abajo - 16.0),
            (cx - 3.0, arriba + 10.0),
            1.2,
            fade(TINTA, 0.7),
        );
        linea(
            l,
            (cx + 7.0, abajo - 16.0),
            (cx + 3.0, arriba + 10.0),
            1.2,
            fade(TINTA, 0.7),
        );
        x += PASO;
    }
}

// ---------------------------------------------------------------------------
// La calle
// ---------------------------------------------------------------------------

/// Farolas de gas a lo largo de la acera, con su cono de luz latiendo, y algun
/// musico callejero debajo tocando el mismo vals.
fn farolas(p: &Paseo, l: &Layout, cam: f32, pulso: f32) {
    const CADA: f32 = 520.0;
    let mut x = (cam / CADA).floor() * CADA + 260.0 - CADA;
    while x < cam + VISTA_W + CADA {
        if p.def.hay_suelo(x) && x < p.def.puerta - 250.0 {
            let arriba = 290.0;
            tri(
                l,
                (x - 8.0, arriba + 12.0),
                (x - 90.0, SUELO_Y),
                (x + 90.0, SUELO_Y),
                fade(LUZ, 0.06 + 0.06 * pulso),
            );
            rect(l, x - 3.0, arriba, 6.0, SUELO_Y - arriba, TINTA);
            rect(l, x - 8.0, SUELO_Y - 14.0, 16.0, 14.0, TINTA);
            rect(l, x - 12.0, arriba - 22.0, 24.0, 26.0, TINTA);
            rect(
                l,
                x - 8.0,
                arriba - 18.0,
                16.0,
                18.0,
                fade(LUZ, 0.8 + 0.2 * pulso),
            );
            tri(
                l,
                (x - 15.0, arriba - 22.0),
                (x, arriba - 34.0),
                (x + 15.0, arriba - 22.0),
                TINTA,
            );
            // Uno de cada tres tiene musico: el vals tambien se toca en la
            // calle, y es gente, que es lo que le faltaba al fondo.
            let n = (x / CADA) as i32;
            if n % 3 == 1 {
                let que = if n % 2 == 0 {
                    Instrumento::Violin
                } else {
                    Instrumento::Violonchelo
                };
                musico(l, x + 34.0, SUELO_Y, que, pulso);
            }
        }
        x += CADA;
    }
}

/// La acera, sus adoquines y el canal en los fosos.
///
/// El foso es **agua**: es la parte de la calle que no se pisa, y el Danubio
/// es de Viena. Los bordes llevan un canto de tinta gordo, porque el borde de
/// un foso es informacion y no decorado, igual que el marco de la arena.
fn acera(p: &Paseo, l: &Layout, cam: f32, t: f32) {
    let fondo = VISTA_H + 10.0;
    // El canal, a lo ancho de la vista: la acera se pinta encima.
    rect(
        l,
        cam - 10.0,
        SUELO_Y + 16.0,
        VISTA_W + 20.0,
        fondo - SUELO_Y,
        AGUA,
    );
    for i in 0..5 {
        // Reflejos que se mecen: rayas cortas que van y vienen.
        let y = SUELO_Y + 30.0 + i as f32 * 9.0;
        let d = (t * 0.03 + i as f32).sin() * 12.0;
        let mut x = (cam / 70.0).floor() * 70.0 + (i * 23) as f32;
        while x < cam + VISTA_W + 70.0 {
            linea(l, (x + d, y), (x + d + 22.0, y), 1.5, fade(LUZ, 0.18));
            x += 70.0;
        }
    }

    for &(a, b) in &p.def.suelo {
        if b < cam - 20.0 || a > cam + VISTA_W + 20.0 {
            continue;
        }
        let (a0, b0) = (a.max(cam - 20.0), b.min(cam + VISTA_W + 20.0));
        rect(l, a0, SUELO_Y, b0 - a0, fondo - SUELO_Y, ADOQUIN);
        // Adoquines: tres filas al tresbolillo.
        for fila in 0..3 {
            let y = SUELO_Y + 20.0 + fila as f32 * 17.0;
            let desfase = if fila % 2 == 0 { 0.0 } else { 15.0 };
            let mut x = ((a0 - desfase) / 30.0).floor() * 30.0 + desfase;
            while x < b0 {
                if x >= a && x + 26.0 <= b {
                    rect(l, x + 2.0, y, 26.0, 13.0, ADOQUIN_LUZ);
                }
                x += 30.0;
            }
        }
        // El bordillo, claro, con el canto en tinta.
        rect(l, a0, SUELO_Y, b0 - a0, 14.0, ACERA);
        linea(l, (a0, SUELO_Y), (b0, SUELO_Y), 3.5, TINTA);
        linea(l, (a0, SUELO_Y + 14.0), (b0, SUELO_Y + 14.0), 1.5, TINTA);
        for x in [a, b] {
            if x > cam - 20.0 && x < cam + VISTA_W + 20.0 {
                linea(l, (x, SUELO_Y), (x, fondo), 4.0, TINTA);
            }
        }
    }
}

/// Mesas de cafe y balcones. Una mesa baja es de marmol con pie de hierro; lo
/// que esta mas alto es un balcon de piedra con sus mensulas.
fn plataformas(p: &Paseo, l: &Layout) {
    for pl in &p.def.plataformas {
        let (x, y, w) = (pl.x, pl.y, pl.ancho);
        if SUELO_Y - y < 60.0 {
            // El pie hasta la acera, y el tablero.
            let cx = x + w * 0.5;
            if p.def.hay_suelo(cx) {
                rect(l, cx - 3.0, y, 6.0, SUELO_Y - y, TINTA);
                rect(l, cx - 14.0, SUELO_Y - 4.0, 28.0, 4.0, TINTA);
            }
            rect(l, x - 2.0, y - 2.0, w + 4.0, 11.0, TINTA);
            rect(l, x, y, w, 7.0, MARMOL);
            // Una taza y su platito, que una mesa vacia no dice cafe.
            rect(l, cx + 10.0, y - 9.0, 9.0, 8.0, TINTA);
            rect(l, cx + 11.5, y - 7.5, 6.0, 6.0, HUESO);
        } else {
            rect(l, x - 3.0, y - 3.0, w + 6.0, 17.0, TINTA);
            rect(l, x, y, w, 11.0, BALCON);
            linea(l, (x, y + 4.0), (x + w, y + 4.0), 1.2, fade(TINTA, 0.6));
            let n = ((w / 45.0) as i32).max(2);
            for i in 0..n {
                let mx = x + 12.0 + (w - 24.0) * i as f32 / (n - 1) as f32;
                tri(
                    l,
                    (mx - 7.0, y + 12.0),
                    (mx + 7.0, y + 12.0),
                    (mx, y + 34.0),
                    TINTA,
                );
                tri(
                    l,
                    (mx - 4.0, y + 12.0),
                    (mx + 4.0, y + 12.0),
                    (mx, y + 28.0),
                    BALCON,
                );
            }
        }
    }
}

/// La puerta del salon, al final de la calle: un portalon con columnas, frontis
/// y el nombre del baile, que **se abre** segun te acercas. Dentro, la luz del
/// salon. Es la meta y tiene que verse desde lejos.
fn puerta(p: &Paseo, l: &Layout, alpha: f32, t: f32, (tinta, ropa): (Color, Color)) {
    let x = p.def.puerta;
    let cam = camara(p, alpha);
    if x + 400.0 < cam || x - 400.0 > cam + VISTA_W {
        return;
    }
    let (cx, ancho, arriba) = (x + 40.0, 150.0, 250.0);

    // El palacio: un bloque que cierra la calle.
    rect(l, x - 220.0, 40.0, 900.0, SUELO_Y - 40.0, PIEDRA_CLARA);
    rect(l, x - 220.0, 40.0, 900.0, 16.0, PIEDRA);
    linea(l, (x - 220.0, 56.0), (x + 680.0, 56.0), 2.5, TINTA);
    linea(l, (x - 220.0, 40.0), (x - 220.0, SUELO_Y), 4.0, TINTA);

    // Ventanales altos a los lados del portal, encendidos: el salon esta lleno.
    for wx in [x - 130.0, cx + 190.0, cx + 330.0] {
        circulo(l, wx, 170.0, 34.0, VENTANA);
        rect(l, wx - 34.0, 170.0, 68.0, 150.0, VENTANA);
        arco(l, wx, 170.0, 34.0, 3.0, TINTA);
        linea(l, (wx - 34.0, 170.0), (wx - 34.0, 320.0), 3.0, TINTA);
        linea(l, (wx + 34.0, 170.0), (wx + 34.0, 320.0), 3.0, TINTA);
        linea(l, (wx, 136.0), (wx, 320.0), 2.0, TINTA);
        linea(l, (wx - 34.0, 220.0), (wx + 34.0, 220.0), 2.0, TINTA);
        rect(l, wx - 44.0, 320.0, 88.0, 8.0, PIEDRA);
        linea(l, (wx - 44.0, 328.0), (wx + 44.0, 328.0), 2.0, TINTA);
    }

    // La luz de dentro: lo que se ve por la puerta abierta.
    let abierta = ((p.jugadora.render_pos(alpha).x - (x - 420.0)) / 320.0).clamp(0.0, 1.0);
    circulo(l, cx, arriba, ancho * 0.5, fade(LUZ, 0.95));
    rect(
        l,
        cx - ancho * 0.5,
        arriba,
        ancho,
        SUELO_Y - arriba,
        fade(LUZ, 0.95),
    );
    // Una lampara de arana al fondo, en contraluz.
    let (lx, ly) = (cx, arriba + 20.0);
    linea(l, (lx, arriba - 60.0), (lx, ly), 1.5, fade(TINTA, 0.6));
    for i in 0..5 {
        let dx = (i as f32 - 2.0) * 12.0;
        circulo(l, lx + dx, ly + 6.0 - (dx.abs() * 0.2), 3.0, fade(ORO, 0.9));
    }
    linea(
        l,
        (lx - 26.0, ly + 4.0),
        (lx + 26.0, ly + 4.0),
        2.0,
        fade(TINTA, 0.6),
    );
    // Parejas bailando dentro, en sombra, al compas.
    let giro = (t * 0.06).sin();
    for dx in [-36.0, 34.0] {
        let bx = cx + dx + giro * 6.0;
        circulo(l, bx, SUELO_Y - 58.0, 6.0, fade(TINTA, 0.5));
        tri(
            l,
            (bx, SUELO_Y - 50.0),
            (bx - 18.0, SUELO_Y),
            (bx + 18.0, SUELO_Y),
            fade(TINTA, 0.5),
        );
    }

    // Las hojas: giran sobre sus bisagras, asi que se estrechan (el mismo
    // aplastamiento que hace girar al vals).
    let hoja = ancho * 0.5 * (1.0 - abierta * 0.85);
    for (x0, dir) in [(cx - ancho * 0.5, 1.0), (cx + ancho * 0.5, -1.0)] {
        let x1 = x0 + hoja * dir;
        let (a, b) = (x0.min(x1), x0.max(x1));
        rect(l, a, arriba, b - a, SUELO_Y - arriba, MADERA);
        draw_rect_lines(l, a, arriba, b - a, SUELO_Y - arriba, 2.5);
        if b - a > 10.0 {
            draw_rect_lines(l, a + 5.0, arriba + 20.0, b - a - 10.0, 80.0, 1.5);
            draw_rect_lines(l, a + 5.0, arriba + 120.0, b - a - 10.0, 90.0, 1.5);
        }
    }
    arco(l, cx, arriba, ancho * 0.5, 5.0, TINTA);
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

    // Columnas, frontis y la cartela con el baile, en sus dos tintas.
    for dx in [-ancho * 0.5 - 40.0, ancho * 0.5 + 18.0] {
        let x0 = cx + dx;
        rect(l, x0 - 2.0, 150.0, 26.0, SUELO_Y - 150.0, TINTA);
        rect(l, x0, 152.0, 22.0, SUELO_Y - 152.0, MARMOL);
        rect(l, x0 - 6.0, 142.0, 34.0, 12.0, ORO);
        draw_rect_lines(l, x0 - 6.0, 142.0, 34.0, 12.0, 2.0);
    }
    let (fx, fw) = (cx - ancho * 0.5 - 60.0, ancho + 120.0);
    tri(
        l,
        (fx - 4.0, 142.0),
        (cx, 78.0),
        (fx + fw + 4.0, 142.0),
        TINTA,
    );
    tri(
        l,
        (fx + 6.0, 138.0),
        (cx, 86.0),
        (fx + fw - 6.0, 138.0),
        tinta,
    );
    rect(l, cx - 70.0, 150.0, 140.0, 36.0, ropa);
    draw_rect_lines(l, cx - 70.0, 150.0, 140.0, 36.0, 3.0);
    let s = l.to_screen(cx, 177.0);
    let nombre = p.def.jefe.to_uppercase();
    fuentes::centrado(&nombre, s.x, s.y, l.len(24.0), Cara::Titulo, TINTA);

    // La luz se sale a la acera segun se abre, y al llegar lo inunda: es lo
    // que dice "ya estas dentro" antes de que caiga el telon. Solo la acera y
    // el portal: lavar la pantalla entera la dejaba gris, no caliente.
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

// ---------------------------------------------------------------------------
// Los enemigos
// ---------------------------------------------------------------------------

/// Pinta una silueta dos veces: engordada en tinta y encima en su color. Es el
/// mismo contorno que `draw_figura`, para piezas sueltas. `pinta` recibe el
/// engorde y si es la pasada de tinta.
fn con_tinta(pinta: impl Fn(f32, bool)) {
    pinta(2.2, true);
    pinta(0.0, false);
}

fn enemigo(e: &Enemigo, l: &Layout, tick: u64, t: f32, tintas: (Color, Color)) {
    let edad = (tick - e.nacio) as f32 + (t - tick as f32);
    // El parpadeo del golpe: el color se va a blanco un par de ticks.
    let golpe = if e.golpe > 0 { 0.6 } else { 0.0 };
    let tinte = |c: Color| mezcla(c, WHITE, golpe);
    match e.tipo {
        Tipo::Pareja => pareja(e, l, edad, tintas, &tinte),
        Tipo::Camarero => camarero(e, l, &tinte),
        Tipo::Nota => nota(e, l, t, &tinte),
    }
}

fn mezcla(a: Color, b: Color, k: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * k,
        a.g + (b.g - a.g) * k,
        a.b + (b.b - a.b) * k,
        a.a,
    )
}

/// Una pareja de vals girando: el en frac y chistera, ella con la falda en la
/// tinta del vals. **El giro es un aplastamiento**, el mismo truco que el jefe:
/// cada uno da vueltas alrededor del otro, y el de delante se pinta el ultimo.
fn pareja(
    e: &Enemigo,
    l: &Layout,
    edad: f32,
    (azul, hueso): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let pies = e.pos.y + 38.0;
    let giro = edad * 0.11;
    let (s, c) = sin_cos(giro);
    // Los dos, a un lado y otro del eje del giro, y el que esta delante al
    // final. `c` los separa en pantalla y `s` dice quien tapa a quien.
    let el = (e.pos.x - 11.0 * c, -s);
    let ella = (e.pos.x + 11.0 * c, s);
    let ancho = 0.55 + 0.45 * c.abs();
    let mut orden = [(el, true), (ella, false)];
    if orden[0].0.1 > orden[1].0.1 {
        orden.swap(0, 1);
    }
    for ((x, _), es_el) in orden {
        if es_el {
            caballero(l, x, pies, ancho, tinte);
        } else {
            dama(l, x, pies, ancho, s, tinte(azul), tinte(hueso));
        }
    }
    // Las manos unidas en alto, por encima de los dos.
    linea(l, (el.0, pies - 58.0), (ella.0, pies - 60.0), 3.0, TINTA);
}

fn caballero(l: &Layout, x: f32, pies: f32, ancho: f32, tinte: &dyn Fn(Color) -> Color) {
    let negro = tinte(color_u8!(34, 30, 40, 255));
    let camisa = tinte(HUESO);
    con_tinta(|g, es_tinta| {
        let (frac, pechera) = if es_tinta {
            (TINTA, TINTA)
        } else {
            (negro, camisa)
        };
        let w = |v: f32| v * ancho + g;
        // Piernas, frac con faldones y cabeza con chistera.
        rect(l, x - w(6.0), pies - 30.0 - g, w(5.0), 30.0 + g, frac);
        rect(
            l,
            x + 1.0 * ancho - g,
            pies - 30.0 - g,
            w(5.0),
            30.0 + g,
            frac,
        );
        tri(
            l,
            (x - w(10.0), pies - 62.0),
            (x + w(10.0), pies - 62.0),
            (x, pies - 18.0 + g),
            frac,
        );
        rect(l, x - w(9.0), pies - 62.0 - g, 2.0 * w(9.0), 34.0 + g, frac);
        tri(
            l,
            (x - w(3.5), pies - 60.0),
            (x + w(3.5), pies - 60.0),
            (x, pies - 40.0),
            pechera,
        );
        circulo(l, x, pies - 70.0, 7.0 + g, pechera);
        rect(l, x - w(6.0), pies - 90.0 - g, 2.0 * w(6.0), 14.0 + g, frac);
        rect(
            l,
            x - w(10.0),
            pies - 78.0 - g,
            2.0 * w(10.0),
            3.0 + g * 2.0,
            frac,
        );
    });
}

fn dama(l: &Layout, x: f32, pies: f32, ancho: f32, vuelo: f32, falda: Color, piel: Color) {
    con_tinta(|g, es_tinta| {
        let (f, p) = if es_tinta {
            (TINTA, TINTA)
        } else {
            (falda, piel)
        };
        let w = |v: f32| v * ancho + g;
        // La falda de campana, con el bajo que se va hacia el lado del giro.
        let bajo = vuelo * 8.0;
        tri(
            l,
            (x, pies - 44.0 - g),
            (x - w(22.0) + bajo, pies + g),
            (x + w(22.0) + bajo, pies + g),
            f,
        );
        rect(l, x - w(22.0) + bajo, pies - 6.0, 2.0 * w(22.0), 6.0 + g, f);
        rect(l, x - w(7.0), pies - 62.0 - g, 2.0 * w(7.0), 20.0 + g, f);
        circulo(l, x, pies - 68.0, 6.5 + g, p);
        // El mono, que es lo que la hace bailarina de vals a esta distancia.
        circulo(l, x + 3.0 * ancho, pies - 76.0, 4.5 + g, TINTA);
    });
}

/// Un camarero de cafe vienes: frac, delantal largo, bigote y la bandeja en
/// alto. Echa el brazo atras justo antes de tirar, que es el aviso: se ve venir
/// el plato si se mira al camarero.
fn camarero(e: &Enemigo, l: &Layout, tinte: &dyn Fn(Color) -> Color) {
    let (x, pies, d) = (e.pos.x, e.pos.y + 36.0, e.dir);
    let negro = tinte(color_u8!(34, 30, 40, 255));
    let blanco = tinte(HUESO);
    // Cuanto tiene el brazo echado atras: sube en los ultimos 16 ticks y
    // vuelve de golpe al tirar.
    let carga = (1.0 - e.reloj as f32 / 16.0).clamp(0.0, 1.0);
    let recien = e.reloj > CADENCIA_CAMARERO - 8;
    let mano = if recien {
        (x + d * 30.0, pies - 70.0)
    } else {
        (x - d * (6.0 + 18.0 * carga), pies - 84.0 - 6.0 * carga)
    };
    con_tinta(|g, es_tinta| {
        let (frac, delantal) = if es_tinta {
            (TINTA, TINTA)
        } else {
            (negro, blanco)
        };
        rect(
            l,
            x - 7.0 - g,
            pies - 30.0 - g,
            5.0 + 2.0 * g,
            30.0 + g,
            frac,
        );
        rect(
            l,
            x + 2.0 - g,
            pies - 30.0 - g,
            5.0 + 2.0 * g,
            30.0 + g,
            frac,
        );
        rect(
            l,
            x - 10.0 - g,
            pies - 64.0 - g,
            20.0 + 2.0 * g,
            36.0 + g,
            frac,
        );
        rect(
            l,
            x - 9.0 - g,
            pies - 40.0 - g,
            18.0 + 2.0 * g,
            30.0 + g,
            delantal,
        );
        circulo(l, x, pies - 72.0, 8.0 + g, delantal);
        linea(l, (x + d * 4.0, pies - 60.0), mano, 5.0 + 2.0 * g, frac);
    });
    // Pelo engominado, bigote y pajarita, en tinta.
    rect(l, x - 8.0, pies - 82.0, 16.0, 5.0, TINTA);
    linea(
        l,
        (x + d * 2.0, pies - 69.0),
        (x + d * 9.0, pies - 67.0),
        2.0,
        TINTA,
    );
    tri(
        l,
        (x - 4.0, pies - 64.0),
        (x + 4.0, pies - 64.0),
        (x, pies - 60.0),
        TINTA,
    );
    // La bandeja con la pila de platos, en la mano de tirar. Recien tirado, la
    // pila tiene uno menos y la bandeja va vacia un momento.
    if !recien {
        let (bx, by) = mano;
        rect(l, bx - 14.0, by - 3.0, 28.0, 4.0, TINTA);
        for k in 0..3 {
            let y = by - 7.0 - k as f32 * 4.0;
            rect(l, bx - 10.0, y, 20.0, 4.0, TINTA);
            rect(l, bx - 9.0, y + 0.8, 18.0, 2.4, blanco);
        }
    }
}

/// Una corchea que vuela: cabeza, plica y corchete. Las rosas llevan el anillo
/// que late de las balas parryables, que es como se dice "parry" en este
/// juego.
fn nota(e: &Enemigo, l: &Layout, t: f32, tinte: &dyn Fn(Color) -> Color) {
    let (x, y) = (e.pos.x, e.pos.y);
    let color = tinte(if e.rosa { ROSA } else { HUESO });
    let aleteo = (t * 0.25 + x * 0.01).sin() * 4.0;
    con_tinta(|g, es_tinta| {
        let c = if es_tinta { TINTA } else { color };
        let p = l.to_screen(x - 3.0, y + 8.0);
        draw_ellipse(p.x, p.y, l.len(9.5 + g), l.len(7.0 + g), -22.0, c);
        linea(l, (x + 5.0, y + 6.0), (x + 5.0, y - 22.0), 3.0 + 2.0 * g, c);
        tri(
            l,
            (x + 5.0 - g, y - 23.0 - g),
            (x + 17.0 + g, y - 10.0 + aleteo),
            (x + 5.0 - g, y - 12.0 + g),
            c,
        );
    });
    if e.rosa {
        let k = 0.5 + 0.5 * (t * 0.2).sin();
        let p = l.to_screen(x, y);
        draw_circle_lines(
            p.x,
            p.y,
            l.len(20.0 + 4.0 * k),
            2.0,
            fade(WHITE, 0.4 + 0.4 * k),
        );
    }
}

/// Los platos, girando en el aire: una elipse que se aplasta, con el filete
/// azul de la vajilla. Los rosas, con el anillo de parry.
fn platos(p: &Paseo, l: &Layout, t: f32, azul: Color) {
    for (i, q) in p.proyectiles.iter().enumerate() {
        let s = l.to_screen(q.pos.x, q.pos.y);
        let giro = (t * 0.35 + i as f32).sin().abs().max(0.25);
        let (rx, ry) = (l.len(11.0), l.len(11.0 * giro));
        let (cara, filete) = if q.rosa { (ROSA, WHITE) } else { (HUESO, azul) };
        draw_ellipse(s.x, s.y, rx + l.len(2.2), ry + l.len(2.2), 0.0, TINTA);
        draw_ellipse(s.x, s.y, rx, ry, 0.0, filete);
        draw_ellipse(s.x, s.y, rx * 0.72, ry * 0.72, 0.0, cara);
        if q.rosa {
            let k = 0.5 + 0.5 * (t * 0.2).sin();
            draw_circle_lines(
                s.x,
                s.y,
                l.len(18.0 + 4.0 * k),
                2.0,
                fade(WHITE, 0.4 + 0.4 * k),
            );
        }
    }
}

/// Los disparos: agujas cortas en la direccion en que van, con tinta. Una
/// raya dice hacia donde va; un punto no.
fn disparos(p: &Paseo, l: &Layout) {
    for d in &p.disparos {
        let dir = d.vel.normalize_or_zero();
        let cola = d.pos - dir * 16.0;
        let (a, b) = ((cola.x, cola.y), (d.pos.x, d.pos.y));
        linea(l, a, b, 6.0, TINTA);
        linea(l, a, b, 3.0, DISPARO);
        circulo(l, d.pos.x, d.pos.y, 3.2, DISPARO);
    }
}

// ---------------------------------------------------------------------------
// La jugadora
// ---------------------------------------------------------------------------

/// La protagonista en la calle: la misma que en el combate (`protagonista.rs`),
/// con su cara y sus gestos. La pose sale de la misma `skeleton::pose`, y los
/// pies se plantan en la acera buscando la articulacion mas baja, como en la
/// pista. Todo su dibujo esta aqui y solo aqui.
fn dibujar_jugadora(p: &Paseo, alpha: f32, t: f32, l: &Layout) {
    let j = &p.jugadora;
    let pos = j.render_pos(alpha);
    let pies = l.to_screen(pos.x, pos.y + PIES);
    let pose = skeleton::pose(j, t, true, 0.0);
    let ls = l.escalado(TALLA);
    let bajo = pose.joints.iter().map(|q| q.y).fold(f32::MIN, f32::max);
    let centro = pies - vec2(0.0, bajo * ls.scale());
    // El parpadeo de los i-frames, como en el combate.
    let alfa = if j.is_invulnerable() {
        0.35 + 0.4 * ((t * 0.9).sin() * 0.5 + 0.5)
    } else {
        1.0
    };
    let gesto = protagonista::Gesto::de(j, t);
    protagonista::dibujar(&pose, &gesto, centro, &ls, alfa, false);
}

/// El alcance del parry y el fogonazo del super, encima de todo lo de la calle.
fn parry_y_super(p: &Paseo, alpha: f32, l: &Layout) {
    let j = &p.jugadora;
    let pos = j.render_pos(alpha);
    let s = l.to_screen(pos.x, pos.y);
    if j.is_parrying() {
        let r = l.len(PARRY_RADIUS);
        draw_circle(s.x, s.y, r, fade(ROSA, 0.10));
        draw_circle_lines(s.x, s.y, r, 2.0, fade(WHITE, 0.75));
    }
    if j.super_ticks > 0 {
        let k = j.super_ticks as f32 / SUPER_TICKS as f32;
        draw_circle(
            s.x,
            s.y,
            l.len(30.0 + 500.0 * (1.0 - k)),
            fade(draw::METER_FULL, k * 0.35),
        );
    }
}

// ---------------------------------------------------------------------------
// Lo de encima
// ---------------------------------------------------------------------------

/// El camino a la puerta: la calle en pequeno, arriba en el centro. Los tramos
/// de acera son trazos y los fosos huecos, asi que ademas de cuanto falta dice
/// cuantos saltos quedan. Al final, la puerta; encima, ella.
fn camino(p: &Paseo, l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let (ancho, y) = (l.len(380.0), o.y + l.len(30.0));
    let x0 = o.x + (l.len(VISTA_W) - ancho) * 0.5;
    let (desde, hasta) = (p.def.salida, p.def.puerta);
    let a_x = |x: f32| x0 + ancho * ((x - desde) / (hasta - desde)).clamp(0.0, 1.0);

    for &(a, b) in &p.def.suelo {
        let (xa, xb) = (a_x(a), a_x(b));
        if xb > xa {
            draw_line(xa, y + 1.5, xb, y + 1.5, 5.0, fade(TINTA, 0.85));
            draw_line(xa, y, xb, y, 3.0, TEXTO);
        }
    }
    // La puerta: un arco de oro.
    let px = x0 + ancho;
    draw_rectangle(px - 6.0, y - 14.0, 12.0, 14.0, ORO);
    draw_circle(px, y - 14.0, 6.0, ORO);
    draw_rectangle_lines(px - 6.0, y - 14.0, 12.0, 14.0, 1.5, TINTA);
    // Y ella: una peonza de vestido y el lazo encima, que es por lo que se la
    // reconoce desde lejos.
    let mx = a_x(p.jugadora.pos.x);
    draw_triangle(
        vec2(mx, y - 10.0),
        vec2(mx - 7.0, y + 2.0),
        vec2(mx + 7.0, y + 2.0),
        TINTA,
    );
    draw_triangle(
        vec2(mx, y - 8.0),
        vec2(mx - 5.0, y + 1.0),
        vec2(mx + 5.0, y + 1.0),
        protagonista::VESTIDO,
    );
    draw_circle(mx, y - 13.0, 4.0, TINTA);
    draw_circle(mx, y - 13.0, 2.8, protagonista::LAZO);
}

/// El KO del paseo: el mismo sello del combate, con la barra de la calle en
/// vez de la del baile. Saber si te quedaste cerca de la puerta es lo que
/// convierte un reintento en informacion.
fn ko(p: &Paseo, l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let (w, h) = (l.len(VISTA_W), l.len(VISTA_H));
    draw_rectangle(o.x, o.y, w, h, VELO);
    let cx = o.x + w * 0.5;
    let mut y = o.y + h * 0.30;

    let tam = 64.0;
    let ancho = fuentes::medir("SE ACABO", tam, Cara::Titulo).width;
    let (rw, rh) = (ancho * 0.5 + 34.0, 44.0);
    for (grosor, alfa, d) in [(4.0, 1.0, 0.0), (1.5, 0.6, 7.0)] {
        draw_rectangle_lines(
            cx - rw - d,
            y - rh - d,
            (rw + d) * 2.0,
            (rh + d) * 2.0,
            grosor,
            fade(DEFEAT, alfa),
        );
    }
    fuentes::centrado("SE ACABO", cx, y + 16.0, tam, Cara::Titulo, DEFEAT);
    y += rh + 46.0;
    fuentes::centrado("te quedaste en la calle", cx, y, 22.0, Cara::Cuerpo, TEXTO);
    y += 30.0;

    let hecho = p.progreso();
    let barra = w * 0.5;
    let x0 = cx - barra * 0.5;
    draw_rectangle(x0, y, barra, 9.0, color_u8!(52, 34, 32, 255));
    draw_rectangle(x0, y, barra * hecho, 9.0, color_u8!(206, 66, 74, 255));
    draw_rectangle_lines(x0, y, barra, 9.0, 1.5, fade(TEXTO_TENUE, 0.9));
    y += 26.0;
    let pct = (hecho * 100.0).round() as u32;
    let dicho = format!("llegaste al {pct}% del camino a la puerta");
    fuentes::centrado(&dicho, cx, y, 17.0, Cara::Cuerpo, TEXTO_TENUE);
    y += 40.0;
    for linea in ["R reintentar el paseo", "ESC volver a la pista"] {
        fuentes::centrado(linea, cx, y, 19.0, Cara::Cuerpo, TEXTO);
        y += 25.0;
    }
}
