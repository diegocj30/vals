//! La sacudida de pantalla y el hitstop.
//!
//! Las dos cosas que hacen que un golpe **se sienta** y que no cuestan ni un
//! sprite. Un impacto sin sacudida y sin freno se lee como un cambio de numero;
//! con las dos, se lee como un impacto.
//!
//! Y las dos viven aqui, en `vals-app`, disparadas desde el `Events` del core.
//! El hitstop en particular podria parecer que toca la simulacion, y no la
//! toca: **congela el reloj de pared, no los ticks**. El mundo avanza los
//! mismos ticks con los mismos inputs, solo que unos frames mas tarde, asi que
//! el replay dorado no se entera.

use macroquad::prelude::*;
use vals_core::{Events, Pcg32};

/// Cuanto se queda de la sacudida en cada tick. Baja rapido: una sacudida que
/// dura se lee como que la ventana esta rota, no como un golpe.
const AMORTIGUACION: f32 = 0.84;
/// Por debajo de esto se corta del todo, para no dejar la pantalla temblando
/// medio pixel eternamente.
const MINIMO: f32 = 0.15;

/// Amplitud de la sacudida, en unidades de arena, para cada suceso.
const SACUDIDA_IMPACTO: f32 = 1.4;
const SACUDIDA_PARRY: f32 = 4.0;
const SACUDIDA_FASE: f32 = 9.0;
const SACUDIDA_SUPER: f32 = 12.0;
const SACUDIDA_MUERTE: f32 = 14.0;
const SACUDIDA_JEFE: f32 = 16.0;

/// Frames congelados para cada suceso.
const FRENO_PARRY: u32 = 3;
const FRENO_FASE: u32 = 5;
const FRENO_SUPER: u32 = 6;
const FRENO_MUERTE: u32 = 9;
const FRENO_JEFE: u32 = 10;

pub struct Zumo {
    /// Lo que queda de sacudida, en unidades de arena.
    amplitud: f32,
    /// Frames que quedan por congelar.
    congelacion: u32,
    rng: Pcg32,
}

impl Zumo {
    pub fn new() -> Self {
        Self {
            amplitud: 0.0,
            congelacion: 0,
            rng: Pcg32::new(0x5641_4C53_5A55_4D4F),
        }
    }

    /// Traduce lo que ha pasado a cuanto se nota.
    ///
    /// **Se queda el mayor, no se suman.** Sumar hace que en un frame con seis
    /// impactos la pantalla salga volando, y son justo los frames en los que
    /// mas falta hace poder ver.
    pub fn reaccionar(&mut self, ev: &Events) {
        let mut sacudir = |v: f32| self.amplitud = self.amplitud.max(v);
        if ev.boss_hit {
            sacudir(SACUDIDA_IMPACTO);
        }
        if ev.parried > 0 {
            sacudir(SACUDIDA_PARRY);
        }
        if ev.phase_changed {
            sacudir(SACUDIDA_FASE);
        }
        if ev.super_fired {
            sacudir(SACUDIDA_SUPER);
        }
        if ev.player_died {
            sacudir(SACUDIDA_MUERTE);
        }
        if ev.boss_down {
            sacudir(SACUDIDA_JEFE);
        }

        let mut frenar = |f: u32| self.congelacion = self.congelacion.max(f);
        if ev.parried > 0 {
            frenar(FRENO_PARRY);
        }
        if ev.phase_changed {
            frenar(FRENO_FASE);
        }
        if ev.super_fired {
            frenar(FRENO_SUPER);
        }
        if ev.player_died {
            frenar(FRENO_MUERTE);
        }
        if ev.boss_down {
            frenar(FRENO_JEFE);
        }
    }

    /// Un tick. La sacudida se apaga con el mismo paso fijo que todo lo demas,
    /// asi que dura lo mismo a 60 que a 144 Hz.
    pub fn step(&mut self) {
        self.amplitud *= AMORTIGUACION;
        if self.amplitud < MINIMO {
            self.amplitud = 0.0;
        }
    }

    /// Si este frame esta congelado. Consume uno.
    pub fn congelado(&mut self) -> bool {
        if self.congelacion > 0 {
            self.congelacion -= 1;
            true
        } else {
            false
        }
    }

    /// Cuanto se desplaza la pantalla este frame, en unidades de arena.
    pub fn desplazamiento(&mut self) -> Vec2 {
        if self.amplitud <= 0.0 {
            return Vec2::ZERO;
        }
        let a = self.amplitud;
        vec2(
            (self.rng.next_f32() * 2.0 - 1.0) * a,
            (self.rng.next_f32() * 2.0 - 1.0) * a,
        )
    }

    /// Para el mundo entero de golpe. Lo usa la trampa de saltarse un baile,
    /// que no pasa por `Events`.
    pub fn golpe_gordo(&mut self) {
        self.amplitud = self.amplitud.max(SACUDIDA_JEFE);
        self.congelacion = self.congelacion.max(FRENO_JEFE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn con(f: impl FnOnce(&mut Events)) -> Events {
        let mut ev = Events::default();
        f(&mut ev);
        ev
    }

    #[test]
    fn la_sacudida_se_apaga_sola() {
        let mut z = Zumo::new();
        z.reaccionar(&con(|e| e.boss_down = true));
        assert!(z.desplazamiento() != Vec2::ZERO);
        for _ in 0..60 {
            z.step();
        }
        assert_eq!(z.desplazamiento(), Vec2::ZERO, "se ha quedado temblando");
    }

    #[test]
    fn los_sucesos_no_se_suman_se_queda_el_mayor() {
        // Seis impactos en un frame no pueden mandar la pantalla a tomar
        // viento: son justo los frames en los que mas falta hace ver.
        let mut solo = Zumo::new();
        solo.reaccionar(&con(|e| e.boss_down = true));

        let mut todos = Zumo::new();
        todos.reaccionar(&con(|e| {
            e.boss_hit = true;
            e.parried = 1;
            e.phase_changed = true;
            e.player_died = true;
            e.boss_down = true;
        }));
        assert_eq!(solo.amplitud, todos.amplitud);
        assert_eq!(solo.congelacion, todos.congelacion);
    }

    #[test]
    fn el_hitstop_dura_lo_que_dice_y_se_acaba() {
        let mut z = Zumo::new();
        z.reaccionar(&con(|e| e.parried = 1));
        for i in 0..FRENO_PARRY {
            assert!(z.congelado(), "el frame {i} deberia estar congelado");
        }
        assert!(!z.congelado(), "y aqui ya no");
    }

    #[test]
    fn un_impacto_normal_no_congela() {
        // Disparar al jefe pasa sesenta veces por segundo. Si cada impacto
        // congelase, el juego iria a trompicones todo el rato.
        let mut z = Zumo::new();
        z.reaccionar(&con(|e| {
            e.boss_hit = true;
            e.player_shot = true;
            e.grazed = 1;
        }));
        assert!(!z.congelado());
        assert!(z.amplitud > 0.0, "pero si sacude un poco");
    }
}
