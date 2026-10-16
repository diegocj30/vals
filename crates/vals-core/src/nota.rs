//! La nota de un baile: la tarjeta de puntuacion de Cuphead, a nuestra manera.
//!
//! Es una funcion pura de cuatro cuentas —cuanto se tardo, cuantos golpes se
//! llevaron, cuantos parries y cuantos supers— y vive en el core aunque no
//! toque la simulacion: se puede probar sin ventana y medir contra un bot.
//!
//! **No cambia nada del mundo.** El `Marcador` solo lee los `Events` de cada
//! tick, que no entran en la huella, asi que el replay dorado ni se entera.

use crate::Dificultad;
use crate::events::Events;

/// Lo que se apunta de un intento, tick a tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Marcador {
    /// Ticks bailados. El `tick` del mundo no vale: reintentar no lo pone a
    /// cero, y la cartela que para el mundo tampoco debe contar.
    pub ticks: u64,
    pub golpes: u32,
    /// Parries **acertados**, no balas parriadas: un parry que se lleva tres
    /// balas de un manotazo es un gesto, no tres.
    pub parries: u32,
    pub supers: u32,
}

impl Marcador {
    /// Apunta un tick. Quien llama lo hace solo mientras se baila: el tick que
    /// gana cuenta, los de la pantalla de victoria no.
    pub fn sumar(&mut self, e: &Events) {
        self.ticks += 1;
        self.golpes += u32::from(e.player_died);
        self.parries += u32::from(e.parried > 0);
        self.supers += u32::from(e.super_fired);
    }
}

/// La nota, de peor a mejor. El orden de las variantes es el de la nota, asi
/// que "mejor que" es `>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Nota {
    D,
    C,
    B,
    A,
    APlus,
}

impl Nota {
    pub const TODAS: [Nota; 5] = [Nota::D, Nota::C, Nota::B, Nota::A, Nota::APlus];

    pub fn letra(self) -> &'static str {
        match self {
            Nota::D => "D",
            Nota::C => "C",
            Nota::B => "B",
            Nota::A => "A",
            Nota::APlus => "A+",
        }
    }

    pub fn por_letra(s: &str) -> Option<Self> {
        Self::TODAS.into_iter().find(|n| n.letra() == s)
    }
}

/// El tiempo de referencia de cada baile, en segundos.
///
/// **Medido, no inventado**: es lo que tarda un bot invulnerable que solo se
/// pone debajo del jefe y dispara sin parar, sin parry ni super (ver el test
/// `el_par_es_lo_que_tarda_el_bot`). Ningun humano lo iguala sin supers,
/// porque hay que esquivar; con ellos se baja de el (el mismo bot parriando y
/// soltando supers tarda entre un 65 y un 95 % de esto). Por eso la nota de
/// tiempo esta entera hasta `par * 1.25` y no hasta `par`.
///
/// El charleston es tan corto porque su jefe apenas se mueve: se le acierta
/// casi todo. Las cifras salen por nombre, como el guardado.
const PAR: [(&str, u64); 4] = [
    ("El Vals", 115),
    ("El Tango", 110),
    ("El Charleston", 55),
    ("El Cancan", 80),
];

/// Para un baile que no esta en la tabla: el del medio.
const PAR_DESCONOCIDO: u64 = 90;

/// El par de un baile, en ticks.
pub fn par(baile: &str) -> u64 {
    let s = PAR
        .iter()
        .find(|(n, _)| *n == baile)
        .map_or(PAR_DESCONOCIDO, |(_, s)| *s);
    s * u64::from(crate::TICK_HZ)
}

/// Puntos de 100, en cuatro partes:
///
/// - **Tiempo, 35.** Enteros hasta `1.25 * par`, cero desde `3 * par`, y en
///   linea entre medias.
/// - **Golpes, 35.** 35, 25, 15, y 5 con tres o mas (con el relicario se
///   puede ganar con tres): ganar siempre da algo.
/// - **Parries, 15.** Cinco por parry, hasta tres. Como Cuphead: el parry es
///   la firma del juego y la nota lo pide, pero tres bastan.
/// - **Supers, 15.** Cinco por super, hasta tres. El roce no puntua aparte:
///   ya cuenta, porque es lo que llena la barra.
pub fn puntos(m: &Marcador, par: u64) -> u32 {
    let par = par.max(1) as f32;
    let t = m.ticks as f32;
    let tiempo = (35.0 * ((3.0 * par - t) / (1.75 * par)).clamp(0.0, 1.0)).round() as u32;
    let golpes = [35, 25, 15, 5][m.golpes.min(3) as usize];
    tiempo + golpes + 5 * m.parries.min(3) + 5 * m.supers.min(3)
}

/// La nota de un intento ganado.
///
/// Los cortes estan puestos para que la primera vez salga una B o una C (dos
/// golpes, el doble del par y un parry suelto: 40, una C justa) y la A+ pida
/// no llevarse ni un golpe, ir rapido y parriar: con un solo golpe y todo lo
/// demas perfecto se queda en 90, una A.
pub fn nota(m: &Marcador, par: u64) -> Nota {
    match puntos(m, par) {
        95.. => Nota::APlus,
        85.. => Nota::A,
        65.. => Nota::B,
        40.. => Nota::C,
        _ => Nota::D,
    }
}

/// Lo mas alto que se saca en la sencilla: una B. Con la mitad de balas y una
/// figura menos, una A no diria lo mismo que en la normal.
pub const TOPE_SENCILLO: Nota = Nota::B;

/// La nota de un intento ganado en su dificultad.
pub fn nota_en(m: &Marcador, par: u64, d: Dificultad) -> Nota {
    let n = nota(m, par);
    match d {
        Dificultad::Normal => n,
        Dificultad::Sencillo => n.min(TOPE_SENCILLO),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::BossDef;
    use crate::{InputFrame, Mode, World};

    fn m(seg: f32, golpes: u32, parries: u32, supers: u32) -> Marcador {
        Marcador {
            ticks: (seg * 60.0) as u64,
            golpes,
            parries,
            supers,
        }
    }

    const PAR: u64 = 100 * 60;

    #[test]
    fn los_cortes_de_la_nota() {
        // Perfecto.
        assert_eq!(nota(&m(100.0, 0, 3, 3), PAR), Nota::APlus);
        // Sin golpes, rapido, tres parries y dos supers: 95 justo.
        assert_eq!(puntos(&m(125.0, 0, 3, 2), PAR), 95);
        assert_eq!(nota(&m(125.0, 0, 3, 2), PAR), Nota::APlus);
        // Un golpe y todo lo demas perfecto no llega.
        assert_eq!(nota(&m(100.0, 1, 3, 3), PAR), Nota::A);
        // La primera vez de alguien: dos golpes, el doble del par, un parry.
        assert_eq!(puntos(&m(200.0, 2, 1, 0), PAR), 40);
        assert_eq!(nota(&m(200.0, 2, 1, 0), PAR), Nota::C);
        assert_eq!(nota(&m(200.0, 2, 0, 0), PAR), Nota::D);
        // Un golpe, algo lento, dos parries y un super: una B.
        assert_eq!(nota(&m(160.0, 1, 2, 1), PAR), Nota::B);
    }

    #[test]
    fn en_sencillo_la_nota_no_pasa_de_b() {
        let perfecto = m(100.0, 0, 3, 3);
        assert_eq!(nota_en(&perfecto, PAR, Dificultad::Normal), Nota::APlus);
        assert_eq!(nota_en(&perfecto, PAR, Dificultad::Sencillo), Nota::B);
        // Lo que ya estaba por debajo se queda como estaba.
        let flojo = m(200.0, 2, 0, 0);
        assert_eq!(nota_en(&flojo, PAR, Dificultad::Sencillo), Nota::D);
    }

    #[test]
    fn el_tiempo_satura_por_los_dos_lados() {
        assert_eq!(puntos(&m(1.0, 3, 0, 0), PAR), 35 + 5);
        assert_eq!(puntos(&m(125.0, 3, 0, 0), PAR), 35 + 5);
        assert_eq!(puntos(&m(300.0, 3, 0, 0), PAR), 5);
        assert_eq!(puntos(&m(9999.0, 9, 9, 9), PAR), 5 + 15 + 15);
    }

    #[test]
    fn la_nota_se_ordena_y_se_lee_por_letra() {
        assert!(Nota::APlus > Nota::A && Nota::C > Nota::D);
        for n in Nota::TODAS {
            assert_eq!(Nota::por_letra(n.letra()), Some(n));
        }
        assert_eq!(Nota::por_letra("Z"), None);
    }

    #[test]
    fn el_marcador_cuenta_gestos_y_no_balas() {
        let mut k = Marcador::default();
        k.sumar(&Events {
            parried: 3,
            super_fired: true,
            player_died: true,
            ..Default::default()
        });
        k.sumar(&Events::default());
        assert_eq!(k, m(2.0 / 60.0, 1, 1, 1));
    }

    #[test]
    fn el_par_es_lo_que_tarda_el_bot() {
        // El bot de la tabla: invulnerable, debajo del jefe y disparando. Si
        // alguien reafina un baile y el par se queda viejo, esto lo dice.
        for (i, def) in BossDef::default_bosses().iter().enumerate() {
            let mut w = World::empezar_en(1, Mode::Flight, i);
            let mut t = 0u64;
            while !w.victory && t < 60 * 300 {
                w.player.iframes = 5;
                let mut f = InputFrame::NONE;
                let dx = w.boss.pos.x - w.player.pos.x;
                f.set(InputFrame::SHOOT, true);
                f.set(InputFrame::LEFT, dx < -6.0);
                f.set(InputFrame::RIGHT, dx > 6.0);
                w.step(f);
                t += 1;
            }
            let p = par(&def.name);
            assert_ne!(p, PAR_DESCONOCIDO * 60, "{} no tiene par", def.name);
            let r = t as f32 / p as f32;
            assert!(
                (0.85..1.15).contains(&r),
                "{}: el bot tarda {:.1} s y el par dice {} s",
                def.name,
                t as f32 / 60.0,
                p / 60
            );
        }
    }
}
