//! Montmartre: la cuesta que se sube antes del cancan.
//!
//! Es el mundo de Toulouse-Lautrec, y por eso el cartel aqui no es una
//! referencia sino el sitio: la noche de color ciruela de sus litografias, las
//! siluetas negras recortadas, los blancos de papel y el rojo del molino. Las
//! capas, de lejos a cerca: el cielo con su luna en creciente, **Paris abajo**
//! con la torre Eiffel barriendo el cielo con su faro, **la Butte** con los
//! molinos de la Galette y las cupulas blancas del Sacre-Coeur aun en obras, y
//! la calle con sus casas de tejado de zinc.
//!
//! Lo que lo hace otro paseo es que **sube**. La camara sigue a la acera en
//! vertical (`calle::camara_y`), y cada capa baja lo que le toca: segun se
//! sube, Paris se queda abajo y la Butte se hunde detras de los tejados, que
//! es como se sabe que se ha llegado arriba sin mirar el camino.
//!
//! Las casas de la calle van **en la capa de la calle** (al 100 %), no en un
//! parallax: estan pegadas a la cuesta y tienen que subir peldano a peldano
//! con ella. La profundidad la dan la Butte y Paris, que si se mueven aparte.
//!
//! Todo lo que se pisa va claro y con canto de tinta —peldanos, bordillo,
//! toldos, mesas—, y todo lo de detras, oscuro: el fondo no puede competir con
//! una corista que viene a por ti.

use macroquad::prelude::*;
use vals_core::math::{PI, sin_cos};
use vals_core::paseo::{Clase, Enemigo, Escalera, Paseo, PaseoDef, VISTA_H, VISTA_W};
use vals_core::rng::Pcg32;

use crate::calle::{self, HUESO, ROSA, con_tinta};
use crate::draw::{Layout, fade};
use crate::escenarios::{arco, circulo, linea, rect, tri};
use crate::fuentes::{self, Cara};
use crate::paleta::{LUZ, ORO, TINTA};

// La noche de Lautrec: ciruela arriba y granate abajo, donde la ciudad la
// enciende. Es la del Moulin Rouge de `escenarios`, sacada a la calle.
const CIELO_ALTO: Color = color_u8!(22, 12, 32, 255);
const CIELO: Color = color_u8!(36, 18, 46, 255);
const CIELO_MEDIO: Color = color_u8!(58, 26, 56, 255);
const CIELO_BAJO: Color = color_u8!(98, 40, 60, 255);
const LUNA: Color = color_u8!(242, 228, 192, 255);
/// Paris, abajo: mas oscuro que el horizonte encendido, que es como se ve una
/// ciudad de noche desde un alto.
const PARIS: Color = color_u8!(44, 20, 42, 255);
/// La Butte, mas oscura aun: esta mas cerca.
const BUTTE: Color = color_u8!(30, 14, 32, 255);
const SACRE: Color = color_u8!(234, 224, 206, 255);
const SACRE_SOMBRA: Color = color_u8!(178, 160, 164, 255);
const MURO: Color = color_u8!(70, 38, 56, 255);
const MURO_OSCURO: Color = color_u8!(52, 28, 44, 255);
const ZINC: Color = color_u8!(84, 90, 116, 255);
const ZINC_CLARO: Color = color_u8!(168, 176, 196, 255);
/// Las persianas verdes de Paris, que en un cartel de Lautrec son el verde.
const PERSIANA: Color = color_u8!(62, 96, 74, 255);
const VENTANA: Color = color_u8!(238, 190, 104, 255);
const APAGADA: Color = color_u8!(34, 26, 54, 255);
const CHIMENEA: Color = color_u8!(150, 70, 56, 255);
/// El muro de contencion: la Butte es una montana de muros con calles encima.
const PIEDRA: Color = color_u8!(56, 36, 50, 255);
const JUNTA: Color = color_u8!(38, 22, 34, 255);
const BORDILLO: Color = color_u8!(138, 116, 118, 255);
/// Los peldanos, claros: son lo que se pisa en una escalera.
const PELDANO: Color = color_u8!(174, 154, 146, 255);
const VACIO: Color = color_u8!(14, 8, 18, 255);
const ROJO: Color = color_u8!(190, 38, 48, 255);
const ROJO_OSCURO: Color = color_u8!(112, 20, 34, 255);
const CREMA: Color = color_u8!(238, 224, 192, 255);
const MARMOL: Color = color_u8!(226, 216, 198, 255);
const MORRIS: Color = color_u8!(44, 78, 62, 255);
const AMARILLO: Color = color_u8!(238, 196, 70, 255);
const MEDIA: Color = color_u8!(28, 22, 28, 255);
const PIEL: Color = color_u8!(242, 214, 188, 255);
const VIDRIO: Color = color_u8!(42, 98, 62, 255);
const BLUSON: Color = color_u8!(76, 108, 170, 255);
const MADERA: Color = color_u8!(134, 86, 52, 255);
/// Los colores de la paleta del pintor, que son los del cartel.
const PINTURAS: [Color; 3] = [
    color_u8!(242, 198, 62, 255),
    color_u8!(76, 132, 214, 255),
    color_u8!(232, 96, 142, 255),
];

/// Un tramo de acera mas corto que esto, entre dos vacios, es un tejado. El RON
/// no dice de que es cada tramo, y no hace falta: en Montmartre la calle es
/// larga y lo que se salta de uno en uno son tejados.
const TEJADO: f32 = 400.0;

// ---------------------------------------------------------------------------
// El fondo
// ---------------------------------------------------------------------------

/// El fondo de Montmartre, de lo mas lejano a las casas de la calle.
pub fn fondo(p: &Paseo, l: &Layout, capa: &dyn Fn(f32) -> Layout, cam: f32, pulso: f32) {
    let t = p.tick as f32;
    cielo(l, pulso);
    paris(&capa(0.1), cam * 0.1, t, pulso);
    butte(&capa(0.3), cam * 0.3, t, pulso);
    casas(p, &capa(1.0), cam, pulso);
}

/// El cielo, con la luna en creciente (la de Viena es llena: dos noches
/// distintas) y pocas estrellas, que la ciudad las apaga.
fn cielo(l: &Layout, pulso: f32) {
    let franjas = [CIELO_ALTO, CIELO, CIELO_MEDIO, CIELO_BAJO];
    let alto = VISTA_H / franjas.len() as f32;
    for (i, c) in franjas.into_iter().enumerate() {
        rect(l, 0.0, alto * i as f32, VISTA_W, alto + 1.0, c);
    }
    let mut rng = Pcg32::new(0x5747_2E11);
    for _ in 0..26 {
        let (x, y) = (rng.next_f32() * VISTA_W, rng.next_f32() * 200.0);
        let brillo = 0.4 + 0.6 * rng.next_f32();
        circulo(l, x, y, 1.2, fade(HUESO, brillo * (0.6 + 0.4 * pulso)));
    }
    circulo(l, 170.0, 92.0, 48.0, fade(LUNA, 0.07));
    circulo(l, 170.0, 92.0, 30.0, LUNA);
    circulo(l, 182.0, 84.0, 27.0, CIELO_ALTO);
}

/// Paris desde la Butte: tejados sin fin con ventanitas y, sola, la torre
/// Eiffel con su faro barriendo el cielo, que en 1889 era lo mas moderno de la
/// ciudad y se veia desde todas partes.
fn paris(l: &Layout, desde: f32, t: f32, pulso: f32) {
    let base = 392.0;
    let mut x = (desde / 40.0).floor() * 40.0 - 40.0;
    while x < desde + VISTA_W + 40.0 {
        // La altura de cada manzana sale de su x: fija, sin guardar nada.
        let mut rng = Pcg32::new(0xFA_2150 + (x / 40.0) as i64 as u64);
        let h = 14.0 + rng.next_f32() * 30.0;
        rect(l, x, base - h, 41.0, VISTA_H, PARIS);
        if rng.next_f32() < 0.6 {
            let vy = base - h + 6.0 + rng.next_f32() * 20.0;
            rect(
                l,
                x + 8.0 + rng.next_f32() * 20.0,
                vy,
                3.0,
                3.0,
                fade(VENTANA, 0.6),
            );
        }
        if rng.next_f32() < 0.15 {
            // Una cupula o un campanario de barrio.
            circulo(l, x + 20.0, base - h, 10.0, PARIS);
            rect(l, x + 19.0, base - h - 22.0, 2.0, 12.0, PARIS);
        }
        x += 40.0;
    }

    // La torre, en su sitio y una sola vez.
    let (tx, pie) = (1150.0, base - 10.0);
    if tx + 200.0 < desde || tx - 200.0 > desde + VISTA_W {
        return;
    }
    let cima = pie - 230.0;
    // El faro: dos haces que giran. Vistos de lado, un haz que gira es una
    // raya que se alarga y se acorta, y eso basta.
    for k in 0..2 {
        let (s, c) = sin_cos(t * 0.012 + PI * k as f32);
        let largo = 520.0 * c;
        let punta = (tx + largo, cima + 6.0 + 30.0 * s);
        tri(
            l,
            (tx, cima + 2.0),
            (punta.0, punta.1 - 9.0),
            (punta.0, punta.1 + 9.0),
            fade(LUZ, 0.07 + 0.04 * pulso),
        );
    }
    let torre = color_u8!(56, 26, 48, 255);
    // Las patas con el arco entre ellas, el primer piso, el fuste y la aguja.
    tri(
        l,
        (tx - 58.0, pie),
        (tx - 16.0, pie - 70.0),
        (tx - 30.0, pie),
        torre,
    );
    tri(
        l,
        (tx + 58.0, pie),
        (tx + 16.0, pie - 70.0),
        (tx + 30.0, pie),
        torre,
    );
    rect(l, tx - 40.0, pie - 72.0, 80.0, 8.0, torre);
    tri(
        l,
        (tx - 30.0, pie - 64.0),
        (tx - 6.0, pie - 160.0),
        (tx - 12.0, pie - 64.0),
        torre,
    );
    tri(
        l,
        (tx + 30.0, pie - 64.0),
        (tx + 6.0, pie - 160.0),
        (tx + 12.0, pie - 64.0),
        torre,
    );
    rect(l, tx - 18.0, pie - 132.0, 36.0, 5.0, torre);
    tri(
        l,
        (tx - 8.0, pie - 158.0),
        (tx, cima),
        (tx + 8.0, pie - 158.0),
        torre,
    );
    circulo(l, tx, cima + 4.0, 3.0, fade(LUZ, 0.7 + 0.3 * pulso));
}

/// La Butte: el monte lleno de casas, con los molinos de la Galette girando y
/// el Sacre-Coeur arriba, blanco y en obras (no se acabo hasta 1914: en 1890
/// tenia andamios, y el andamio es lo que dice la epoca).
fn butte(l: &Layout, desde: f32, t: f32, pulso: f32) {
    // El perfil sube de izquierda a derecha, como el paseo.
    let cresta = |x: f32| 350.0 - 150.0 * (x / 2300.0).clamp(0.0, 1.0);
    let mut x = (desde / 56.0).floor() * 56.0 - 56.0;
    while x < desde + VISTA_W + 56.0 {
        let mut rng = Pcg32::new(0x000B_077E + (x / 56.0) as i64 as u64);
        let techo = cresta(x) + rng.next_f32() * 26.0;
        rect(l, x, techo, 57.0, VISTA_H + 300.0, BUTTE);
        tri(
            l,
            (x - 2.0, techo),
            (x + 28.0, techo - 14.0 - rng.next_f32() * 10.0),
            (x + 58.0, techo),
            BUTTE,
        );
        for k in 0..2 {
            if rng.next_f32() < 0.35 {
                let vx = x + 12.0 + k as f32 * 22.0;
                rect(
                    l,
                    vx,
                    techo + 14.0,
                    5.0,
                    7.0,
                    fade(VENTANA, 0.55 + 0.2 * pulso),
                );
            }
        }
        x += 56.0;
    }

    // Los molinos de la Galette y el Radet, sobre la cresta.
    for mx in [760.0, 1320.0] {
        if mx + 150.0 < desde || mx - 150.0 > desde + VISTA_W {
            continue;
        }
        let pie = cresta(mx) + 6.0;
        tri(
            l,
            (mx - 26.0, pie),
            (mx - 14.0, pie - 70.0),
            (mx + 26.0, pie),
            BUTTE,
        );
        tri(
            l,
            (mx + 26.0, pie),
            (mx - 14.0, pie - 70.0),
            (mx + 14.0, pie - 70.0),
            BUTTE,
        );
        tri(
            l,
            (mx - 20.0, pie - 70.0),
            (mx, pie - 88.0),
            (mx + 20.0, pie - 70.0),
            BUTTE,
        );
        rect(l, mx - 5.0, pie - 30.0, 10.0, 14.0, fade(VENTANA, 0.7));
        aspas(l, (mx, pie - 74.0), 64.0, t * 0.008 + mx, BUTTE, 2.0);
    }

    // El Sacre-Coeur, arriba del todo.
    let sx = 2380.0;
    if sx + 260.0 < desde || sx - 260.0 > desde + VISTA_W {
        return;
    }
    let pie = cresta(sx) + 4.0;
    let luz = |c: Color| fade(c, 0.88 + 0.12 * pulso);
    // El cuerpo y las cupulas menores.
    rect(l, sx - 110.0, pie - 70.0, 220.0, 70.0, luz(SACRE));
    rect(l, sx - 110.0, pie - 70.0, 36.0, 70.0, luz(SACRE_SOMBRA));
    for (dx, r) in [(-78.0, 20.0), (78.0, 20.0)] {
        rect(l, sx + dx - r, pie - 100.0, r * 2.0, 32.0, luz(SACRE));
        circulo(l, sx + dx, pie - 100.0, r, luz(SACRE));
        arco(l, sx + dx, pie - 100.0, r, 2.0, TINTA);
        rect(l, sx + dx - 1.5, pie - 100.0 - r - 12.0, 3.0, 12.0, TINTA);
    }
    // La cupula grande, alargada, que es la suya, sobre su tambor de
    // columnas: la elipse entera y el tambor tapandole la mitad de abajo.
    let p = l.to_screen(sx, pie - 132.0);
    draw_ellipse(p.x, p.y, l.len(40.0), l.len(64.0), 0.0, luz(SACRE));
    draw_ellipse_lines(p.x, p.y, l.len(40.0), l.len(64.0), 0.0, l.len(2.5), TINTA);
    rect(l, sx - 4.0, pie - 214.0, 8.0, 20.0, luz(SACRE));
    circulo(l, sx, pie - 218.0, 5.0, luz(SACRE));
    rect(l, sx - 44.0, pie - 134.0, 88.0, 66.0, luz(SACRE));
    linea(
        l,
        (sx - 46.0, pie - 134.0),
        (sx + 46.0, pie - 134.0),
        2.5,
        TINTA,
    );
    for k in 0..6 {
        let vx = sx - 36.0 + k as f32 * 14.0;
        rect(l, vx, pie - 124.0, 5.0, 26.0, fade(TINTA, 0.5));
    }
    // Las puertas y el contorno, en tinta.
    for dx in [-24.0, 0.0, 24.0] {
        circulo(l, sx + dx, pie - 30.0, 8.0, TINTA);
        rect(l, sx + dx - 8.0, pie - 30.0, 16.0, 30.0, TINTA);
    }
    linea(
        l,
        (sx - 110.0, pie - 70.0),
        (sx + 110.0, pie - 70.0),
        2.5,
        TINTA,
    );
    linea(l, (sx - 110.0, pie - 70.0), (sx - 110.0, pie), 2.5, TINTA);
    linea(l, (sx + 110.0, pie - 70.0), (sx + 110.0, pie), 2.5, TINTA);
    // El andamio del campanario que aun no esta: una reja de palos a la
    // derecha, mas alta que la cupula.
    let (ax, ab, aa) = (sx + 130.0, pie, pie - 180.0);
    for k in 0..4 {
        let x = ax + k as f32 * 16.0;
        linea(l, (x, ab), (x, aa + k as f32 * 8.0), 2.0, TINTA);
    }
    let mut y = ab;
    while y > aa {
        linea(l, (ax - 2.0, y), (ax + 50.0, y), 1.5, TINTA);
        linea(l, (ax, y), (ax + 48.0, y - 26.0), 1.0, fade(TINTA, 0.7));
        y -= 26.0;
    }
}

/// Las aspas de un molino: cuatro velas de enrejado girando alrededor de
/// `eje`. La usan los molinos de la Butte y el Moulin Rouge.
fn aspas(l: &Layout, eje: (f32, f32), largo: f32, giro: f32, lona: Color, g: f32) {
    let ancho = largo * 0.2;
    for k in 0..4 {
        let (s, c) = sin_cos(giro + PI * 0.5 * k as f32);
        let (ps, pc) = (c, -s);
        let base = (eje.0 + c * largo * 0.12, eje.1 + s * largo * 0.12);
        let punta = (eje.0 + c * largo, eje.1 + s * largo);
        let esquina = |p: (f32, f32), d: f32| (p.0 + pc * d, p.1 + ps * d);
        let (a0, a1) = (esquina(base, 0.0), esquina(base, ancho));
        let (b0, b1) = (esquina(punta, 0.0), esquina(punta, ancho));
        tri(l, a0, a1, b1, lona);
        tri(l, a0, b1, b0, lona);
        linea(l, eje, punta, g * 1.5, TINTA);
        linea(l, a1, b1, g, TINTA);
        linea(l, b0, b1, g, TINTA);
        for f in [0.4, 0.6, 0.8] {
            let q = (
                base.0 + (punta.0 - base.0) * f,
                base.1 + (punta.1 - base.1) * f,
            );
            linea(l, esquina(q, 0.0), esquina(q, ancho), g * 0.7, TINTA);
        }
    }
    circulo(l, eje.0, eje.1, largo * 0.08, TINTA);
}

/// Las casas de la calle, pegadas a la cuesta: fachadas de color vino,
/// persianas verdes, mansardas de zinc con chimeneas de barro, y de vez en
/// cuando un cartel pegado. Suben peldano a peldano con la acera.
fn casas(p: &Paseo, l: &Layout, cam: f32, pulso: f32) {
    const VANO: f32 = 210.0;
    let def = &p.def;
    let primero = (cam / VANO).floor() as i32 - 1;
    for k in primero..primero + (VISTA_W / VANO) as i32 + 3 {
        let x0 = k as f32 * VANO;
        let cx = x0 + VANO * 0.5;
        // Cada cuatro, una bocacalle: sin huecos la calle era un pasillo y no
        // se veia la Butte, que es lo que dice donde estas.
        if !es_calle(def, cx) || cx > def.puerta - 300.0 || k.rem_euclid(4) == 3 {
            continue;
        }
        let mut rng = Pcg32::new(0xCA5A + k as u64);
        // El pie de la casa, lo mas bajo de su vano: la acera tapa el resto.
        let pie = def.altura(x0).max(def.altura(x0 + VANO));
        let techo = def.altura(cx) - 150.0 - rng.next_f32() * 60.0;
        let muro = if k % 2 == 0 { MURO } else { MURO_OSCURO };
        rect(l, x0, techo, VANO + 1.0, pie - techo + 4.0, muro);
        linea(l, (x0, techo), (x0, pie), 2.0, TINTA);

        // La mansarda de zinc, con su buhardilla y sus chimeneas.
        let m = 38.0;
        tri(
            l,
            (x0, techo),
            (x0 + 16.0, techo - m),
            (x0 + VANO, techo),
            ZINC,
        );
        tri(
            l,
            (x0 + 16.0, techo - m),
            (x0 + VANO - 16.0, techo - m),
            (x0 + VANO, techo),
            ZINC,
        );
        linea(
            l,
            (x0 + 16.0, techo - m),
            (x0 + VANO - 16.0, techo - m),
            2.5,
            TINTA,
        );
        linea(l, (x0 - 2.0, techo), (x0 + VANO + 2.0, techo), 3.0, TINTA);
        let bx = x0 + 60.0 + rng.next_f32() * 70.0;
        rect(l, bx - 12.0, techo - m + 6.0, 24.0, 26.0, ZINC_CLARO);
        rect(
            l,
            bx - 8.0,
            techo - m + 12.0,
            16.0,
            18.0,
            if rng.next_f32() < 0.5 {
                fade(VENTANA, 0.8 + 0.2 * pulso)
            } else {
                APAGADA
            },
        );
        tri(
            l,
            (bx - 15.0, techo - m + 7.0),
            (bx, techo - m - 6.0),
            (bx + 15.0, techo - m + 7.0),
            ZINC_CLARO,
        );
        for c in 0..(1 + (rng.next_f32() * 2.0) as i32) {
            let chx = x0 + 30.0 + c as f32 * 110.0 + rng.next_f32() * 20.0;
            rect(l, chx - 9.0, techo - m - 22.0, 18.0, 24.0, MURO_OSCURO);
            linea(
                l,
                (chx - 9.0, techo - m - 22.0),
                (chx + 9.0, techo - m - 22.0),
                2.0,
                TINTA,
            );
            for d in [-4.0, 4.0] {
                rect(l, chx + d - 2.5, techo - m - 30.0, 5.0, 9.0, CHIMENEA);
            }
        }

        // Dos pisos de ventanas altas con sus persianas abiertas.
        for piso in 0..2 {
            let vy = techo + 30.0 + piso as f32 * 70.0;
            if vy + 44.0 > pie - 60.0 {
                break;
            }
            for col in 0..2 {
                let vx = x0 + 50.0 + col as f32 * 90.0;
                let encendida = rng.next_f32() < 0.4;
                let cristal = if encendida {
                    fade(VENTANA, 0.8 + 0.2 * pulso)
                } else {
                    APAGADA
                };
                rect(l, vx - 12.0, vy, 24.0, 40.0, cristal);
                linea(l, (vx, vy), (vx, vy + 40.0), 1.5, TINTA);
                draw_rect(l, vx - 12.0, vy, 24.0, 40.0, 2.0);
                for lado in [-1.0, 1.0] {
                    let px = vx + lado * 12.0 + if lado < 0.0 { -11.0 } else { 0.0 };
                    rect(l, px, vy, 11.0, 40.0, PERSIANA);
                    draw_rect(l, px, vy, 11.0, 40.0, 1.5);
                }
                // Una barandilla de hierro en el piso de abajo.
                if piso == 1 {
                    linea(
                        l,
                        (vx - 16.0, vy + 32.0),
                        (vx + 16.0, vy + 32.0),
                        2.0,
                        TINTA,
                    );
                }
            }
        }

        // Cada tres casas, un cartel pegado a la pared.
        if k.rem_euclid(3) == 1 {
            cartel(
                l,
                x0 + 30.0 + rng.next_f32() * 90.0,
                def.altura(cx) - 130.0,
                k,
                pulso,
            );
        }
        // Y cada cinco, una columna Morris delante, en lo llano.
        if k.rem_euclid(5) == 2 && (def.altura(x0) - def.altura(x0 + VANO)).abs() < 1.0 {
            morris(l, x0 + VANO * 0.5, def.altura(cx), k, pulso);
        }
    }

    // Farolas en lo llano: lo de las escaleras va con las escaleras.
    const CADA: f32 = 560.0;
    let mut x = (cam / CADA).floor() * CADA + 300.0 - CADA;
    while x < cam + VISTA_W + CADA {
        let llano = def
            .escaleras
            .iter()
            .all(|e| x < e.x - 160.0 || x > e.x + e.ancho + 160.0);
        if llano && es_calle(def, x) && x < def.puerta - 400.0 {
            farola(l, x, def.altura(x), pulso);
        }
        x += CADA;
    }
}

/// Si en `x` hay calle (y no un tejado ni un foso).
fn es_calle(def: &PaseoDef, x: f32) -> bool {
    def.suelo
        .iter()
        .any(|&(a, b)| x >= a && x <= b && b - a >= TEJADO)
}

/// Si en `x` se pisa un tejado.
fn es_tejado(def: &PaseoDef, x: f32) -> bool {
    def.suelo
        .iter()
        .any(|&(a, b)| x >= a && x <= b && b - a < TEJADO)
}

fn draw_rect(l: &Layout, x: f32, y: f32, w: f32, h: f32, g: f32) {
    let p = l.to_screen(x, y);
    draw_rectangle_lines(p.x, p.y, l.len(w), l.len(h), l.len(g).max(1.0), TINTA);
}

/// Un cartel de cabaret pegado a la pared. Dos modelos, los dos de la epoca:
/// el del gato negro sobre amarillo (el Chat Noir) y el de la corista sobre
/// crema con su letrero rojo (el Moulin Rouge). Formas planas y tinta, que es
/// lo que era un cartel.
fn cartel(l: &Layout, x: f32, y: f32, k: i32, pulso: f32) {
    let (w, h) = (54.0, 78.0);
    let luz = |c: Color| fade(c, 0.85 + 0.15 * pulso);
    if k.rem_euclid(2) == 0 {
        rect(l, x, y, w, h, luz(AMARILLO));
        // El gato, sentado de perfil, con la cola en gancho.
        let (gx, gy) = (x + w * 0.5, y + h - 10.0);
        let q = l.to_screen(gx, gy - 14.0);
        draw_ellipse(q.x, q.y, l.len(13.0), l.len(18.0), 0.0, TINTA);
        circulo(l, gx + 6.0, gy - 36.0, 9.0, TINTA);
        tri(
            l,
            (gx, gy - 42.0),
            (gx + 2.0, gy - 54.0),
            (gx + 7.0, gy - 44.0),
            TINTA,
        );
        tri(
            l,
            (gx + 8.0, gy - 44.0),
            (gx + 13.0, gy - 54.0),
            (gx + 14.0, gy - 40.0),
            TINTA,
        );
        linea(l, (gx - 10.0, gy - 4.0), (gx - 20.0, gy - 24.0), 3.5, TINTA);
        linea(
            l,
            (gx - 20.0, gy - 24.0),
            (gx - 14.0, gy - 34.0),
            3.5,
            TINTA,
        );
        circulo(l, gx + 9.0, gy - 38.0, 1.8, luz(AMARILLO));
        rect(l, x, y, w, 12.0, luz(ROJO));
    } else {
        rect(l, x, y, w, h, luz(CREMA));
        rect(l, x, y, w, 14.0, luz(ROJO));
        // La corista, pateando: la falda blanca y la pierna negra arriba.
        let (cx, cy) = (x + w * 0.5, y + h - 14.0);
        tri(
            l,
            (cx, cy - 26.0),
            (cx - 16.0, cy - 4.0),
            (cx + 14.0, cy - 8.0),
            HUESO,
        );
        linea(l, (cx + 4.0, cy - 12.0), (cx + 20.0, cy - 40.0), 3.0, TINTA);
        linea(l, (cx - 4.0, cy - 6.0), (cx - 6.0, cy + 10.0), 3.0, TINTA);
        circulo(l, cx - 2.0, cy - 32.0, 5.0, TINTA);
        // El publico: la fila de sombreros negros de abajo, como en el de
        // Lautrec.
        rect(l, x, y + h - 8.0, w, 8.0, TINTA);
        for i in 0..4 {
            circulo(l, x + 7.0 + i as f32 * 13.0, y + h - 9.0, 5.0, TINTA);
        }
    }
    draw_rect(l, x, y, w, h, 2.0);
}

/// Una columna Morris: el cilindro verde de los carteles, con su cupulita.
fn morris(l: &Layout, x: f32, suelo: f32, k: i32, pulso: f32) {
    let (w, h) = (40.0, 150.0);
    let arriba = suelo - h;
    rect(l, x - w * 0.5, arriba, w, h, MORRIS);
    // Carteles alrededor: franjas planas de color.
    let colores = [AMARILLO, CREMA, ROJO, CREMA];
    for (i, c) in colores.iter().enumerate() {
        let y = arriba + 24.0 + i as f32 * 26.0;
        let dx = ((k + i as i32) % 3) as f32 * 4.0;
        rect(
            l,
            x - w * 0.5 + 3.0 + dx,
            y,
            w - 10.0,
            22.0,
            fade(*c, 0.9 + 0.1 * pulso),
        );
        draw_rect(l, x - w * 0.5 + 3.0 + dx, y, w - 10.0, 22.0, 1.2);
    }
    draw_rect(l, x - w * 0.5, arriba, w, h, 2.5);
    rect(l, x - w * 0.5 - 5.0, arriba - 8.0, w + 10.0, 10.0, MORRIS);
    draw_rect(l, x - w * 0.5 - 5.0, arriba - 8.0, w + 10.0, 10.0, 2.0);
    circulo(l, x, arriba - 8.0, w * 0.45, MORRIS);
    arco(l, x, arriba - 8.0, w * 0.45, 2.0, TINTA);
    circulo(l, x, arriba - 8.0 - w * 0.45, 3.5, TINTA);
}

/// Una farola de Paris: fuste con anillos y el farol de cuatro cristales, con
/// su cono de luz latiendo con el galop.
fn farola(l: &Layout, x: f32, suelo: f32, pulso: f32) {
    let arriba = suelo - 170.0;
    tri(
        l,
        (x, arriba + 10.0),
        (x - 80.0, suelo),
        (x + 80.0, suelo),
        fade(LUZ, 0.05 + 0.06 * pulso),
    );
    rect(l, x - 3.0, arriba, 6.0, suelo - arriba, TINTA);
    rect(l, x - 9.0, suelo - 18.0, 18.0, 18.0, TINTA);
    rect(l, x - 6.0, suelo - 70.0, 12.0, 5.0, TINTA);
    tri(
        l,
        (x - 10.0, arriba),
        (x + 10.0, arriba),
        (x, arriba + 12.0),
        TINTA,
    );
    rect(l, x - 11.0, arriba - 26.0, 22.0, 26.0, TINTA);
    rect(
        l,
        x - 8.0,
        arriba - 23.0,
        16.0,
        20.0,
        fade(LUZ, 0.8 + 0.2 * pulso),
    );
    linea(l, (x, arriba - 23.0), (x, arriba - 3.0), 1.2, TINTA);
    tri(
        l,
        (x - 14.0, arriba - 26.0),
        (x, arriba - 40.0),
        (x + 14.0, arriba - 26.0),
        TINTA,
    );
    circulo(l, x, arriba - 42.0, 3.0, TINTA);
}

// ---------------------------------------------------------------------------
// La calle: la cuesta, lo que se pisa y el molino
// ---------------------------------------------------------------------------

/// Lo que se pisa en Montmartre: la cuesta con sus peldanos y sus muros, las
/// barandillas, las plataformas y la puerta del Moulin Rouge.
pub fn calle(p: &Paseo, l: &Layout, alpha: f32, t: f32, pulso: f32) {
    let cam = calle::camara(p, alpha);
    let fondo = calle::camara_y(p, alpha) + VISTA_H + 10.0;
    suelo(p, l, cam, fondo, t);
    barandillas(p, l, cam, pulso);
    plataformas(p, l);
    molino_rojo(p, l, alpha, t, pulso);
}

/// Cuantos peldanos tiene una escalera: de unos 14 de alto cada uno.
fn peldanos(e: &Escalera) -> usize {
    (e.sube / 14.0).round().max(1.0) as usize
}

/// La altura de la acera **dibujada**: la de `altura` pero a peldanos. La
/// fisica pisa una rampa (ver `Escalera`), y la rampa pasa por el centro de
/// cada huella: los pies no se separan del peldano mas de medio escalon.
fn perfil(def: &PaseoDef, x: f32) -> f32 {
    def.escaleras.iter().fold(def.altura(-1.0e6), |y, e| {
        let n = peldanos(e) as f32;
        let k = ((x - e.x) / (e.ancho / n) + 0.5).floor().clamp(0.0, n);
        y - e.sube * k / n
    })
}

/// Si `x` cae en una escalera (en sus huellas, no en el llano de al lado).
fn en_escalera(def: &PaseoDef, x: f32) -> bool {
    def.escaleras.iter().any(|e| {
        let medio = e.ancho / peldanos(e) as f32 * 0.5;
        x > e.x + medio && x < e.x + e.ancho - medio
    })
}

/// La cuesta. Cada tramo se parte por donde cambia de altura —los bordes de
/// cada peldano— y cada trozo es un rectangulo hasta abajo del todo: la calle
/// es la tapa de un muro, y la escalera, un muro a peldanos. En los tejados el
/// muro es la fachada de la casa, con sus ventanas, y el foso, la calle de
/// abajo a oscuras.
fn suelo(p: &Paseo, l: &Layout, cam: f32, fondo: f32, t: f32) {
    let def = &p.def;
    let (izq, der) = (cam - 20.0, cam + VISTA_W + 20.0);

    // Los fosos: el vacio, con una farola lejana abajo del todo.
    let mut antes = f32::MIN;
    for &(a, b) in def
        .suelo
        .iter()
        .chain([(def.largo + 1.0, def.largo + 2.0)].iter())
    {
        if antes > f32::MIN && a > izq && antes < der {
            let y = def.altura((antes + a) * 0.5);
            rect(l, antes, y, a - antes, fondo - y, VACIO);
            let luz = 0.5 + 0.5 * (t * 0.05 + antes).sin();
            circulo(
                l,
                (antes + a) * 0.5,
                y + 150.0,
                26.0,
                fade(LUZ, 0.05 + 0.03 * luz),
            );
            circulo(l, (antes + a) * 0.5, y + 150.0, 3.0, fade(LUZ, 0.6));
        }
        antes = b;
    }

    for &(a, b) in &def.suelo {
        if b < izq || a > der {
            continue;
        }
        let (a0, b0) = (a.max(izq), b.min(der));
        let tejado = b - a < TEJADO;
        // Los cortes: los bordes del tramo y los de cada peldano.
        let mut cortes = vec![a0, b0];
        for e in &def.escaleras {
            let n = peldanos(e);
            for k in 0..n {
                let x = e.x + (k as f32 + 0.5) * e.ancho / n as f32;
                if x > a0 && x < b0 {
                    cortes.push(x);
                }
            }
        }
        cortes.sort_by(f32::total_cmp);
        for par in cortes.windows(2) {
            let (u, v) = (par[0], par[1]);
            let y = perfil(def, (u + v) * 0.5);
            if tejado {
                tejado_de_zinc(l, u, v, y, fondo, a);
            } else if en_escalera(def, (u + v) * 0.5) {
                rect(l, u, y, v - u, fondo - y, PIEDRA);
                rect(l, u, y, v - u, 7.0, PELDANO);
                linea(l, (u, y + 7.0), (v, y + 7.0), 1.5, TINTA);
                rect(l, u, y + 7.0, v - u, 6.0, fade(TINTA, 0.35));
            } else {
                muro(l, u, v, y, fondo);
                rect(l, u, y, v - u, 12.0, BORDILLO);
                linea(l, (u, y + 12.0), (v, y + 12.0), 1.5, TINTA);
                // Adoquines a lo largo del bordillo.
                let mut x = (u / 26.0).ceil() * 26.0;
                while x < v - 4.0 {
                    linea(l, (x, y + 3.0), (x, y + 10.0), 1.0, fade(TINTA, 0.35));
                    x += 26.0;
                }
            }
            linea(l, (u, y), (v, y), 3.5, TINTA);
        }
        // Las contrahuellas: donde cambia la altura, una raya de tinta.
        for par in cortes.windows(2) {
            let x = par[1];
            if x >= b0 {
                continue;
            }
            let (y0, y1) = (perfil(def, x - 0.5), perfil(def, x + 0.5));
            if (y0 - y1).abs() > 0.5 {
                linea(l, (x, y0.min(y1)), (x, y0.max(y1)), 3.0, TINTA);
            }
        }
        // Los bordes del foso, gordos: son informacion.
        for x in [a, b] {
            if x > izq && x < der {
                linea(l, (x, perfil(def, x)), (x, fondo), 4.5, TINTA);
            }
        }
    }
}

/// Un muro de contencion de sillares, bajo la acera.
fn muro(l: &Layout, u: f32, v: f32, y: f32, fondo: f32) {
    rect(l, u, y, v - u, fondo - y, PIEDRA);
    const FILA: f32 = 22.0;
    let mut fy = (y / FILA).floor() * FILA + FILA;
    while fy < fondo {
        if fy > y + 14.0 {
            linea(l, (u, fy), (v, fy), 1.5, JUNTA);
        }
        let desfase = if (fy / FILA) as i64 % 2 == 0 {
            0.0
        } else {
            24.0
        };
        let mut x = ((u - desfase) / 48.0).ceil() * 48.0 + desfase;
        while x < v {
            let arriba = (fy - FILA).max(y + 14.0);
            if fy > arriba {
                linea(l, (x, arriba), (x, fy), 1.5, JUNTA);
            }
            x += 48.0;
        }
        fy += FILA;
    }
}

/// Un tejado de zinc que se pisa: la cumbrera clara y, debajo, la fachada de
/// la casa con sus ventanas.
fn tejado_de_zinc(l: &Layout, u: f32, v: f32, y: f32, fondo: f32, a: f32) {
    rect(l, u, y, v - u, fondo - y, MURO_OSCURO);
    rect(l, u, y, v - u, 14.0, ZINC_CLARO);
    // Las juntas alzadas del zinc.
    let mut x = (u / 18.0).ceil() * 18.0;
    while x < v {
        linea(l, (x, y + 2.0), (x, y + 14.0), 1.2, fade(TINTA, 0.5));
        x += 18.0;
    }
    linea(l, (u, y + 14.0), (v, y + 14.0), 2.0, TINTA);
    rect(l, u, y + 14.0, v - u, 10.0, ZINC);
    linea(l, (u, y + 24.0), (v, y + 24.0), 2.0, TINTA);
    // Ventanas de buhardilla y del piso de abajo, las de siempre.
    let mut x = ((u - a) / 70.0).ceil() * 70.0 + a + 35.0;
    while x < v - 10.0 {
        let mut rng = Pcg32::new(x as i64 as u64);
        let c = if rng.next_f32() < 0.5 {
            VENTANA
        } else {
            APAGADA
        };
        rect(l, x - 10.0, y + 40.0, 20.0, 32.0, c);
        draw_rect(l, x - 10.0, y + 40.0, 20.0, 32.0, 2.0);
        rect(l, x - 13.0, y + 40.0, 3.0, 32.0, PERSIANA);
        rect(l, x + 10.0, y + 40.0, 3.0, 32.0, PERSIANA);
        x += 70.0;
    }
}

/// Las escaleras de Montmartre: la barandilla de hierro por el medio y una
/// farola abajo y otra arriba, que es su foto.
fn barandillas(p: &Paseo, l: &Layout, cam: f32, pulso: f32) {
    let def = &p.def;
    for e in &def.escaleras {
        if e.x + e.ancho + 100.0 < cam || e.x - 100.0 > cam + VISTA_W {
            continue;
        }
        let alto = 36.0;
        let (x0, x1) = (e.x - 14.0, e.x + e.ancho + 14.0);
        let (y0, y1) = (def.altura(x0) - alto, def.altura(x1) - alto);
        let mut x = e.x;
        while x <= e.x + e.ancho + 0.5 {
            let s = perfil(def, x);
            linea(l, (x, s), (x, def.altura(x) - alto), 2.5, TINTA);
            x += e.ancho / 6.0;
        }
        linea(l, (x0, y0), (x1, y1), 4.0, TINTA);
        linea(l, (x0, y0 + 12.0), (x1, y1 + 12.0), 1.5, TINTA);
        for (x, s) in [
            (e.x - 40.0, def.altura(e.x)),
            (e.x + e.ancho + 40.0, def.altura(e.x + e.ancho)),
        ] {
            farola(l, x, s, pulso);
        }
    }
}

/// Mesas de terraza, toldos y chimeneas. Lo que va entre tejados es una
/// chimenea, y si cuelga sobre el vacio, una de las altas de la casa de abajo;
/// lo bajo, una mesa de marmol con su copa; lo alto, un toldo a rayas, que en
/// un cartel se lee de lejos.
fn plataformas(p: &Paseo, l: &Layout) {
    let def = &p.def;
    for pl in &def.plataformas {
        let (x, y, w) = (pl.x, pl.y, pl.ancho);
        let cx = x + w * 0.5;
        let debajo = def.suelo_en(cx);
        if es_tejado(def, x) || es_tejado(def, x + w) {
            // El cuerpo de ladrillo solo donde hay tejado debajo. Sobre el
            // vacio va volada, con escuadras: una chimenea hasta el fondo del
            // foso lo tapaba y parecia que se podia andar por ahi.
            for &(a, b) in &def.suelo {
                let (u, v) = (x.max(a), (x + w).min(b));
                if v <= u {
                    continue;
                }
                let base = def.altura(u);
                rect(l, u - 2.0, y - 2.0, v - u + 4.0, base - y + 2.0, TINTA);
                rect(l, u + 2.0, y + 10.0, v - u - 4.0, base - y - 10.0, CHIMENEA);
                let mut fy = y + 22.0;
                while fy < base {
                    linea(l, (u + 2.0, fy), (v - 2.0, fy), 1.2, fade(TINTA, 0.6));
                    fy += 12.0;
                }
            }
            let mut bx = x + 12.0;
            while bx < x + w {
                if !def.hay_suelo(bx) {
                    tri(
                        l,
                        (bx - 8.0, y + 10.0),
                        (bx + 8.0, y + 10.0),
                        (bx, y + 30.0),
                        TINTA,
                    );
                }
                bx += 36.0;
            }
            rect(l, x - 2.0, y - 2.0, w + 4.0, 14.0, TINTA);
            rect(l, x, y, w, 10.0, PELDANO);
            let n = ((w / 24.0) as i32).max(1);
            for i in 0..n {
                let px = x + (i as f32 + 0.5) * w / n as f32;
                rect(l, px - 4.0, y - 12.0, 8.0, 12.0, TINTA);
                rect(l, px - 2.5, y - 10.5, 5.0, 10.0, CHIMENEA);
            }
            linea(l, (x, y), (x + w, y), 3.0, TINTA);
        } else if debajo.is_some_and(|g| g - y < 70.0) {
            let g = debajo.unwrap_or(y);
            rect(l, cx - 3.0, y, 6.0, g - y, TINTA);
            rect(l, cx - 14.0, g - 4.0, 28.0, 4.0, TINTA);
            rect(l, x - 2.0, y - 2.0, w + 4.0, 11.0, TINTA);
            rect(l, x, y, w, 7.0, MARMOL);
            // Una copa de champan: el cancan se bebe.
            linea(l, (cx - 12.0, y - 2.0), (cx - 12.0, y - 10.0), 1.5, TINTA);
            tri(
                l,
                (cx - 18.0, y - 18.0),
                (cx - 6.0, y - 18.0),
                (cx - 12.0, y - 9.0),
                TINTA,
            );
            tri(
                l,
                (cx - 16.5, y - 17.0),
                (cx - 7.5, y - 17.0),
                (cx - 12.0, y - 11.0),
                AMARILLO,
            );
        } else {
            toldo(l, x, y, w);
        }
    }
}

/// Un toldo a rayas rojas y crema con el faldon festoneado.
fn toldo(l: &Layout, x: f32, y: f32, w: f32) {
    let n = ((w / 18.0) as i32).max(2);
    let paso = w / n as f32;
    let color = |i: i32| if i % 2 == 0 { ROJO } else { CREMA };
    // Primero los festones enteros y luego las rayas encima: de cada circulo
    // queda la mitad de abajo, que es el faldon.
    for i in 0..n {
        let fx = x + (i as f32 + 0.5) * paso;
        circulo(l, fx, y + 15.0, paso * 0.5 + 2.2, TINTA);
    }
    for i in 0..n {
        circulo(
            l,
            x + (i as f32 + 0.5) * paso,
            y + 15.0,
            paso * 0.5,
            color(i),
        );
    }
    rect(l, x - 3.0, y - 3.0, w + 6.0, 18.0, TINTA);
    for i in 0..n {
        rect(l, x + i as f32 * paso, y, paso + 0.5, 15.0, color(i));
    }
    linea(l, (x, y), (x + w, y), 3.0, TINTA);
    // Las escuadras de hierro que lo sujetan a la pared.
    for bx in [x + 10.0, x + w - 10.0] {
        linea(
            l,
            (bx, y + 18.0),
            (bx + if bx < x + w * 0.5 { 14.0 } else { -14.0 }, y + 44.0),
            2.5,
            TINTA,
        );
    }
}

/// La puerta: el Moulin Rouge, con su molino encima girando las aspas rojas,
/// el letrero de bombillas y un telon de terciopelo que **se abre** segun te
/// acercas. Dentro, la luz y la fila de coristas pateando.
fn molino_rojo(p: &Paseo, l: &Layout, alpha: f32, t: f32, pulso: f32) {
    let x = p.def.puerta;
    let cam = calle::camara(p, alpha);
    if x + 600.0 < cam || x - 400.0 > cam + VISTA_W {
        return;
    }
    let s = p.def.altura(x);
    let (cx, ancho, boca) = (x + 40.0, 124.0, s - 136.0);

    // La fachada, roja y oscura, que cierra la calle.
    let alto = 230.0;
    rect(l, x - 220.0, s - alto, 900.0, alto, ROJO_OSCURO);
    rect(l, x - 220.0, s - alto, 900.0, 18.0, ROJO);
    linea(
        l,
        (x - 220.0, s - alto + 18.0),
        (x + 680.0, s - alto + 18.0),
        2.5,
        TINTA,
    );
    linea(l, (x - 220.0, s - alto), (x - 220.0, s), 4.0, TINTA);
    linea(l, (x - 220.0, s - alto), (x + 680.0, s - alto), 4.0, TINTA);
    // Las bombillas de la cornisa, corriendo como en el club.
    let paso = (t / 8.0) as i32;
    let mut i = 0;
    let mut bx = x - 210.0;
    while bx < x + 680.0 {
        let encendida = (i + paso).rem_euclid(3) == 0;
        let c = if encendida { LUZ } else { fade(ORO, 0.4) };
        if (bx - cx).abs() > 130.0 {
            circulo(l, bx, s - alto + 9.0, 3.5, c);
        }
        bx += 20.0;
        i += 1;
    }
    // Ventanas encendidas a los lados.
    for wx in [x - 150.0, x - 70.0, cx + 140.0, cx + 230.0, cx + 320.0] {
        rect(
            l,
            wx - 22.0,
            s - 150.0,
            44.0,
            70.0,
            fade(VENTANA, 0.85 + 0.15 * pulso),
        );
        draw_rect(l, wx - 22.0, s - 150.0, 44.0, 70.0, 3.0);
        linea(l, (wx, s - 150.0), (wx, s - 80.0), 2.0, TINTA);
    }

    // El molino, encima de la entrada.
    let (tb, tt) = (s - alto, s - alto - 110.0);
    tri(l, (cx - 50.0, tb), (cx - 36.0, tt), (cx + 50.0, tb), ROJO);
    tri(l, (cx + 50.0, tb), (cx - 36.0, tt), (cx + 36.0, tt), ROJO);
    linea(l, (cx - 50.0, tb), (cx - 36.0, tt), 3.0, TINTA);
    linea(l, (cx + 50.0, tb), (cx + 36.0, tt), 3.0, TINTA);
    tri(
        l,
        (cx - 44.0, tt),
        (cx, tt - 34.0),
        (cx + 44.0, tt),
        ROJO_OSCURO,
    );
    linea(l, (cx - 44.0, tt), (cx, tt - 34.0), 3.0, TINTA);
    linea(l, (cx + 44.0, tt), (cx, tt - 34.0), 3.0, TINTA);
    circulo(l, cx, tb - 50.0, 12.0, fade(VENTANA, 0.9));
    arco(l, cx, tb - 50.0, 12.0, 2.5, TINTA);
    let eje = (cx, tt - 6.0);
    let giro = t * 0.02;
    aspas(l, eje, 128.0, giro, ROJO, 2.5);
    // Bombillas en las puntas de las aspas.
    for k in 0..4 {
        let (sn, cs) = sin_cos(giro + PI * 0.5 * k as f32);
        circulo(
            l,
            eje.0 + cs * 128.0,
            eje.1 + sn * 128.0,
            4.0,
            fade(LUZ, 0.7 + 0.3 * pulso),
        );
    }

    // El letrero, en la cornisa, entre el molino y la puerta.
    let ly = s - alto - 2.0;
    rect(l, cx - 124.0, ly, 248.0, 36.0, TINTA);
    rect(l, cx - 120.0, ly + 4.0, 240.0, 28.0, ROJO);
    let q = l.to_screen(cx, ly + 26.0);
    fuentes::centrado(
        "MOULIN ROUGE",
        q.x,
        q.y,
        l.len(22.0),
        Cara::Titulo,
        fade(CREMA, 0.9 + 0.1 * pulso),
    );

    // Dentro, la luz y la fila de coristas pateando en contraluz.
    let abierta = ((p.jugadora.render_pos(alpha).x - (x - 420.0)) / 320.0).clamp(0.0, 1.0);
    circulo(l, cx, boca, ancho * 0.5, fade(LUZ, 0.95));
    rect(l, cx - ancho * 0.5, boca, ancho, s - boca, fade(LUZ, 0.95));
    for i in 0..4 {
        let bx = cx - 48.0 + i as f32 * 32.0;
        let arriba = ((t * 0.13 + i as f32 * 1.4).sin() * 0.5 + 0.5) * 34.0;
        let sombra = fade(TINTA, 0.55);
        tri(
            l,
            (bx, s - 60.0),
            (bx - 12.0, s - 36.0),
            (bx + 12.0, s - 36.0),
            sombra,
        );
        circulo(l, bx, s - 68.0, 5.0, sombra);
        linea(l, (bx - 3.0, s - 38.0), (bx - 4.0, s), 3.0, sombra);
        linea(
            l,
            (bx + 3.0, s - 40.0),
            (bx + 14.0, s - 12.0 - arriba),
            3.0,
            sombra,
        );
    }

    // El telon: dos cortinas de terciopelo que se recogen hacia los lados.
    let hoja = ancho * 0.5 * (1.0 - abierta * 0.8);
    for (x0, dir) in [(cx - ancho * 0.5, 1.0), (cx + ancho * 0.5, -1.0)] {
        let x1 = x0 + hoja * dir;
        let (a, b) = (x0.min(x1), x0.max(x1));
        rect(
            l,
            a,
            boca - ancho * 0.25,
            b - a,
            s - boca + ancho * 0.25,
            ROJO,
        );
        let n = 4;
        for k in 1..n {
            let fx = a + (b - a) * k as f32 / n as f32;
            linea(l, (fx, boca - 20.0), (fx + dir * 3.0, s), 2.0, ROJO_OSCURO);
        }
        circulo(l, x1, boca + 60.0, 5.0, ORO);
    }
    arco(l, cx, boca, ancho * 0.5, 5.0, TINTA);
    for dx in [-ancho * 0.5, ancho * 0.5] {
        linea(l, (cx + dx, boca), (cx + dx, s), 5.0, TINTA);
    }

    // La luz se derrama a la calle segun se abre, y al llegar lo inunda.
    let llegada = p
        .completado
        .map_or(0.0, |d| (((p.tick - d) as f32 + alpha) / 40.0).min(1.0));
    let derrame = fade(LUZ, 0.10 + 0.25 * abierta + 0.3 * llegada);
    let (izq, der) = (cx - ancho * 0.5, cx + ancho * 0.5);
    let abajo = s + 80.0;
    tri(l, (izq, s), (der, s), (der + 160.0, abajo), derrame);
    tri(
        l,
        (izq, s),
        (der + 160.0, abajo),
        (izq - 160.0, abajo),
        derrame,
    );
}

// ---------------------------------------------------------------------------
// Los enemigos
// ---------------------------------------------------------------------------

/// Una corista del cancan: botines y medias negras, la falda en la tinta del
/// baile sujeta en alto con una mano y las enaguas blancas de volantes, que es
/// lo que se ve cuando patea. La pierna es la de `Enemigo::pierna`, la misma
/// que pega: lo que se ve arriba es lo que hace dano.
pub fn corista(
    e: &Enemigo,
    l: &Layout,
    edad: f32,
    (rosa, granate): (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let Some((cadera, pie)) = e.pierna() else {
        return;
    };
    let d = e.dir;
    let k = e.gesto;
    // El chasse: saltitos al compas mientras no patea.
    let bote = if k == 0.0 {
        (edad * PI / 12.0).sin().abs() * 5.0
    } else {
        0.0
    };
    let x = e.pos.x;
    let pies = e.pos.y + 44.0 - bote;
    let (cadera, pie) = ((cadera.x, cadera.y - bote), (pie.x, pie.y - bote));
    let (falda, cuerpo) = (tinte(rosa), tinte(granate));
    let (enagua, piel, media) = (tinte(HUESO), tinte(PIEL), tinte(MEDIA));
    let cintura = pies - 58.0;
    // El bajo de la falda: delante se levanta con la pierna, y la mano de
    // atras lo sujeta en alto, mas cuanto mas patea. Es la bandera del cancan.
    let bajo_atras = (x - d * (22.0 + 10.0 * k), pies - 30.0 - 38.0 * k);
    let bajo_delante = (x + d * (20.0 + 6.0 * k), pies - 30.0 - 16.0 * k);
    let mano = (bajo_atras.0 - d * 4.0, bajo_atras.1 - 6.0);

    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        // Las dos piernas con sus medias negras y sus botines.
        let apoyo = (x - d * 2.0, pies - 3.0);
        linea(
            l,
            (x - d * 3.0, pies - 40.0),
            apoyo,
            7.0 + 2.0 * g,
            c(media),
        );
        let pb = l.to_screen(apoyo.0 + d * 3.0, apoyo.1);
        draw_ellipse(pb.x, pb.y, l.len(7.5 + g), l.len(4.0 + g), 0.0, c(media));
        linea(l, cadera, pie, 7.0 + 2.0 * g, c(media));
        circulo(l, pie.0, pie.1, 5.5 + g, c(media));
        // Las enaguas: un abanico de volantes blancos entre las piernas,
        // que es lo que ensena la patada.
        for i in 0..5 {
            let f = i as f32 / 4.0;
            let (vx, vy) = (
                bajo_atras.0 + (bajo_delante.0 - bajo_atras.0) * f,
                bajo_atras.1 + (bajo_delante.1 - bajo_atras.1) * f + 4.0,
            );
            circulo(l, vx, vy, 6.0 + 3.0 * k + g, c(enagua));
        }
        // La falda, una campana de la cintura al bajo.
        tri(
            l,
            (x - d * 6.0, cintura - g),
            bajo_atras,
            bajo_delante,
            c(falda),
        );
        tri(
            l,
            (x - d * 6.0, cintura - g),
            (x + d * 8.0, cintura - g),
            bajo_delante,
            c(falda),
        );
        // El corpino, los brazos y la cabeza.
        tri(
            l,
            (x - 9.0 - g, pies - 78.0 - g),
            (x + 9.0 + g, pies - 78.0 - g),
            (x, cintura + 6.0 + g),
            c(cuerpo),
        );
        linea(l, (x - d * 6.0, pies - 74.0), mano, 3.5 + 2.0 * g, c(piel));
        linea(
            l,
            (x + d * 6.0, pies - 74.0),
            (x + d * 16.0, pies - 86.0),
            3.5 + 2.0 * g,
            c(piel),
        );
        circulo(l, x + d * 1.0, pies - 86.0, 8.0 + g, c(piel));
        // El mono y la pluma, en las tintas del baile. La pluma cabecea con
        // el chasse.
        circulo(l, x - d * 5.0, pies - 92.0, 5.5 + g, c(cuerpo));
        // Dos plumas en abanico: con una sola parecia una oreja.
        for (dx, a, largo) in [(8.0, 18.0, 13.0), (13.0, 42.0, 10.0)] {
            let giro = d * (a + 12.0 * k) + bote * 2.0;
            let pp = l.to_screen(x - d * dx, pies - 104.0 + dx * 0.4);
            draw_ellipse(
                pp.x,
                pp.y,
                l.len(3.2 + g),
                l.len(largo + g),
                -giro,
                c(falda),
            );
        }
    });
    // El festoneado del bajo, en tinta, y la liga roja de la pierna alzada.
    for i in 0..6 {
        let f = (i as f32 + 0.5) / 6.0;
        let (vx, vy) = (
            bajo_atras.0 + (bajo_delante.0 - bajo_atras.0) * f,
            bajo_atras.1 + (bajo_delante.1 - bajo_atras.1) * f,
        );
        circulo(l, vx, vy, 1.4, TINTA);
    }
    let liga = (
        cadera.0 + (pie.0 - cadera.0) * 0.45,
        cadera.1 + (pie.1 - cadera.1) * 0.45,
    );
    circulo(l, liga.0, liga.1, 4.0, TINTA);
    circulo(l, liga.0, liga.1, 2.6, tinte(ROJO));
    // Ojo y boca, en tinta: va cantando.
    circulo(l, x + d * 4.5, pies - 88.0, 1.4, TINTA);
    circulo(l, x + d * 5.0, pies - 82.0, 1.6, TINTA);
}

// Una botella de champan: vidrio verde, papel de oro en el cuello y el corcho
/// que **va saliendo** mientras tiembla. Ese corcho asomando es el aviso.
pub fn botella(
    e: &Enemigo,
    l: &Layout,
    t: f32,
    _tintas: (Color, Color),
    tinte: &dyn Fn(Color) -> Color,
) {
    let aviso = (1.0 - e.reloj as f32 / 24.0).clamp(0.0, 1.0);
    let temblor = if aviso > 0.0 {
        (t * 1.9).sin() * 2.0 * aviso
    } else {
        0.0
    };
    let x = e.pos.x + temblor;
    let fondo = e.pos.y + 18.0;
    let (vidrio, oro, crema) = (tinte(VIDRIO), tinte(ORO), tinte(CREMA));
    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        rect(
            l,
            x - 8.0 - g,
            fondo - 24.0,
            16.0 + 2.0 * g,
            24.0 + g,
            c(vidrio),
        );
        circulo(l, x, fondo - 24.0, 8.0 + g, c(vidrio));
        rect(
            l,
            x - 3.5 - g,
            fondo - 40.0 - g,
            7.0 + 2.0 * g,
            16.0 + g,
            c(vidrio),
        );
        rect(
            l,
            x - 4.5 - g,
            fondo - 44.0 - g,
            9.0 + 2.0 * g,
            10.0 + g,
            c(oro),
        );
        // El corcho, saliendo.
        let sube = aviso * 7.0;
        rect(
            l,
            x - 4.0 - g,
            fondo - 50.0 - sube - g,
            8.0 + 2.0 * g,
            7.0 + g,
            c(crema),
        );
        rect(l, x - 7.0, fondo - 18.0, 14.0, 10.0, c(crema));
    });
    rect(l, x - 7.0, fondo - 15.0, 14.0, 3.0, ROJO);
    linea(
        l,
        (x - 4.0, fondo - 22.0),
        (x - 4.0, fondo - 4.0),
        1.5,
        fade(WHITE, 0.35),
    );
    // Recien descorchada, la espuma se sale.
    if e.gesto > 0.0 {
        for i in 0..5 {
            let (s, c) = sin_cos(-PI * 0.5 + (i as f32 - 2.0) * 0.45);
            let r = 8.0 + 14.0 * (1.0 - e.gesto);
            circulo(
                l,
                x + c * r,
                fondo - 46.0 + s * r,
                3.0 * e.gesto + 1.0,
                fade(HUESO, e.gesto),
            );
        }
    }
}

/// Un pintor de la place du Tertre: blusa azul, boina roja y barba, el
/// caballete a la espalda con el lienzo empezado y la paleta en la mano. Echa
/// el pincel atras justo antes de sacudirlo, que es el aviso.
pub fn pintor(e: &Enemigo, l: &Layout, (rosa, _): (Color, Color), tinte: &dyn Fn(Color) -> Color) {
    let (x, pies, d) = (e.pos.x, e.pos.y + 36.0, e.dir);
    // El caballete, detras.
    let cx = x - d * 30.0;
    for (a, b) in [
        ((cx - 12.0, pies), (cx, pies - 78.0)),
        ((cx + 12.0, pies), (cx, pies - 78.0)),
        ((cx + d * 10.0, pies), (cx - d * 2.0, pies - 60.0)),
    ] {
        linea(l, a, b, 6.0, TINTA);
        linea(l, a, b, 3.0, tinte(MADERA));
    }
    rect(l, cx - 17.0, pies - 74.0, 34.0, 40.0, TINTA);
    rect(l, cx - 15.0, pies - 72.0, 30.0, 36.0, tinte(CREMA));
    // El cuadro: la Butte con su molino, en tres manchas.
    rect(l, cx - 15.0, pies - 50.0, 30.0, 14.0, tinte(PINTURAS[1]));
    circulo(l, cx + 5.0, pies - 62.0, 6.0, tinte(PINTURAS[0]));
    tri(
        l,
        (cx - 12.0, pies - 36.0),
        (cx - 4.0, pies - 58.0),
        (cx + 4.0, pies - 36.0),
        tinte(rosa),
    );

    let aviso = (1.0 - e.reloj as f32 / 16.0).clamp(0.0, 1.0);
    let mano = if e.gesto > 0.0 {
        (x + d * 28.0, pies - 70.0 + 10.0 * e.gesto)
    } else {
        (x - d * (4.0 + 14.0 * aviso), pies - 72.0 - 12.0 * aviso)
    };
    let (blusa, piel) = (tinte(BLUSON), tinte(PIEL));
    con_tinta(|g, es_tinta| {
        let c = |col: Color| if es_tinta { TINTA } else { col };
        linea(
            l,
            (x - 4.0, pies - 30.0),
            (x - 5.0, pies),
            6.0 + 2.0 * g,
            c(MEDIA),
        );
        linea(
            l,
            (x + 4.0, pies - 30.0),
            (x + 5.0, pies),
            6.0 + 2.0 * g,
            c(MEDIA),
        );
        tri(
            l,
            (x - 11.0 - g, pies - 64.0 - g),
            (x + 11.0 + g, pies - 64.0 - g),
            (x + 16.0 + g, pies - 24.0 + g),
            c(blusa),
        );
        tri(
            l,
            (x - 11.0 - g, pies - 64.0 - g),
            (x + 16.0 + g, pies - 24.0 + g),
            (x - 16.0 - g, pies - 24.0 + g),
            c(blusa),
        );
        linea(l, (x + d * 6.0, pies - 58.0), mano, 4.0 + 2.0 * g, c(blusa));
        circulo(l, x, pies - 72.0, 8.0 + g, c(piel));
        // La paleta, en la otra mano.
        let pp = l.to_screen(x - d * 14.0, pies - 44.0);
        draw_ellipse(pp.x, pp.y, l.len(10.0 + g), l.len(6.0 + g), 0.0, c(CREMA));
    });
    for (i, col) in PINTURAS.iter().enumerate() {
        circulo(l, x - d * (18.0 - i as f32 * 4.0), pies - 45.0, 1.8, *col);
    }
    // La boina, la barba y el pincel.
    let pb = l.to_screen(x - d * 1.0, pies - 79.0);
    draw_ellipse(pb.x, pb.y, l.len(11.0), l.len(4.5), 0.0, TINTA);
    draw_ellipse(pb.x, pb.y, l.len(9.5), l.len(3.2), 0.0, tinte(ROJO));
    tri(
        l,
        (x - 5.0, pies - 70.0),
        (x + 5.0 * d + 3.0, pies - 70.0),
        (x + d * 2.0, pies - 60.0),
        TINTA,
    );
    let punta = (mano.0 + d * 12.0, mano.1 - 8.0);
    linea(l, mano, punta, 2.5, TINTA);
    circulo(
        l,
        punta.0,
        punta.1,
        3.0,
        PINTURAS[(e.pos.x as i64).rem_euclid(3) as usize],
    );
}

/// Lo fugaz del cancan: volantes, corchos, burbujas y pintura. Todo con su
/// canto de tinta, y lo que se va apagando se desvanece en sus ultimas decimas:
/// que se vea que se acaba es lo que deja fiarse de que se acaba.
pub fn fugaces(p: &Paseo, l: &Layout, t: f32, (rosa, _): (Color, Color)) {
    for f in &p.fugaces {
        let (x, y) = (f.pos.x, f.pos.y);
        let k = (f.ttl / 0.2).min(1.0);
        match f.clase {
            Clase::Volante => {
                // Una ola de volantes blancos con su cinta rosa, a ras de
                // acera: tres festones que se empujan.
                let d = f.vel.x.signum();
                for (i, r) in [9.0, 12.0, 9.0].into_iter().enumerate() {
                    let vx = x + (i as f32 - 1.0) * 12.0 * d;
                    let vy = y + 10.0 - r + (t * 0.5 + i as f32).sin() * 1.5;
                    circulo(l, vx, vy, r + 2.2, fade(TINTA, k));
                    circulo(l, vx, vy, r, fade(HUESO, k));
                }
                linea(
                    l,
                    (x - 18.0, y + 6.0),
                    (x + 18.0, y + 6.0),
                    3.0,
                    fade(rosa, k),
                );
            }
            Clase::Corcho => {
                let (s, c) = sin_cos(t * 0.45 + x * 0.01);
                let (a, b) = ((x - c * 5.0, y - s * 5.0), (x + c * 5.0, y + s * 5.0));
                linea(l, a, b, 10.0, TINTA);
                linea(l, a, b, 6.0, if f.rosa { ROSA } else { CREMA });
                if f.rosa {
                    let q = 0.5 + 0.5 * (t * 0.2).sin();
                    let s = l.to_screen(x, y);
                    draw_circle_lines(
                        s.x,
                        s.y,
                        l.len(17.0 + 4.0 * q),
                        2.0,
                        fade(WHITE, 0.4 + 0.4 * q),
                    );
                }
            }
            Clase::Burbuja => {
                let s = l.to_screen(x, y);
                let a = (f.ttl / 0.15).min(1.0);
                draw_circle(s.x, s.y, l.len(7.5), fade(TINTA, a));
                draw_circle(s.x, s.y, l.len(5.5), fade(AMARILLO, a));
                draw_circle(
                    s.x - l.len(1.8),
                    s.y - l.len(1.8),
                    l.len(1.8),
                    fade(WHITE, 0.8 * a),
                );
            }
            Clase::Gota => {
                let c = PINTURAS[((f.vel.x * 0.1) as i64).rem_euclid(3) as usize];
                circulo(l, x, y, 7.5, TINTA);
                circulo(l, x, y, 5.5, c);
            }
            Clase::Mancha => {
                let c = PINTURAS[((x * 0.1) as i64).rem_euclid(3) as usize];
                let a = (f.ttl / 0.4).min(1.0);
                let s = l.to_screen(x, y + 2.0);
                draw_ellipse(s.x, s.y, l.len(18.0), l.len(5.5), 0.0, fade(TINTA, a));
                draw_ellipse(s.x, s.y, l.len(15.5), l.len(3.8), 0.0, fade(c, a));
                for (dx, r) in [(-20.0, 2.5), (19.0, 2.0), (8.0, 1.6)] {
                    circulo(l, x + dx, y - 1.0 - r, r, fade(c, a));
                }
            }
        }
    }
}
