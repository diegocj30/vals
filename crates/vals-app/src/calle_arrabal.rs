//! El arrabal: la calle que se anda antes de El Tango.
//!
//! La Boca al caer la noche, con la luz de `escenarios::arrabal` —cielo de
//! ciruela, luna enorme, farolas— sacada a la calle. No es Viena con otro
//! color: donde alli hay piedra y ventanales, aqui hay **chapa pintada**. Los
//! conventillos del barrio se pintaban con lo que sobraba en los astilleros,
//! cada pared de un color, y eso es lo que dice "La Boca" sin escribirlo. Van
//! apagados por la hora, para que se queden detras de lo que se juega.
//!
//! Las capas, de lejos a cerca: el cielo quieto; el puerto al 12 % (gruas,
//! chimeneas de barco y el Puente Transbordador, que es a La Boca lo que la
//! aguja de San Esteban a Viena); los conventillos al 40 %, con la ropa
//! tendida meciendose al compas; la calle del tranvia al 70 %, con su tranvia
//! pasando; y las farolas con el bandoneon en la acera, al 100 %.
//!
//! Los fosos son el Riachuelo: el agua es la misma de `calle::acera`, en otro
//! color, y lo que aqui cambia es lo que flota encima (botes y cajones, en
//! `plataformas`).
//!
//! Los enemigos llevan el verbo del jefe, la **aceleracion negativa**, y el
//! dibujo lo tiene que contar: el compadrito se echa atras antes de lanzarse
//! (el aviso), sale con rayas de velocidad y frena levantando polvo; la rosa
//! vuela girando, se queda temblando en el aire y cae mustia.

use macroquad::prelude::*;
use vals_core::math::{PI, sin_cos};
use vals_core::paseo::{
    AMAGO_COMPADRITO, CADENCIA_FLORISTA, COLGADA_ROSA, ESTOCADA_COMPADRITO, Enemigo,
    PLANTADO_COMPADRITO, Paseo, SUELO_Y, VISTA_W, VUELO_ROSA,
};
use vals_core::rng::Pcg32;

use crate::calle::{HUESO, ROSA, VENTANA, acera, camara, con_tinta, draw_rect_lines, mezcla};
use crate::draw::{Layout, fade};
use crate::escenarios::{Instrumento, arco, circulo, linea, musico, rect, tri};
use crate::fuentes::{self, Cara};
use crate::paleta::{LUZ, ORO, TINTA};

/// El agua del Riachuelo: verde de aceite, mas turbia que el canal de Viena.
const RIACHUELO: Color = color_u8!(24, 34, 32, 255);

// El atardecer de `escenarios::arrabal`, alargado hacia el horizonte.
const CIELO: [Color; 6] = [
    color_u8!(26, 16, 34, 255),
    color_u8!(36, 20, 40, 255),
    color_u8!(50, 27, 46, 255),
    color_u8!(70, 36, 52, 255),
    color_u8!(98, 48, 58, 255),
    color_u8!(128, 64, 62, 255),
];
const LUNA: Color = color_u8!(240, 224, 186, 255);
/// El puerto, en una tinta: mas oscuro que el horizonte, que es como se ve una
/// grua contra el cielo del atardecer.
const PUERTO: Color = color_u8!(40, 22, 36, 255);
const HUMO: Color = color_u8!(84, 52, 64, 255);
/// Los colores de la chapa, apagados por la hora: rojo, ocre, azul, verde y
/// un naranja de minio. Son los de La Boca, bajados de brillo y de saturacion
/// para que ningun conventillo compita con un enemigo.
const CHAPAS: [Color; 5] = [
    color_u8!(112, 44, 48, 255),
    color_u8!(138, 108, 54, 255),
    color_u8!(46, 66, 98, 255),
    color_u8!(50, 84, 70, 255),
    color_u8!(128, 70, 46, 255),
];
const TEJADO: Color = color_u8!(72, 44, 44, 255);
const PERSIANA: Color = color_u8!(40, 70, 58, 255);
const NOCHE: Color = color_u8!(30, 22, 38, 255);
const ADOQUIN: Color = color_u8!(48, 36, 46, 255);
const MURO: Color = color_u8!(40, 30, 38, 255);
const RIEL: Color = color_u8!(150, 138, 140, 255);
/// El tranvia: bordo abajo y crema arriba, como los de la Anglo.
const TRANVIA: Color = color_u8!(122, 34, 42, 255);
const TRANVIA_CREMA: Color = color_u8!(214, 196, 158, 255);
/// Lo que se pisa va claro, igual que en Viena: madera pintada de cal.
const TABLA: Color = color_u8!(214, 194, 156, 255);
const TABLA_SOMBRA: Color = color_u8!(150, 120, 92, 255);
const CAJON: Color = color_u8!(150, 104, 62, 255);
const PIEL: Color = color_u8!(232, 196, 164, 255);
const VERDE: Color = color_u8!(62, 120, 64, 255);

/// El fondo del arrabal, de lo mas lejano a las farolas de la acera.
pub(crate) fn fondo(
    p: &Paseo,
    l: &Layout,
    capa: &dyn Fn(f32) -> Layout,
    cam: f32,
    pulso: f32,
    t: f32,
) {
    cielo(l, pulso);
    puerto(&capa(0.12), cam * 0.12, t);
    conventillos(&capa(0.4), cam * 0.4, pulso, t);
    calle_del_tranvia(&capa(0.7), cam * 0.7, pulso, t);
    farolas(p, &capa(1.0), cam, pulso);
}

/// Lo que se pisa: la acera de Viena con el Riachuelo en los fosos, lo que
/// flota o se apila encima y la puerta de la milonga.
pub(crate) fn calle(
    p: &Paseo,
    l: &Layout,
    alpha: f32,
    cam: f32,
    t: f32,
    pulso: f32,
    tintas: (Color, Color),
) {
    acera(p, l, cam, t, RIACHUELO);
    plataformas(p, l, tintas);
    puerta(p, l, alpha, pulso, tintas);
}

// ---------------------------------------------------------------------------
// El fondo
// ---------------------------------------------------------------------------

/// El cielo del atardecer y la luna. Quietos: estan tan
/// lejos que moverlos seria mentir.
fn cielo(l: &Layout, pulso: f32) {
    let alto = 400.0 / CIELO.len() as f32;
    for (i, c) in CIELO.into_iter().enumerate() {
        rect(l, 0.0, alto * i as f32, VISTA_W, alto + 1.0, c);
    }
    rect(l, 0.0, 400.0, VISTA_W, 140.0, CIELO[5]);
    let mut rng = Pcg32::new(0xB0CA_7A60);
    for _ in 0..26 {
        let (x, y) = (rng.next_f32() * VISTA_W, rng.next_f32() * 150.0);
        let brillo = 0.3 + 0.6 * rng.next_f32();
        circulo(l, x, y, 1.1, fade(LUNA, brillo * (0.6 + 0.4 * pulso)));
    }
    // Una luna enorme, que es la mitad de un tango.
    circulo(l, 250.0, 120.0, 70.0, fade(LUNA, 0.07));
    circulo(l, 250.0, 120.0, 48.0, LUNA);
}

/// El puerto, en una sola tinta: galpones de diente de sierra, gruas, un
/// carguero con sus chimeneas echando humo y el Puente Transbordador, con la
/// barquilla colgada yendo y viniendo de orilla a orilla.
fn puerto(l: &Layout, desde: f32, t: f32) {
    const PERIODO: f32 = 1600.0;
    let base = 330.0;
    let primera = (desde / PERIODO).floor() as i32;
    for k in primera..=primera + 1 {
        let x0 = k as f32 * PERIODO;
        // Galpones: tejado de diente de sierra, corrido.
        let mut x = x0;
        let mut rng = Pcg32::new(0x6A1_90E5 + k as u64);
        while x < x0 + PERIODO {
            let w = 90.0 + rng.next_f32() * 110.0;
            let h = 26.0 + rng.next_f32() * 30.0;
            rect(l, x, base - h, w + 1.0, h + 120.0, PUERTO);
            let dientes = (w / 30.0) as i32;
            for d in 0..dientes {
                let dx = x + d as f32 * w / dientes as f32;
                tri(
                    l,
                    (dx, base - h),
                    (dx, base - h - 12.0),
                    (dx + w / dientes as f32, base - h),
                    PUERTO,
                );
            }
            x += w;
        }

        // El carguero: casco, puente y dos chimeneas con su franja.
        let bx = x0 + 120.0;
        rect(l, bx, base - 46.0, 380.0, 30.0, PUERTO);
        tri(
            l,
            (bx + 380.0, base - 46.0),
            (bx + 420.0, base - 46.0),
            (bx + 380.0, base - 16.0),
            PUERTO,
        );
        rect(l, bx + 60.0, base - 76.0, 120.0, 30.0, PUERTO);
        for (i, cx) in [bx + 220.0, bx + 290.0].into_iter().enumerate() {
            rect(l, cx - 14.0, base - 128.0, 28.0, 82.0, PUERTO);
            rect(l, cx - 14.0, base - 118.0, 28.0, 9.0, fade(CIELO[4], 0.9));
            // El humo sube y se va a la izquierda, bocanada a bocanada.
            for n in 0..5 {
                let f = ((t * 0.004 + n as f32 * 0.2 + i as f32 * 0.1) % 1.0 + 1.0) % 1.0;
                circulo(
                    l,
                    cx - 70.0 * f,
                    base - 136.0 - 70.0 * f,
                    6.0 + 12.0 * f,
                    fade(HUMO, 0.45 * (1.0 - f)),
                );
            }
        }

        // Las gruas: torre, pluma inclinada y el cable con el gancho.
        for (gx, alto, largo) in [(x0 + 640.0, 170.0, 130.0), (x0 + 1440.0, 150.0, 110.0)] {
            rect(l, gx - 6.0, base - alto, 12.0, alto, PUERTO);
            linea(l, (gx - 20.0, base), (gx, base - alto * 0.6), 4.0, PUERTO);
            linea(l, (gx + 20.0, base), (gx, base - alto * 0.6), 4.0, PUERTO);
            let punta = (gx + largo, base - alto - 50.0);
            linea(l, (gx, base - alto + 10.0), punta, 6.0, PUERTO);
            linea(
                l,
                (gx - 30.0, base - alto + 14.0),
                (gx, base - alto),
                5.0,
                PUERTO,
            );
            rect(l, gx - 12.0, base - alto - 6.0, 24.0, 18.0, PUERTO);
            linea(l, punta, (punta.0, base - 80.0), 1.5, PUERTO);
            rect(l, punta.0 - 4.0, base - 82.0, 8.0, 8.0, PUERTO);
        }

        // El Transbordador: dos torres de celosia, la viga arriba y la
        // barquilla colgada, que cruza el Riachuelo sin parar.
        let (ta, tb, arriba) = (x0 + 880.0, x0 + 1220.0, base - 250.0);
        for tx in [ta, tb] {
            tri(
                l,
                (tx - 30.0, base),
                (tx - 8.0, arriba),
                (tx + 8.0, arriba),
                PUERTO,
            );
            tri(
                l,
                (tx + 30.0, base),
                (tx - 8.0, arriba),
                (tx + 8.0, arriba),
                PUERTO,
            );
            tri(
                l,
                (tx - 30.0, base),
                (tx + 30.0, base),
                (tx, arriba + 40.0),
                PUERTO,
            );
        }
        rect(l, ta - 40.0, arriba - 22.0, tb - ta + 80.0, 22.0, PUERTO);
        // La celosia, en el color del cielo: es lo que la hace de hierro.
        let mut x = ta - 36.0;
        while x < tb + 36.0 {
            linea(
                l,
                (x, arriba - 18.0),
                (x + 14.0, arriba - 4.0),
                1.5,
                CIELO[1],
            );
            linea(
                l,
                (x + 14.0, arriba - 18.0),
                (x, arriba - 4.0),
                1.5,
                CIELO[1],
            );
            x += 18.0;
        }
        let ida = 0.5 - 0.5 * (t * 0.004 + k as f32).cos();
        let gx = ta + 20.0 + (tb - ta - 40.0) * ida;
        linea(
            l,
            (gx - 12.0, arriba),
            (gx - 12.0, base - 60.0),
            1.2,
            PUERTO,
        );
        linea(
            l,
            (gx + 12.0, arriba),
            (gx + 12.0, base - 60.0),
            1.2,
            PUERTO,
        );
        rect(l, gx - 26.0, base - 62.0, 52.0, 14.0, PUERTO);
    }
}

/// Los conventillos: casas de dos pisos de chapa acanalada, cada una de un
/// color, con persianas, balcones de hierro y la ropa tendida de una a otra.
///
/// Bajos a proposito, con el tejado por debajo de la mitad de la vista, igual
/// que las fachadas de Viena: tiene que quedar cielo para el puerto.
fn conventillos(l: &Layout, desde: f32, pulso: f32, t: f32) {
    const CASA: f32 = 210.0;
    let primera = (desde / CASA).floor() as i32 - 1;
    let ultima = primera + (VISTA_W / CASA) as i32 + 3;
    // Donde cuelga la ropa, de una casa a la siguiente.
    let mut cuerdas = Vec::with_capacity(8);
    for k in primera..ultima {
        let x0 = k as f32 * CASA;
        let mut rng = Pcg32::new(0x00B0_CAC0 + k as u64);
        // De dos en dos por la lista: dos vecinas nunca son del mismo color.
        let color = CHAPAS[(k.rem_euclid(5) * 2 % 5) as usize];
        let techo = 262.0 + rng.next_f32() * 36.0;
        let w = CASA - 14.0 - rng.next_f32() * 20.0;

        // El tejado de chapa, a un agua, y el cano de la estufa.
        tri(
            l,
            (x0 - 6.0, techo),
            (x0 + w + 6.0, techo),
            (x0 + w + 6.0, techo - 20.0),
            TEJADO,
        );
        linea(
            l,
            (x0 - 6.0, techo),
            (x0 + w + 6.0, techo - 20.0),
            2.5,
            TINTA,
        );
        if rng.next_f32() < 0.6 {
            let cx = x0 + 30.0 + rng.next_f32() * (w - 60.0);
            rect(l, cx - 4.0, techo - 44.0, 8.0, 40.0, TINTA);
            rect(l, cx - 7.0, techo - 48.0, 14.0, 5.0, TINTA);
        }

        // La pared: el color y las ondas de la chapa, en tinta aguada.
        rect(l, x0, techo, w, SUELO_Y - techo, color);
        let mut x = x0 + 5.0;
        while x < x0 + w {
            linea(l, (x, techo), (x, SUELO_Y), 1.0, fade(TINTA, 0.28));
            x += 7.0;
        }
        linea(l, (x0, techo), (x0, SUELO_Y), 2.5, TINTA);
        linea(l, (x0 + w, techo), (x0 + w, SUELO_Y), 2.5, TINTA);
        // La linea del forjado entre los dos pisos.
        let piso = techo + (SUELO_Y - techo) * 0.48;
        rect(l, x0 - 3.0, piso - 3.0, w + 6.0, 6.0, fade(TINTA, 0.8));

        // Arriba, dos ventanas con persianas; una con balcon de hierro.
        let arriba = techo + 22.0;
        for (i, vx) in [x0 + w * 0.28, x0 + w * 0.72].into_iter().enumerate() {
            let encendida = rng.next_f32() < 0.55;
            let (vw, vh) = (30.0, 44.0);
            let cristal = if encendida {
                fade(VENTANA, 0.75 + 0.25 * pulso)
            } else {
                NOCHE
            };
            rect(l, vx - vw * 0.5, arriba, vw, vh, cristal);
            // Las hojas de la persiana, abiertas hacia fuera.
            for lado in [-1.0, 1.0] {
                let px = vx + lado * (vw * 0.5 + 7.0);
                rect(l, px - 7.0, arriba, 14.0, vh, PERSIANA);
                for f in 1..5 {
                    let y = arriba + f as f32 * vh / 5.0;
                    linea(l, (px - 7.0, y), (px + 7.0, y), 1.0, fade(TINTA, 0.6));
                }
                draw_rect_lines(l, px - 7.0, arriba, 14.0, vh, 1.5);
            }
            draw_rect_lines(l, vx - vw * 0.5, arriba, vw, vh, 2.0);
            if i == 1 && encendida && rng.next_f32() < 0.6 {
                // Alguien asomado, en sombra.
                circulo(l, vx, arriba + 18.0, 6.0, fade(TINTA, 0.7));
                rect(l, vx - 8.0, arriba + 24.0, 16.0, 20.0, fade(TINTA, 0.7));
            }
            if i == 0 {
                // El balconcito: losa y barandilla.
                let by = arriba + vh;
                rect(l, vx - 30.0, by, 60.0, 5.0, TINTA);
                linea(
                    l,
                    (vx - 30.0, by - 16.0),
                    (vx + 30.0, by - 16.0),
                    1.8,
                    TINTA,
                );
                let mut bx = vx - 28.0;
                while bx <= vx + 28.0 {
                    linea(l, (bx, by - 16.0), (bx, by), 1.2, TINTA);
                    bx += 7.0;
                }
            }
            if i == 1 {
                cuerdas.push((vx + 22.0, arriba + 6.0));
            }
        }
        // Abajo, la puerta del conventillo y una ventana baja. Asoman por
        // encima de la calle del tranvia.
        let px = x0 + w * 0.5;
        rect(
            l,
            px - 16.0,
            piso + 30.0,
            32.0,
            SUELO_Y - piso - 30.0,
            NOCHE,
        );
        arco(l, px, piso + 30.0, 16.0, 2.0, TINTA);
        circulo(l, px, piso + 30.0, 16.0, NOCHE);
        draw_rect_lines(l, px - 16.0, piso + 30.0, 32.0, SUELO_Y - piso - 30.0, 2.0);
    }

    // La ropa tendida, de una casa a la siguiente: la cuerda cae en curva y la
    // ropa se mece con el viento y salta un poco en cada tiempo.
    for par in cuerdas.windows(2) {
        let ((ax, ay), (bx, by)) = (par[0], par[1]);
        let (cx, cy) = ((ax + bx) * 0.5, (ay + by) * 0.5 + 22.0);
        let punto = |f: f32| {
            let u = 1.0 - f;
            (
                u * u * ax + 2.0 * u * f * cx + f * f * bx,
                u * u * ay + 2.0 * u * f * cy + f * f * by,
            )
        };
        let mut antes = punto(0.0);
        for n in 1..=12 {
            let ahora = punto(n as f32 / 12.0);
            linea(l, antes, ahora, 1.2, TINTA);
            antes = ahora;
        }
        let mut rng = Pcg32::new(ax.to_bits() as u64);
        for n in 1..5 {
            let (x, y) = punto(n as f32 / 5.0);
            let mece = (t * 0.05 + x * 0.03).sin() * 3.0 + pulso * 2.0;
            let c = CHAPAS[(rng.next_u32() % CHAPAS.len() as u32) as usize];
            let prenda = mezcla(c, HUESO, 0.45);
            if rng.next_f32() < 0.5 {
                // Una camisa: cuerpo y mangas.
                rect(l, x - 7.0 + mece * 0.3, y, 14.0, 18.0, prenda);
                tri(
                    l,
                    (x - 7.0, y),
                    (x - 13.0, y + 8.0),
                    (x - 7.0, y + 8.0),
                    prenda,
                );
                tri(
                    l,
                    (x + 7.0, y),
                    (x + 13.0, y + 8.0),
                    (x + 7.0, y + 8.0),
                    prenda,
                );
            } else {
                // Una sabana, que es la que mas se mueve.
                tri(
                    l,
                    (x - 9.0, y),
                    (x + 9.0, y),
                    (x + 9.0 + mece, y + 26.0),
                    prenda,
                );
                tri(
                    l,
                    (x - 9.0, y),
                    (x + 9.0 + mece, y + 26.0),
                    (x - 9.0 + mece, y + 26.0),
                    prenda,
                );
            }
            linea(l, (x - 3.0, y - 2.0), (x - 3.0, y + 2.0), 2.0, TINTA);
        }
    }
}

/// La calle del tranvia, entre los conventillos y la acera: adoquines, los
/// rieles, los postes del cable y, cada tanto, el tranvia que pasa con las
/// ventanillas encendidas.
fn calle_del_tranvia(l: &Layout, desde: f32, pulso: f32, t: f32) {
    let (x0, x1) = (desde - 40.0, desde + VISTA_W + 40.0);
    let calzada = 440.0;
    rect(l, x0, calzada, x1 - x0, SUELO_Y - calzada, ADOQUIN);
    linea(l, (x0, calzada), (x1, calzada), 2.0, TINTA);
    // Debajo de la calzada, el muro del muelle: es lo que se ve por encima del
    // agua en los fosos, y lo que hace que un foso sea la orilla del Riachuelo.
    rect(l, x0, SUELO_Y, x1 - x0, 80.0, MURO);
    linea(l, (x0, SUELO_Y), (x1, SUELO_Y), 2.0, TINTA);
    let mut bx = (desde / 60.0).floor() * 60.0;
    while bx < x1 {
        linea(
            l,
            (bx, SUELO_Y + 2.0),
            (bx, SUELO_Y + 16.0),
            1.2,
            fade(TINTA, 0.6),
        );
        bx += 60.0;
    }
    for y in [452.0, 462.0] {
        linea(l, (x0, y), (x1, y), 2.2, RIEL);
    }
    // Los postes, con el cable arriba colgando en curva de uno a otro.
    const POSTE: f32 = 340.0;
    let cable = 300.0;
    let mut x = (desde / POSTE).floor() * POSTE - POSTE;
    while x < x1 + POSTE {
        rect(l, x - 3.0, cable - 14.0, 6.0, calzada - cable + 14.0, TINTA);
        linea(l, (x, cable - 6.0), (x + 26.0, cable - 6.0), 3.0, TINTA);
        let (a, b) = ((x + 26.0, cable), (x + POSTE + 26.0, cable));
        for n in 0..8 {
            let f0 = n as f32 / 8.0;
            let f1 = (n + 1) as f32 / 8.0;
            let comba = |f: f32| 14.0 * 4.0 * f * (1.0 - f);
            linea(
                l,
                (a.0 + (b.0 - a.0) * f0, a.1 + comba(f0)),
                (a.0 + (b.0 - a.0) * f1, a.1 + comba(f1)),
                1.2,
                TINTA,
            );
        }
        x += POSTE;
    }

    // El tranvia: viene de la derecha, cruza y se va. Uno cada `VUELTA`
    // unidades de esta capa, asi que se le ve pasar de vez en cuando.
    const VUELTA: f32 = 2600.0;
    const LARGO: f32 = 250.0;
    let recorrido = (t * 1.4) % VUELTA;
    let primera = (desde / VUELTA).floor() as i32;
    for k in primera - 1..=primera + 1 {
        let tx = k as f32 * VUELTA + VUELTA - recorrido;
        if tx + LARGO < x0 || tx > x1 {
            continue;
        }
        tranvia(l, tx, pulso, t);
    }
}

fn tranvia(l: &Layout, x: f32, pulso: f32, t: f32) {
    let (arriba, abajo) = (340.0, 452.0);
    let traqueteo = ((t * 0.5).sin() * 1.2).round();
    let y = |v: f32| v + traqueteo;
    // El trole: la percha hasta el cable, y la chispa que salta a veces.
    linea(
        l,
        (x + 180.0, y(arriba - 6.0)),
        (x + 60.0, 300.0 + 12.0),
        2.5,
        TINTA,
    );
    if pulso > 0.8 {
        circulo(l, x + 60.0, 312.0, 4.0, fade(LUZ, 0.9));
    }
    // La caja, con el morro redondeado, bordo abajo y crema arriba.
    rect(l, x - 4.0, y(arriba - 12.0), 258.0, 14.0, TINTA);
    rect(l, x + 4.0, y(arriba - 10.0), 242.0, 10.0, TRANVIA);
    rect(
        l,
        x - 3.0,
        y(arriba - 3.0),
        256.0,
        abajo - arriba - 6.0,
        TINTA,
    );
    rect(l, x, y(arriba), 250.0, 50.0, TRANVIA_CREMA);
    rect(
        l,
        x,
        y(arriba + 50.0),
        250.0,
        abajo - arriba - 62.0,
        TRANVIA,
    );
    // Las ventanillas, encendidas, con pasajeros en sombra.
    let mut rng = Pcg32::new(0x7A_4A71);
    for i in 0..7 {
        let vx = x + 12.0 + i as f32 * 33.0;
        rect(
            l,
            vx,
            y(arriba + 8.0),
            26.0,
            32.0,
            fade(VENTANA, 0.8 + 0.2 * pulso),
        );
        draw_rect_lines(l, vx, y(arriba + 8.0), 26.0, 32.0, 1.5);
        if rng.next_f32() < 0.6 {
            circulo(l, vx + 13.0, y(arriba + 26.0), 5.0, fade(TINTA, 0.75));
            rect(l, vx + 6.0, y(arriba + 31.0), 14.0, 9.0, fade(TINTA, 0.75));
        }
    }
    // El numero de la linea, en su cartelito, y el faro.
    rect(l, x + 100.0, y(arriba + 56.0), 50.0, 16.0, TRANVIA_CREMA);
    draw_rect_lines(l, x + 100.0, y(arriba + 56.0), 50.0, 16.0, 1.5);
    let s = l.to_screen(x + 125.0, y(arriba + 69.0));
    fuentes::centrado("64", s.x, s.y, l.len(13.0), Cara::Titulo, TINTA);
    circulo(l, x + 4.0, y(arriba + 70.0), 6.0, fade(LUZ, 0.9));
    // Las ruedas, sobre el riel.
    for rx in [x + 40.0, x + 80.0, x + 170.0, x + 210.0] {
        circulo(l, rx, abajo + 2.0, 9.0, TINTA);
        circulo(l, rx, abajo + 2.0, 4.0, RIEL);
    }
}

/// Faroles porteños en la acera: poste de hierro y farol de cuatro vidrios,
/// con su cono de luz latiendo. Bajo uno de cada tres, el bandoneonista que
/// toca el tango que suena, sentado en su silla.
fn farolas(p: &Paseo, l: &Layout, cam: f32, pulso: f32) {
    const CADA: f32 = 560.0;
    let mut x = (cam / CADA).floor() * CADA + 280.0 - CADA;
    while x < cam + VISTA_W + CADA {
        if p.def.hay_suelo(x) && x < p.def.puerta - 280.0 {
            let arriba = 300.0;
            tri(
                l,
                (x - 8.0, arriba + 14.0),
                (x - 100.0, SUELO_Y),
                (x + 100.0, SUELO_Y),
                fade(LUZ, 0.06 + 0.07 * pulso),
            );
            rect(l, x - 3.0, arriba, 6.0, SUELO_Y - arriba, TINTA);
            rect(l, x - 9.0, SUELO_Y - 18.0, 18.0, 18.0, TINTA);
            rect(l, x - 6.0, arriba + 40.0, 12.0, 5.0, TINTA);
            // El farol: trapecio de vidrio, tejadillo y remate.
            tri(
                l,
                (x - 13.0, arriba - 26.0),
                (x + 13.0, arriba - 26.0),
                (x + 8.0, arriba),
                TINTA,
            );
            tri(
                l,
                (x - 13.0, arriba - 26.0),
                (x + 8.0, arriba),
                (x - 8.0, arriba),
                TINTA,
            );
            let vidrio = fade(LUZ, 0.8 + 0.2 * pulso);
            tri(
                l,
                (x - 10.0, arriba - 23.0),
                (x + 10.0, arriba - 23.0),
                (x + 6.0, arriba - 3.0),
                vidrio,
            );
            tri(
                l,
                (x - 10.0, arriba - 23.0),
                (x + 6.0, arriba - 3.0),
                (x - 6.0, arriba - 3.0),
                vidrio,
            );
            linea(l, (x, arriba - 23.0), (x, arriba - 3.0), 1.5, TINTA);
            tri(
                l,
                (x - 17.0, arriba - 26.0),
                (x, arriba - 40.0),
                (x + 17.0, arriba - 26.0),
                TINTA,
            );
            circulo(l, x, arriba - 42.0, 3.5, TINTA);
            // Solo donde no le tapa un anden o unos cajones.
            let n = (x / CADA) as i32;
            let libre = p
                .def
                .plataformas
                .iter()
                .all(|pl| pl.x > x + 70.0 || pl.x + pl.ancho < x);
            if n % 3 == 1 && libre {
                musico(l, x + 36.0, SUELO_Y, Instrumento::Bandoneon, pulso);
            }
        }
        x += CADA;
    }
}

// ---------------------------------------------------------------------------
// Lo que se pisa
// ---------------------------------------------------------------------------

/// Las plataformas del arrabal. Lo bajo es madera: el anden del tranvia si es
/// largo, un bote amarrado si esta sobre el Riachuelo, cajones del puerto si
/// esta en el muelle. Lo alto es de las casas: balcones de hierro y, lo mas
/// alto, tejados de chapa. La superficie siempre clara, que es informacion.
fn plataformas(p: &Paseo, l: &Layout, tintas: (Color, Color)) {
    for pl in &p.def.plataformas {
        let (x, y, w) = (pl.x, pl.y, pl.ancho);
        let cx = x + w * 0.5;
        if SUELO_Y - y < 60.0 {
            if !p.def.hay_suelo(cx) {
                bote(l, x, y, w, tintas.0);
            } else if w >= 200.0 {
                anden(l, x, y, w);
            } else {
                cajones(l, x, y, w);
            }
        } else if y > 330.0 {
            balcon(l, x, y, w);
        } else {
            tejado(l, x, y, w, tintas.0);
        }
    }
}

/// El canto claro de lo que se pisa, con su tinta.
fn tabla(l: &Layout, x: f32, y: f32, w: f32, alto: f32) {
    rect(l, x - 3.0, y - 3.0, w + 6.0, alto + 6.0, TINTA);
    rect(l, x, y, w, alto, TABLA);
    linea(
        l,
        (x, y + alto - 2.0),
        (x + w, y + alto - 2.0),
        2.0,
        TABLA_SOMBRA,
    );
}

fn anden(l: &Layout, x: f32, y: f32, w: f32) {
    rect(l, x, y, w, SUELO_Y - y, color_u8!(96, 84, 88, 255));
    let mut bx = x + 20.0;
    while bx < x + w {
        linea(l, (bx, y + 10.0), (bx, SUELO_Y), 1.2, fade(TINTA, 0.5));
        bx += 40.0;
    }
    draw_rect_lines(l, x, y, w, SUELO_Y - y, 2.5);
    tabla(l, x, y, w, 9.0);
    // El cartel de la parada, en un poste al borde del anden.
    let px = x + w - 24.0;
    rect(l, px - 2.0, y - 70.0, 4.0, 70.0, TINTA);
    circulo(l, px, y - 76.0, 14.0, TINTA);
    circulo(l, px, y - 76.0, 11.0, TRANVIA_CREMA);
    rect(l, px - 8.0, y - 78.0, 16.0, 4.0, TRANVIA);
}

fn cajones(l: &Layout, x: f32, y: f32, w: f32) {
    let n = ((w / 45.0) as i32).max(1);
    let cw = w / n as f32;
    for i in 0..n {
        let cx = x + cw * i as f32;
        rect(l, cx, y, cw, SUELO_Y - y, CAJON);
        draw_rect_lines(l, cx, y, cw, SUELO_Y - y, 2.0);
        linea(l, (cx, y), (cx + cw, SUELO_Y), 1.5, fade(TINTA, 0.6));
        linea(l, (cx, SUELO_Y), (cx + cw, y), 1.5, fade(TINTA, 0.6));
    }
    tabla(l, x, y, w, 7.0);
}

/// Un bote de La Boca amarrado: casco pintado en la tinta del baile, con la
/// borda clara que es donde se pisa, y la linea de flotacion en el agua.
fn bote(l: &Layout, x: f32, y: f32, w: f32, casco: Color) {
    // Hundido en el agua: la flotacion queda por debajo de la orilla.
    let fondo = SUELO_Y + 28.0;
    let pinta = |g: f32, c: Color| {
        tri(
            l,
            (x - 14.0 - g, y - g),
            (x + 10.0, fondo + g),
            (x + 10.0, y - g),
            c,
        );
        tri(
            l,
            (x + w + 14.0 + g, y - g),
            (x + w - 10.0, fondo + g),
            (x + w - 10.0, y - g),
            c,
        );
        rect(l, x + 10.0, y - g, w - 20.0, fondo - y + 2.0 * g, c);
    };
    pinta(3.0, TINTA);
    pinta(0.0, casco);
    rect(l, x, y + 12.0, w, 5.0, fade(HUESO, 0.8));
    tabla(l, x - 6.0, y, w + 12.0, 6.0);
    // El agua tapando el casco por abajo, y las ondas al pie.
    rect(l, x - 24.0, fondo - 8.0, w + 48.0, 10.0, RIACHUELO);
    linea(
        l,
        (x - 24.0, fondo - 8.0),
        (x + w + 24.0, fondo - 8.0),
        2.0,
        fade(LUZ, 0.3),
    );
}

/// Un balcon del conventillo: losa de madera, barandilla de hierro detras y
/// dos mensulas de volutas debajo.
fn balcon(l: &Layout, x: f32, y: f32, w: f32) {
    linea(l, (x + 2.0, y - 20.0), (x + w - 2.0, y - 20.0), 2.5, TINTA);
    let mut bx = x + 6.0;
    while bx < x + w - 2.0 {
        linea(l, (bx, y - 20.0), (bx, y), 1.5, fade(TINTA, 0.85));
        bx += 10.0;
    }
    tabla(l, x, y, w, 10.0);
    for mx in [x + 16.0, x + w - 16.0] {
        linea(l, (mx, y + 12.0), (mx, y + 34.0), 3.0, TINTA);
        linea(l, (mx - 10.0, y + 12.0), (mx, y + 34.0), 2.0, TINTA);
        circulo(l, mx - 10.0, y + 16.0, 4.0, TINTA);
    }
}

/// Un tejado de chapa: el alero con sus ondas y la canaleta en la tinta del
/// baile.
fn tejado(l: &Layout, x: f32, y: f32, w: f32, canaleta: Color) {
    tabla(l, x, y, w, 8.0);
    rect(l, x - 2.0, y + 9.0, w + 4.0, 12.0, TEJADO);
    let mut bx = x + 3.0;
    while bx < x + w {
        linea(l, (bx, y + 9.0), (bx, y + 21.0), 1.5, TINTA);
        bx += 8.0;
    }
    draw_rect_lines(l, x - 2.0, y + 9.0, w + 4.0, 12.0, 2.0);
    rect(l, x - 4.0, y + 21.0, w + 8.0, 5.0, canaleta);
    draw_rect_lines(l, x - 4.0, y + 21.0, w + 8.0, 5.0, 1.5);
}

// ---------------------------------------------------------------------------
// La puerta
// ---------------------------------------------------------------------------

/// La puerta de la milonga, al final de la calle: un conventillo grande de
/// chapa con la puerta de dos hojas que se abre segun te acercas, guirnaldas de
/// bombillas latiendo al compas y el cartel fileteado con el nombre del baile.
/// Dentro, luz y parejas en la quebrada.
fn puerta(p: &Paseo, l: &Layout, alpha: f32, pulso: f32, (carmin, negro): (Color, Color)) {
    let x = p.def.puerta;
    let cam = camara(p, alpha);
    if x + 400.0 < cam || x - 400.0 > cam + VISTA_W {
        return;
    }
    let (cx, ancho, arriba) = (x + 40.0, 150.0, 260.0);
    let pared = CHAPAS[0];

    // El edificio, que cierra la calle, con su tejado y las ondas de chapa.
    rect(l, x - 230.0, 80.0, 920.0, SUELO_Y - 80.0, pared);
    let mut bx = x - 226.0;
    while bx < x + 690.0 {
        linea(l, (bx, 80.0), (bx, SUELO_Y), 1.0, fade(TINTA, 0.3));
        bx += 7.0;
    }
    rect(l, x - 240.0, 64.0, 940.0, 18.0, TEJADO);
    linea(l, (x - 240.0, 82.0), (x + 700.0, 82.0), 3.0, TINTA);
    linea(l, (x - 230.0, 80.0), (x - 230.0, SUELO_Y), 4.0, TINTA);

    // Ventanas altas encendidas, con persianas abiertas.
    for wx in [x - 140.0, cx + 200.0, cx + 340.0] {
        rect(l, wx - 26.0, 130.0, 52.0, 110.0, VENTANA);
        linea(l, (wx, 130.0), (wx, 240.0), 2.0, TINTA);
        linea(l, (wx - 26.0, 180.0), (wx + 26.0, 180.0), 2.0, TINTA);
        draw_rect_lines(l, wx - 26.0, 130.0, 52.0, 110.0, 3.0);
        for lado in [-1.0, 1.0] {
            let px = wx + lado * 38.0;
            rect(l, px - 11.0, 130.0, 22.0, 110.0, PERSIANA);
            draw_rect_lines(l, px - 11.0, 130.0, 22.0, 110.0, 2.0);
        }
        rect(l, wx - 44.0, 240.0, 88.0, 7.0, TINTA);
    }

    // La luz de dentro, y dos parejas bailando en la quebrada, en sombra.
    let abierta = ((p.jugadora.render_pos(alpha).x - (x - 420.0)) / 320.0).clamp(0.0, 1.0);
    rect(
        l,
        cx - ancho * 0.5,
        arriba,
        ancho,
        SUELO_Y - arriba,
        fade(LUZ, 0.95),
    );
    circulo(l, cx, arriba, ancho * 0.5, fade(LUZ, 0.95));
    for (dx, lado) in [(-34.0, 1.0), (32.0, -1.0)] {
        let bx = cx + dx;
        let quiebre = lado * (4.0 + 6.0 * pulso);
        // El: recto, inclinado hacia ella. Ella: la cintura quebrada.
        linea(
            l,
            (bx - lado * 8.0, SUELO_Y),
            (bx - lado * 6.0 + quiebre * 0.3, SUELO_Y - 60.0),
            7.0,
            fade(TINTA, 0.55),
        );
        circulo(
            l,
            bx - lado * 5.0 + quiebre * 0.4,
            SUELO_Y - 68.0,
            6.0,
            fade(TINTA, 0.55),
        );
        tri(
            l,
            (bx + lado * 6.0 + quiebre, SUELO_Y - 58.0),
            (bx + lado * 2.0, SUELO_Y),
            (bx + lado * 18.0, SUELO_Y),
            fade(TINTA, 0.55),
        );
        circulo(
            l,
            bx + lado * 8.0 + quiebre * 1.2,
            SUELO_Y - 64.0,
            5.5,
            fade(TINTA, 0.55),
        );
    }

    // Las hojas de la puerta, que giran sobre sus bisagras.
    let hoja = ancho * 0.5 * (1.0 - abierta * 0.85);
    for (x0, dir) in [(cx - ancho * 0.5, 1.0), (cx + ancho * 0.5, -1.0)] {
        let x1 = x0 + hoja * dir;
        let (a, b) = (x0.min(x1), x0.max(x1));
        rect(l, a, arriba, b - a, SUELO_Y - arriba, PERSIANA);
        draw_rect_lines(l, a, arriba, b - a, SUELO_Y - arriba, 2.5);
        if b - a > 10.0 {
            rect(
                l,
                a + 6.0,
                arriba + 16.0,
                b - a - 12.0,
                70.0,
                fade(VENTANA, 0.8),
            );
            draw_rect_lines(l, a + 6.0, arriba + 16.0, b - a - 12.0, 70.0, 1.5);
            draw_rect_lines(l, a + 6.0, arriba + 110.0, b - a - 12.0, 80.0, 1.5);
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

    // El cartel fileteado: tabla negra con el nombre, marco en carmin y dos
    // volutas a los lados, que es el filete porteno reducido a lo que cabe.
    let (sy, sw, sh) = (118.0, 230.0, 46.0);
    rect(l, cx - sw * 0.5 - 4.0, sy - 4.0, sw + 8.0, sh + 8.0, TINTA);
    rect(l, cx - sw * 0.5, sy, sw, sh, carmin);
    rect(
        l,
        cx - sw * 0.5 + 6.0,
        sy + 6.0,
        sw - 12.0,
        sh - 12.0,
        negro,
    );
    for lado in [-1.0, 1.0] {
        let vx = cx + lado * (sw * 0.5 + 22.0);
        circulo(l, vx, sy + sh * 0.5, 14.0, TINTA);
        circulo(l, vx, sy + sh * 0.5, 10.0, ORO);
        circulo(l, vx + lado * 3.0, sy + sh * 0.5 - 2.0, 5.0, carmin);
        linea(
            l,
            (vx - lado * 14.0, sy + sh * 0.5),
            (vx - lado * 26.0, sy + sh * 0.5),
            3.0,
            TINTA,
        );
    }
    let s = l.to_screen(cx, sy + sh * 0.5 + 9.0);
    let nombre = p.def.jefe.to_uppercase();
    fuentes::centrado(&nombre, s.x, s.y, l.len(26.0), Cara::Titulo, ORO);

    // La guirnalda de bombillas, de punta a punta, encendidas a compas: una
    // de cada dos salta en el golpe.
    let (ga, gb) = (x - 220.0, x + 680.0);
    let n = 30;
    for i in 0..=n {
        let f = i as f32 / n as f32;
        let gx = ga + (gb - ga) * f;
        let gy = 96.0 + 14.0 * (f * n as f32 / 5.0 * PI).sin().abs();
        let color = [ORO, carmin, color_u8!(110, 170, 120, 255)][i % 3];
        let k = if i % 2 == 0 {
            0.6 + 0.4 * pulso
        } else {
            1.0 - 0.4 * pulso
        };
        circulo(l, gx, gy, 4.5, TINTA);
        circulo(l, gx, gy, 3.2, fade(color, k));
    }

    // La luz se sale a la acera segun se abre, y al llegar la inunda.
    let llegada = p
        .completado
        .map_or(0.0, |d| (((p.tick - d) as f32 + alpha) / 40.0).min(1.0));
    let derrame = fade(LUZ, 0.10 + 0.25 * abierta + 0.3 * llegada);
    let (izq, der) = (cx - ancho * 0.5, cx + ancho * 0.5);
    tri(
        l,
        (izq, SUELO_Y),
        (der, SUELO_Y),
        (der + 160.0, 540.0),
        derrame,
    );
    tri(
        l,
        (izq, SUELO_Y),
        (der + 160.0, 540.0),
        (izq - 160.0, 540.0),
        derrame,
    );
}

// ---------------------------------------------------------------------------
// Los enemigos
// ---------------------------------------------------------------------------

/// El compadrito: chambergo con cinta carmin, saco oscuro, panuelo blanco al
/// cuello y pantalon a rayas. **Su dibujo es su aviso**: en el amago se echa
/// atras y dobla las rodillas; en la estocada va tendido hacia delante con
/// rayas de velocidad que se acortan al frenar; al clavarse levanta polvo y se
/// queda plantado con un pie cruzado, que es el corte del tango.
pub(crate) fn compadrito(
    e: &Enemigo,
    l: &Layout,
    t: f32,
    (carmin, negro): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, pies, d) = (e.pos.x, e.pos.y + 38.0, e.dir);
    let saco = tinte(negro);
    let pantalon = tinte(color_u8!(150, 140, 146, 255));
    let blanco = tinte(HUESO);
    let piel = tinte(PIEL);
    let cinta = tinte(carmin);
    // En que va: amago (0 a 1, cargando), estocada (1 a 0, frenando) o nada.
    let r = e.reloj;
    let (amago, estocada) = if r > PLANTADO_COMPADRITO + ESTOCADA_COMPADRITO {
        let k = (r - PLANTADO_COMPADRITO - ESTOCADA_COMPADRITO) as f32 / AMAGO_COMPADRITO as f32;
        (1.0 - k, 0.0)
    } else if r > PLANTADO_COMPADRITO {
        (
            0.0,
            (r - PLANTADO_COMPADRITO) as f32 / ESTOCADA_COMPADRITO as f32,
        )
    } else {
        (0.0, 0.0)
    };
    // Recien clavado: los primeros ticks de plantado.
    let frenazo = if r <= PLANTADO_COMPADRITO && r > PLANTADO_COMPADRITO - 14 {
        (r - (PLANTADO_COMPADRITO - 14)) as f32 / 14.0
    } else {
        0.0
    };
    // Inclinacion del cuerpo: atras en el amago, adelante en la estocada. Y
    // lo que se agacha.
    let inclina = d * (-10.0 * amago + 16.0 * estocada.max(frenazo * 0.6));
    let agacha = 8.0 * amago + 6.0 * estocada;
    let cadera = (x - inclina * 0.2, pies - 32.0 + agacha);
    let hombros = (x + inclina, pies - 62.0 + agacha * 1.2);
    let cabeza = (hombros.0 + d * 2.0, hombros.1 - 12.0);
    // Los pies: juntos, abiertos en el amago, en zancada en la estocada y
    // cruzados al plantarse.
    let plantado = amago == 0.0 && estocada == 0.0;
    let (delante, detras) = if estocada > 0.0 {
        (
            x + d * (8.0 + 22.0 * estocada),
            x - d * (8.0 + 20.0 * estocada),
        )
    } else if plantado {
        (x - d * 3.0, x + d * 5.0)
    } else {
        (x + d * (6.0 + 4.0 * amago), x - d * (6.0 + 6.0 * amago))
    };

    // Las rayas de la estocada, detras, mas cortas segun frena.
    if estocada > 0.0 {
        for (k, h) in [(0.0, 18.0), (1.0, 40.0), (2.0, 60.0)] {
            let largo = 70.0 * estocada * (1.0 - k * 0.2);
            let y = pies - h;
            let x0 = x - d * (20.0 + k * 6.0);
            linea(l, (x0, y), (x0 - d * largo, y), 3.0, fade(TINTA, 0.8));
        }
    }
    // El polvo del frenazo, delante de los pies.
    if frenazo > 0.0 {
        let k = 1.0 - frenazo;
        for (i, dx) in [(0.0, 10.0), (1.0, 22.0), (2.0, 34.0)] {
            let r = 4.0 + 7.0 * k + i * 1.5;
            circulo(
                l,
                delante + d * dx * (0.5 + k),
                pies - 4.0 - 6.0 * k * i,
                r + 2.0,
                fade(TINTA, 0.8 * frenazo),
            );
            circulo(
                l,
                delante + d * dx * (0.5 + k),
                pies - 4.0 - 6.0 * k * i,
                r,
                fade(HUESO, 0.9 * frenazo),
            );
        }
    }

    // El baile en reposo: un vaiven con el compas del tango, 30 ticks.
    let vaiven = if plantado {
        (0.5 + 0.5 * (t * PI / 15.0).cos()) * 1.5
    } else {
        0.0
    };
    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        // Piernas, con el botin de dos tonos.
        linea(l, cadera, (delante, pies - 4.0), 7.0 + 2.0 * g, c(pantalon));
        linea(l, cadera, (detras, pies - 4.0), 7.0 + 2.0 * g, c(pantalon));
        for px in [delante, detras] {
            rect(
                l,
                px - 6.0 - g,
                pies - 6.0 - g,
                12.0 + 2.0 * g,
                6.0 + 2.0 * g,
                c(blanco),
            );
        }
        // El saco, cruzado, un poco acampanado de faldon.
        linea(
            l,
            (cadera.0, cadera.1 + 6.0),
            (hombros.0, hombros.1 + vaiven),
            20.0 + 2.0 * g,
            c(saco),
        );
        // Los brazos: en el amago uno atras, arriba; en la estocada, los dos
        // tendidos; plantado, en jarras.
        let mano_a = if amago > 0.0 {
            (
                hombros.0 - d * (14.0 + 10.0 * amago),
                hombros.1 - 8.0 * amago,
            )
        } else if estocada > 0.0 {
            (hombros.0 + d * 24.0, hombros.1 + 6.0)
        } else {
            (hombros.0 + d * 10.0, hombros.1 + 22.0)
        };
        let mano_b = if estocada > 0.0 {
            (hombros.0 - d * 18.0, hombros.1 - 4.0)
        } else {
            (hombros.0 - d * 10.0, hombros.1 + 22.0)
        };
        for m in [mano_a, mano_b] {
            linea(l, (hombros.0, hombros.1 + 4.0), m, 6.0 + 2.0 * g, c(saco));
        }
        circulo(l, cabeza.0, cabeza.1 + vaiven, 8.0 + g, c(piel));
    });
    // El panuelo al cuello, que vuela hacia atras en la estocada.
    let (hx, hy) = (hombros.0, hombros.1 + vaiven);
    let vuelo = 10.0 + 18.0 * estocada;
    tri(
        l,
        (hx - 6.0, hy - 3.0),
        (hx + 6.0, hy - 3.0),
        (hx - d * vuelo, hy + 8.0),
        TINTA,
    );
    tri(
        l,
        (hx - 4.5, hy - 2.0),
        (hx + 4.5, hy - 2.0),
        (hx - d * (vuelo - 3.0), hy + 6.0),
        blanco,
    );
    // El chambergo: ala ancha y copa, ladeado sobre un ojo, con la cinta.
    let (sx, sy) = (cabeza.0 + d * 1.5, cabeza.1 + vaiven - 6.0);
    rect(l, sx - 18.0, sy - 2.0, 36.0, 5.0, TINTA);
    rect(l, sx - 9.0, sy - 10.0, 18.0, 9.0, TINTA);
    tri(
        l,
        (sx - 9.0, sy - 10.0),
        (sx + 9.0, sy - 10.0),
        (sx, sy - 7.0),
        fade(HUESO, 0.35),
    );
    rect(l, sx - 16.0, sy - 1.0, 32.0, 2.5, saco);
    rect(l, sx - 8.0, sy - 5.0, 16.0, 3.0, cinta);
    // El bigotito y el ojo, bajo el ala.
    linea(
        l,
        (cabeza.0 + d * 2.0, cabeza.1 + vaiven + 3.0),
        (cabeza.0 + d * 8.0, cabeza.1 + vaiven + 3.0),
        1.8,
        TINTA,
    );
    circulo(l, cabeza.0 + d * 4.0, cabeza.1 + vaiven - 1.0, 1.4, TINTA);
}

/// La florista: falda larga en carmin, blusa clara, mantoncito oscuro, un
/// clavel en el pelo y la canasta de rosas al brazo. Echa el brazo atras antes
/// de tirar, como el camarero de Viena: se ve venir la rosa si se la mira.
pub(crate) fn florista(
    e: &Enemigo,
    l: &Layout,
    (carmin, negro): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let (x, pies, d) = (e.pos.x, e.pos.y + 36.0, e.dir);
    let falda = tinte(carmin);
    let blusa = tinte(HUESO);
    let manton = tinte(negro);
    let piel = tinte(PIEL);
    let carga = (1.0 - e.reloj as f32 / 16.0).clamp(0.0, 1.0);
    let recien = e.reloj > CADENCIA_FLORISTA - 8;
    let hombro = (x, pies - 58.0);
    let mano = if recien {
        (x + d * 26.0, pies - 60.0)
    } else {
        (x - d * (8.0 + 16.0 * carga), pies - 76.0 - 8.0 * carga)
    };
    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        tri(
            l,
            (x - 6.0 - g, pies - 42.0 - g),
            (x - 18.0 - g, pies + g),
            (x + 18.0 + g, pies + g),
            c(falda),
        );
        tri(
            l,
            (x - 6.0 - g, pies - 42.0 - g),
            (x + 18.0 + g, pies + g),
            (x + 6.0 + g, pies - 42.0 - g),
            c(falda),
        );
        rect(
            l,
            x - 8.0 - g,
            pies - 62.0 - g,
            16.0 + 2.0 * g,
            22.0 + g,
            c(blusa),
        );
        tri(
            l,
            (x - 11.0 - g, pies - 62.0 - g),
            (x + 11.0 + g, pies - 62.0 - g),
            (x, pies - 46.0 + g),
            c(manton),
        );
        linea(l, hombro, mano, 5.0 + 2.0 * g, c(blusa));
        circulo(l, x, pies - 70.0, 7.5 + g, c(piel));
    });
    // El pelo recogido y el clavel.
    arco(l, x, pies - 72.0, 7.5, 4.0, TINTA);
    circulo(l, x - d * 8.0, pies - 76.0, 4.5, TINTA);
    circulo(l, x - d * 5.0, pies - 80.0, 3.6, TINTA);
    circulo(l, x - d * 5.0, pies - 80.0, 2.6, carmin);
    circulo(l, x + d * 3.0, pies - 69.0, 1.3, TINTA);
    // La canasta, al otro brazo, con las rosas asomando.
    let (cx, cy) = (x - d * 14.0, pies - 38.0);
    arco(l, cx, cy, 10.0, 2.0, TINTA);
    rect(l, cx - 12.0, cy, 24.0, 12.0, TINTA);
    rect(l, cx - 10.0, cy + 1.5, 20.0, 9.0, tinte(CAJON));
    for dx in [-6.0, 0.0, 6.0] {
        circulo(l, cx + dx, cy - 1.0, 3.5, TINTA);
        circulo(l, cx + dx, cy - 1.0, 2.5, carmin);
    }
    // La rosa en la mano, lista para tirar.
    if !recien {
        circulo(l, mano.0, mano.1, 5.0, TINTA);
        circulo(l, mano.0, mano.1, 3.8, carmin);
    }
}

/// La rosa: cabeza de petalos y tallo con una hoja. **Cuenta en que va**:
/// volando gira y deja rayas, colgada tiembla, y marchita se oscurece, se
/// dobla y va soltando petalos. Las rosas de parry, en el rosa de siempre y
/// con su anillo.
pub(crate) fn rosa(
    e: &Enemigo,
    l: &Layout,
    edad: f32,
    t: f32,
    carmin: Color,
    tinte: &dyn Fn(Color) -> Color,
) {
    let (mut x, mut y) = (e.pos.x, e.pos.y);
    let vuela = edad < VUELO_ROSA as f32;
    let marchita = ((edad - (VUELO_ROSA + COLGADA_ROSA) as f32) / 40.0).clamp(0.0, 1.0);
    let base = if e.rosa { ROSA } else { carmin };
    let color = tinte(mezcla(base, color_u8!(70, 40, 34, 255), marchita * 0.6));
    if vuela {
        let dir = e.vel.normalize_or_zero();
        let rapido = (e.vel.length() / 950.0).min(1.0);
        for k in [-1.0, 1.0] {
            let n = vec2(-dir.y, dir.x) * 5.0 * k;
            let a = (x - dir.x * 14.0 + n.x, y - dir.y * 14.0 + n.y);
            let b = (a.0 - dir.x * 40.0 * rapido, a.1 - dir.y * 40.0 * rapido);
            linea(l, a, b, 2.0, fade(TINTA, 0.7));
        }
    } else if marchita == 0.0 {
        // Colgada: tiembla, como algo que esta a punto de caer.
        x += (t * 1.7).sin() * 1.2;
        y += (t * 2.3).cos() * 0.8;
    }
    // El tallo, que gira volando y se dobla al marchitarse.
    let giro = if vuela {
        edad * 0.45
    } else {
        PI * 0.5 + marchita * 1.2
    };
    let (s, c) = sin_cos(giro);
    let tallo = (x + c * 22.0, y + s * 22.0);
    linea(l, (x, y), tallo, 4.5, TINTA);
    linea(l, (x, y), tallo, 2.2, tinte(VERDE));
    let hoja = (x + c * 13.0 - s * 7.0, y + s * 13.0 + c * 7.0);
    circulo(l, hoja.0, hoja.1, 5.2, TINTA);
    circulo(l, hoja.0, hoja.1, 3.8, tinte(VERDE));
    // La cabeza: petalos en corona y el cogollo en espiral.
    con_tinta(|g, es_tinta| {
        let col = if es_tinta { TINTA } else { color };
        for i in 0..5 {
            let (ps, pc) = sin_cos(giro + i as f32 * PI * 0.4);
            circulo(l, x + pc * 6.0, y + ps * 6.0, 6.5 + g, col);
        }
    });
    circulo(l, x, y, 5.5, mezcla(color, TINTA, 0.35));
    arco(l, x + 0.5, y + 1.5, 3.5, 1.5, TINTA);
    // Marchita, suelta petalos que caen.
    if marchita > 0.0 {
        for i in 0..3 {
            let f = (marchita * 2.0 + i as f32 * 0.33) % 1.0;
            let px = x + (i as f32 - 1.0) * 8.0 + (t * 0.1 + i as f32).sin() * 4.0;
            circulo(l, px, y + 8.0 + f * 40.0, 2.4, fade(color, 1.0 - f));
        }
    }
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
