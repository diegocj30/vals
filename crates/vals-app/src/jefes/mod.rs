//! Los jefes: cada baile es un objeto vivo de su epoca, y baila.
//!
//! Al principio los cuatro jefes eran la misma figura —una bailarina
//! de huesos con falda— cambiada de color y de tocado, y a la distancia a la
//! que se juega parecian el mismo. En Cuphead casi ningun jefe es
//! una persona: son cosas con cara. Aqui cada baile pasa a ser un objeto de su
//! mundo que cobra vida:
//!
//! - **El Vals**: una caja de musica, con su bailarina girando encima.
//! - **El Tango**: un bandoneon con brazos y cara, que respira con el fuelle.
//! - **El Charleston**: un gramofono; la bocina es la boca y el disco gira.
//! - **El Cancan**: una fila de coristas pateando a la vez.
//!
//! La regla de siempre sigue en pie: **cada jefe baila lo que dispara**.
//! Por eso todos reciben el pulso del compas y la figura en la que van.
//!
//! Un fichero por jefe, y **una sola puerta**: `dibujar`. Lo llaman el combate
//! (`draw_boss`) y el cartel del panel lateral, asi que un jefe nuevo se
//! dibuja en los dos sitios sin tocar nada mas.
//!
//! Hubo aqui una `bailarina_de_siempre` de transicion, para que el juego no se
//! quedara sin jefe mientras cada uno tenia su dibujo. Con los cuatro hechos
//! sobraba, y se borro.
//!
//! **Y cada uno se muere a su manera**: `dibujar_muerte` es la
//! segunda puerta, con `k` de 0 (el golpe que lo tumba) a 1 (tumbado del
//! todo). En Cuphead el KO es el premio, y un objeto que desaparece de golpe
//! no premia nada: la caja de musica se queda sin cuerda, el bandoneon se
//! desinfla, el gramofono se raya y las coristas caen en fila al espagat. El
//! reloj de la caida lo lleva la app (`caida`), no la simulacion: el replay
//! dorado no se entera.

use macroquad::prelude::*;

mod bandoneon;
mod caja_de_musica;
mod coristas;
mod gramofono;

/// Todo lo que un jefe necesita para dibujarse en este instante.
pub struct Escena {
    /// Donde esta su hitbox, en pixeles de pantalla. El jefe se dibuja
    /// alrededor de este punto: el aro de golpeo esta centrado aqui y mide
    /// 46 unidades de radio, asi que el cuerpo tiene que llenarlo mas o menos.
    pub centro: Vec2,
    /// Pixeles por unidad logica. En combate es la escala de la arena por
    /// `bailarines::ESCALA`; en el cartel lateral, lo que quepa.
    pub escala: f32,
    /// Ticks continuos (`tick + alpha`). Todo movimiento sale de aqui: la pose
    /// es funcion pura, sin estado de animacion.
    pub t: f32,
    /// La figura del baile, desde 0. Cuphead cambia al jefe entre fases, y
    /// aqui tambien: cada figura puede verse distinta.
    pub fase: usize,
    /// Vida que le queda en la figura, de 0 a 1.
    pub vida: f32,
    /// El pulso del compas: 1 justo en el golpe y cayendo hasta el siguiente.
    pub pulso: f32,
    /// Si el golpe que acaba de sonar es el uno del compas.
    pub fuerte: bool,
    /// Su tinta principal y la segunda. En combate son las de su baile (ver
    /// `paleta::del_baile`) o blanco si le acaban de dar; en el cartel
    /// lateral la principal es `TINTA`, para que salga en silueta.
    pub tinta: Color,
    pub ropa: Color,
    /// En modo suelo, la y de pantalla de las tablas del escenario. **Nada del
    /// jefe puede quedar por debajo**: si baja hasta ahi, se sube lo justo.
    pub tablas: Option<f32>,
}

/// Dibuja al jefe de un baile.
///
/// Aqui hierve la linea (`trazo::hervir`), para los cuatro a la vez: ninguno
/// tiene que saberlo.
pub fn dibujar(baile: usize, e: &Escena) {
    // El aro de golpeo mide 46 unidades de arena, y la figura lo llena.
    let radio = 46.0 * e.escala / crate::bailarines::ESCALA;
    crate::trazo::hervir(e.centro, radio, 0xB0_55 + baile as u64, || match baile {
        0 => caja_de_musica::dibujar(e),
        1 => bandoneon::dibujar(e),
        2 => gramofono::dibujar(e),
        _ => coristas::dibujar(e),
    });
}

/// Dibuja al jefe de un baile cayendo, con `k` de 0 a 1.
///
/// Como `dibujar`, es funcion pura de la escena y de `k`: la pose de cada
/// instante de la caida sale de curvas (`tramo`, `suave`, `bote`), no de un
/// estado que se vaya acumulando. Lo que haga falta del reloj para temblar
/// sale de `e.t`, que sigue corriendo con el jefe ya en el suelo. Y hierve
/// igual que vivo, con la misma semilla: morirse no cambia el trazo.
pub fn dibujar_muerte(baile: usize, e: &Escena, k: f32) {
    let k = k.clamp(0.0, 1.0);
    let radio = 46.0 * e.escala / crate::bailarines::ESCALA;
    crate::trazo::hervir(e.centro, radio, 0xB0_55 + baile as u64, || match baile {
        0 => caja_de_musica::dibujar_muerte(e, k),
        1 => bandoneon::dibujar_muerte(e, k),
        2 => gramofono::dibujar_muerte(e, k),
        _ => coristas::dibujar_muerte(e, k),
    });
}

/// Lo que dura la caida de un jefe, en segundos, antes del sello de victoria.
///
/// Dos segundos y pico: lo bastante para que se lea que se muere **ese**
/// objeto, y poco para que ganar no se haga largo la vigesima vez. R y ESC
/// siguen funcionando durante la caida, asi que no bloquea nada que antes no
/// bloquease.
pub const CAIDA: f32 = 2.2;

/// Por donde va la caida, de 0 a 1, tras `segundos` desde el golpe.
pub fn caida(segundos: f32) -> f32 {
    (segundos / CAIDA).clamp(0.0, 1.0)
}

/// De 0 a 1 mientras `k` va de `a` a `b`, y quieto fuera de ahi: cada cosa
/// de una caida pasa en su tramo, y asi se encadenan sin estado.
pub(crate) fn tramo(k: f32, a: f32, b: f32) -> f32 {
    ((k - a) / (b - a)).clamp(0.0, 1.0)
}

/// Sale y llega despacio.
pub(crate) fn suave(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// Cae y bota: de 0 a 1 acelerando, y al llegar rebota dos veces cada vez
/// menos. Es como cae una tapa o una bailarina de cuerda: una caida lisa se
/// lee como que se posa, y lo que se muere en un dibujo animado se desploma.
pub(crate) fn bote(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    // El `easeOutBounce` de siempre: una parabola hasta el suelo y dos botes
    // mas pequenos, que suman 1 en x = 1.
    let (n, d) = (7.5625, 2.75);
    if x < 1.0 / d {
        n * x * x
    } else if x < 2.0 / d {
        let x = x - 1.5 / d;
        n * x * x + 0.75
    } else if x < 2.5 / d {
        let x = x - 2.25 / d;
        n * x * x + 0.9375
    } else {
        let x = x - 2.625 / d;
        n * x * x + 0.984375
    }
}

/// Un muelle que se dispara: pasa de 0 a 1, se pasa y oscila hasta quedarse
/// en 1. El "boing" de los muelles que saltan y de lo que se estira de golpe.
pub(crate) fn boing(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    1.0 - (-6.0 * x).exp() * (14.0 * x).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_caida_empieza_en_cero_y_se_queda_en_uno() {
        assert_eq!(caida(0.0), 0.0);
        assert_eq!(caida(-1.0), 0.0);
        assert_eq!(caida(CAIDA), 1.0);
        assert_eq!(caida(CAIDA * 10.0), 1.0);
        // Y nunca va hacia atras: el sello sale una vez y no parpadea.
        let mut antes = 0.0;
        for i in 0..300 {
            let k = caida(i as f32 * 0.01);
            assert!(k >= antes);
            antes = k;
        }
        assert_eq!(antes, 1.0, "tres segundos no bastan para caer");
    }

    #[test]
    fn las_curvas_van_de_cero_a_uno() {
        for f in [suave, bote, boing] {
            assert!(f(0.0).abs() < 1e-3);
            assert!((f(1.0) - 1.0).abs() < 3e-3, "no acaba en 1: {}", f(1.0));
            assert_eq!(f(-5.0), f(0.0));
            assert_eq!(f(5.0), f(1.0));
        }
        assert_eq!(tramo(0.1, 0.2, 0.4), 0.0);
        assert!((tramo(0.3, 0.2, 0.4) - 0.5).abs() < 1e-6);
        assert_eq!(tramo(0.9, 0.2, 0.4), 1.0);
        // El bote llega al suelo antes del final y se levanta un poco.
        assert!((bote(1.0 / 2.75) - 1.0).abs() < 1e-4);
        assert!(bote(0.55) < 0.95);
        // Y el boing se pasa de largo antes de asentarse.
        let pico = (0..100)
            .map(|i| boing(i as f32 / 100.0))
            .fold(0.0, f32::max);
        assert!(pico > 1.2, "el muelle no se pasa: {pico}");
    }
}
