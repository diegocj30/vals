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

use macroquad::prelude::*;

use crate::bailarines;
use crate::draw::{Layout, draw_figura};
use crate::skeleton;

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
    #[expect(
        dead_code,
        reason = "la bailarina de transicion no lo usa; el primer jefe propio que lo lea tiene que quitar esto"
    )]
    pub pulso: f32,
    /// Si el golpe que acaba de sonar es el uno del compas.
    #[expect(
        dead_code,
        reason = "la bailarina de transicion no lo usa; el primer jefe propio que lo lea tiene que quitar esto"
    )]
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
pub fn dibujar(baile: usize, e: &Escena) {
    match baile {
        0 => caja_de_musica::dibujar(e),
        1 => bandoneon::dibujar(e),
        2 => gramofono::dibujar(e),
        _ => coristas::dibujar(e),
    }
}

/// La bailarina de huesos de antes de los jefes objeto.
///
/// Es de transicion: cada jefe la usa hasta que tiene su dibujo propio, asi el
/// juego nunca se queda sin jefe a medio camino. Cuando ninguno la necesite,
/// se borra.
pub(super) fn bailarina_de_siempre(baile: usize, e: &Escena) {
    let ls = Layout::con_escala(e.escala);
    let poses = bailarines::poses(baile, e.fase, e.t, e.vida);
    let mut centro = e.centro;
    if let Some(tablas) = e.tablas {
        let pies = poses
            .iter()
            .flat_map(|p| [p.joints[skeleton::PIE_I].y, p.joints[skeleton::PIE_D].y])
            .fold(f32::MIN, f32::max);
        centro.y -= (e.centro.y + pies * e.escala - tablas).max(0.0);
    }
    for pose in &poses {
        draw_figura(pose, centro, &ls, 1.0, e.tinta, e.ropa);
    }
}
