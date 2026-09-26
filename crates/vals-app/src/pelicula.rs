//! La pelicula: grano, parpadeo, rayas y vineta, como una copia de 1930.
//!
//! Es la firma de Cuphead que faltaba, y la mas barata: la imagen entera pasa a
//! parecer **proyectada**. Va por encima de todo —arena, papel, cartelas,
//! telon— porque es la pelicula la que tiene grano, no las cosas que salen en
//! ella.
//!
//! A diferencia del grano del papel (`paleta::grano`), este **se mueve**, y es
//! a proposito: el papel esta quieto y la pelicula corre. Pero corre a 24
//! fotogramas por segundo y no a los hercios del monitor: un grano a 144 Hz se
//! lee como ruido de video, y a 24 se lee como cine.
//!
//! Sobre la arena va muy tenue. Aqui la legibilidad de las balas manda
//!: la pelicula es ambiente, y el ambiente no puede tapar una
//! bala.

use macroquad::prelude::*;
use vals_core::rng::Pcg32;

/// Los fotogramas por segundo de la pelicula, que no son los del juego.
const FPS: f64 = 24.0;
/// Motas de grano por fotograma.
const MOTAS: usize = 220;
/// Cuantos fotogramas dura una raya. Una raya de verdad es un aranazo en la
/// copia: aparece, se queda un poco temblando y se va.
const DURA_RAYA: u64 = 7;

const OSCURO: Color = Color::new(0.10, 0.07, 0.05, 1.0);
const CLARO: Color = Color::new(1.0, 0.96, 0.86, 1.0);

/// Dibuja la pelicula encima de todo lo demas.
pub fn dibujar() {
    let fotograma = (get_time() * FPS) as u64;
    let (w, h) = (screen_width(), screen_height());
    // Un generador por fotograma, sembrado con el numero de fotograma: dentro
    // del mismo fotograma el grano no cambia aunque el monitor pinte tres
    // veces, y entre fotogramas cambia entero.
    let mut rng = Pcg32::new(fotograma.wrapping_mul(0x9E37_79B9_7F4A_7C15));

    // El parpadeo: la luz del proyector no es constante.
    let parpadeo = rng.next_f32();
    draw_rectangle(
        0.0,
        0.0,
        w,
        h,
        Color {
            a: parpadeo * 0.04,
            ..OSCURO
        },
    );

    // El grano. Casi todo oscuro y algo claro, que es polvo en la copia.
    for _ in 0..MOTAS {
        let (x, y) = (rng.next_f32() * w, rng.next_f32() * h);
        let r = 0.5 + rng.next_f32() * 0.9;
        let c = if rng.next_f32() < 0.3 {
            Color { a: 0.10, ..CLARO }
        } else {
            Color { a: 0.11, ..OSCURO }
        };
        draw_circle(x, y, r, c);
    }

    // Las rayas. Se siembran por bloques de fotogramas para que duren, y
    // tiemblan un poco en cada uno.
    let mut aranazo =
        Pcg32::new((fotograma / DURA_RAYA).wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x51);
    for _ in 0..2 {
        if aranazo.next_f32() < 0.82 {
            continue;
        }
        let x = aranazo.next_f32() * w;
        let claro = aranazo.next_f32() < 0.5;
        let temblor = (rng.next_f32() - 0.5) * 3.0;
        let c = if claro { CLARO } else { OSCURO };
        draw_line(x, 0.0, x + temblor, h, 1.0, Color { a: 0.14, ..c });
    }

    // Y la vineta de la pantalla entera: el proyector ilumina el centro y se
    // queda corto en las esquinas.
    let grosor = w.min(h) * 0.11;
    for i in 0..6 {
        let g = grosor * (i + 1) as f32 / 6.0;
        let a = 0.045 * (6 - i) as f32 / 6.0;
        let c = Color { a, ..OSCURO };
        draw_rectangle(0.0, 0.0, w, g, c);
        draw_rectangle(0.0, h - g, w, g, c);
        draw_rectangle(0.0, 0.0, g, h, c);
        draw_rectangle(w - g, 0.0, g, h, c);
    }
}
