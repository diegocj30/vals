//! El salon: el fondo de la arena.
//!
//! Antes era un rectangulo con una rejilla. Veinte lineas, y hacia su trabajo
//! —se veia donde estaban las cosas— pero no decia en ningun momento que esto
//! ocurre en un baile.
//!
//! Ahora hay pared al fondo, suelo en perspectiva, una lampara colgando y
//! focos en el suelo. Y **la sala late con la musica**: se sabe el compas de
//! cada tema —cuantos ticks dura un tiempo y cuantos tiempos tiene un compas—,
//! asi que los focos y la lampara pulsan a tiempo sin analizar una sola muestra
//! de audio. Los dos numeros salen de la misma partitura que esta sonando.
//!
//! Todo esto es **fondo**: la jugabilidad sigue siendo plana. Las balas y la
//! bailarina se mueven en coordenadas de arena y no saben que hay perspectiva,
//! igual que en la pista. Y por eso el borde de la arena sigue siendo un
//! rectangulo nitido: es lo unico de aqui que es informacion y no decorado.

use macroquad::prelude::*;
use vals_core::math::{TAU, sin_cos};
use vals_core::{ARENA_H, ARENA_W};

use crate::draw::Layout;
use crate::music::{self, Tema};

/// Altura del horizonte. Por encima esta la pared; por debajo, la tarima.
const HORIZONTE: f32 = ARENA_H * 0.13;
/// Lo que se estrecha la tarima al fondo.
const FONDO: f32 = 0.34;
/// Vetas de la tarima, por eje.
const VETAS: usize = 11;
/// Focos en el suelo.
const FOCOS: usize = 3;

// Los colores salen de `paleta`: la sala es madera y luz de bombilla, no un
// vectorial azul. Sigue siendo oscura porque las balas tienen que brillar
// encima, pero deja de ser fria, que es lo que la hacia parecer el espacio.
use crate::paleta::{LUZ, ORO, PARED, TARIMA, VETA};
const SOMBRA: Color = color_u8!(10, 5, 6, 255);

/// Lleva un punto del suelo a la pantalla.
///
/// `d` es la profundidad: 0 al fondo, 1 delante. Es la misma idea que el
/// `suelo()` de la pista, pero con su propio horizonte, porque aqui la pared
/// tiene que caber arriba y el jefe se planta justo delante de ella.
fn suelo(l: &Layout, x: f32, d: f32) -> Vec2 {
    let t = d.clamp(0.0, 1.0).powf(1.5);
    let ancho = FONDO + (1.0 - FONDO) * t;
    let px = ARENA_W * 0.5 + (x - ARENA_W * 0.5) * ancho;
    l.to_screen(px, HORIZONTE + (ARENA_H - HORIZONTE) * t)
}

/// Cuanto falta para el proximo tiempo y si el que acaba de sonar era el uno.
///
/// Devuelve `(pulso, fuerte)`: `pulso` vale 1 justo en el golpe y baja hasta 0
/// antes del siguiente, y `fuerte` dice si es el primer tiempo del compas.
fn latido(tema: Tema, t: f32) -> (f32, bool) {
    let (ticks_tiempo, tiempos) = music::compas(tema);
    let tiempo = t / ticks_tiempo;
    let dentro = tiempo.rem_euclid(1.0);
    let cual = (tiempo.rem_euclid(tiempos as f32)).floor() as usize;
    // Ataque instantaneo y caida rapida: un latido que dura se lee como un
    // parpadeo roto, no como un golpe.
    ((1.0 - dentro).powf(2.2), cual == 0)
}

/// El fondo entero.
pub fn dibujar(l: &Layout, tema: Tema, t: f32) {
    let (pulso, fuerte) = latido(tema, t);
    let brillo = pulso * if fuerte { 1.0 } else { 0.45 };

    // La lamina es opaca. La tarima es un trapecio y deja dos triangulos
    // sueltos bajo el horizonte; desde que el fondo es papel, por ahi se colaba
    // la pagina dentro del escenario. Un fondo a toda la arena lo cierra.
    let o = l.to_screen(0.0, 0.0);
    draw_rectangle(o.x, o.y, l.len(ARENA_W), l.len(ARENA_H), PARED);

    pared(l, brillo);
    tarima(l);
    focos(l, t, brillo);
    lampara(l, brillo);
    vineta(l);
}

/// La pared del fondo, con un resplandor en el horizonte que la separa del
/// suelo sin necesidad de dibujar un zocalo.
fn pared(l: &Layout, brillo: f32) {
    let o = l.to_screen(0.0, 0.0);
    let alto = l.len(HORIZONTE);
    draw_rectangle(o.x, o.y, l.len(ARENA_W), alto, PARED);

    // Un degradado a mano: cuatro franjas bastan a esta altura, y evitan tener
    // que montar un shader para el fondo.
    for i in 0..4 {
        let f = i as f32 / 4.0;
        let y = o.y + alto * (0.55 + 0.45 * f);
        let h = alto * 0.45 / 4.0 + 1.0;
        draw_rectangle(
            o.x,
            y,
            l.len(ARENA_W),
            h,
            fade(LUZ, (0.020 + 0.030 * f) * (1.0 + brillo * 0.8)),
        );
    }
}

/// La tarima: un trapecio con vetas que convergen al fondo.
fn tarima(l: &Layout) {
    let (fi, fd) = (suelo(l, 0.0, 0.0), suelo(l, ARENA_W, 0.0));
    let (ci, cd) = (suelo(l, 0.0, 1.0), suelo(l, ARENA_W, 1.0));
    draw_triangle(fi, fd, cd, TARIMA);
    draw_triangle(fi, cd, ci, TARIMA);

    for i in 0..=VETAS {
        let f = i as f32 / VETAS as f32;
        // A lo ancho: se juntan solas hacia el fondo, porque la perspectiva ya
        // esta en `suelo`.
        let (a, b) = (suelo(l, 0.0, f), suelo(l, ARENA_W, f));
        draw_line(a.x, a.y, b.x, b.y, 1.0, fade(VETA, 0.30 + f * 0.45));
        // Y a lo largo, que son las que apuntan al horizonte.
        let x = ARENA_W * f;
        let (a, b) = (suelo(l, x, 0.0), suelo(l, x, 1.0));
        draw_line(a.x, a.y, b.x, b.y, 1.0, fade(VETA, 0.42));
    }
}

/// Los focos del suelo. Laten con el compas.
fn focos(l: &Layout, t: f32, brillo: f32) {
    for i in 0..FOCOS {
        let f = (i as f32 + 0.5) / FOCOS as f32;
        // Se pasean despacio, cada uno a su ritmo, para que la sala no parezca
        // una foto.
        let (vaiven, _) = sin_cos(t * 0.006 + i as f32 * 2.1);
        let x = ARENA_W * (f + vaiven * 0.10);
        let d = 0.34 + 0.30 * (i as f32 - 1.0).abs();
        let c = suelo(l, x, d);
        let (borde, _) = (suelo(l, x + 150.0, d), 0);
        let rx = (borde.x - c.x).abs();
        let ry = rx * 0.34;
        // Tres elipses concentricas hacen un charco de luz con borde suave sin
        // gradiente de verdad.
        for (k, a) in [(1.0, 0.030), (0.66, 0.030), (0.36, 0.035)] {
            draw_ellipse(
                c.x,
                c.y,
                rx * k,
                ry * k,
                0.0,
                fade(LUZ, a * (1.0 + brillo * 1.6)),
            );
        }
    }
}

/// La lampara: anillos concentricos colgando del techo, con lagrimas.
fn lampara(l: &Layout, brillo: f32) {
    // Pequena y tenue a proposito: comparte sitio con el nombre del jefe, y
    // sobre todo no puede competir con el jefe por la atencion. Es un detalle
    // que dice "salon", no un elemento de la escena.
    let c = l.to_screen(ARENA_W * 0.5, HORIZONTE * 0.44);
    let r = l.len(17.0);
    let intensidad = 0.22 + brillo * 0.40;

    // El cable.
    let arriba = l.to_screen(ARENA_W * 0.5, 0.0);
    draw_line(c.x, arriba.y, c.x, c.y, 1.0, fade(ORO, 0.25));

    for (k, grosor, a) in [(1.0, 1.5, 0.55), (0.62, 1.2, 0.75), (0.3, 1.0, 1.0)] {
        draw_poly_lines(c.x, c.y, 12, r * k, 0.0, grosor, fade(ORO, intensidad * a));
    }
    draw_circle(c.x, c.y, r * 0.14, fade(ORO, intensidad));

    // Las lagrimas, que es lo que la hace una lampara y no un aro.
    for i in 0..10 {
        let ang = TAU * i as f32 / 10.0;
        let (s, cos) = sin_cos(ang);
        let p = vec2(c.x + cos * r * 0.88, c.y + s * r * 0.88 * 0.5);
        draw_circle(p.x, p.y + r * 0.22, r * 0.05, fade(ORO, intensidad * 0.9));
    }

    // Y el halo, que es lo que hace que ilumine.
    draw_circle(c.x, c.y, r * 2.6, fade(ORO, 0.014 + brillo * 0.022));
}

/// Oscurece los bordes. Es el truco mas viejo que hay para que la vista se
/// vaya al centro, y en una pantalla llena de balas ayuda de verdad.
fn vineta(l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let (w, h) = (l.len(ARENA_W), l.len(ARENA_H));
    let grosor = l.len(34.0);
    for i in 0..5 {
        let f = (5 - i) as f32 / 5.0;
        let g = grosor * (i + 1) as f32 / 5.0;
        let a = 0.055 * f;
        draw_rectangle(o.x, o.y, w, g, fade(SOMBRA, a));
        draw_rectangle(o.x, o.y + h - g, w, g, fade(SOMBRA, a));
        draw_rectangle(o.x, o.y, g, h, fade(SOMBRA, a));
        draw_rectangle(o.x + w - g, o.y, g, h, fade(SOMBRA, a));
    }
}

fn fade(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_latido_va_al_compas_del_tema() {
        // La sala respira con lo que suena. Si esto se desincroniza, la
        // escenografia pasa de contar algo a distraer.
        for tema in [Tema::PasoBase, Tema::Caminata, Tema::CharlestonA] {
            let (ticks, tiempos) = music::compas(tema);
            // Justo en un tiempo, el pulso esta arriba.
            let (p, _) = latido(tema, 0.0);
            assert!(p > 0.99, "{tema:?}: no late en el tiempo");
            // Y justo antes del siguiente, abajo.
            let (p, _) = latido(tema, ticks * 0.98);
            assert!(p < 0.05, "{tema:?}: no se apaga entre tiempos");
            // El uno del compas es fuerte; los demas no.
            assert!(latido(tema, 0.0).1, "{tema:?}: el uno deberia ser fuerte");
            for i in 1..tiempos {
                assert!(
                    !latido(tema, ticks * i as f32).1,
                    "{tema:?}: el tiempo {i} no es el uno"
                );
            }
            // Y al compas siguiente vuelve a ser fuerte.
            assert!(latido(tema, ticks * tiempos as f32).1);
        }
    }

    #[test]
    fn el_vals_late_de_tres_y_el_tango_de_cuatro() {
        // Es lo que hace que la sala se sienta distinta en cada baile sin
        // dibujar nada distinto.
        assert_eq!(music::compas(Tema::PasoBase).1, 3);
        assert_eq!(music::compas(Tema::Caminata).1, 4);
        assert_eq!(music::compas(Tema::CharlestonA).1, 4);
    }

    #[test]
    fn el_tango_late_cada_treinta_ticks() {
        // 120 negras por minuto a 60 ticks por segundo son 30 ticks por tiempo,
        // que es exactamente la reja en la que caen sus balas. Si esto y el RON
        // dejan de cuadrar, la sala y los disparos van a destiempo.
        let (ticks, _) = music::compas(Tema::Caminata);
        assert!((ticks - 30.0).abs() < 0.01, "{ticks} ticks por tiempo");
    }
}
