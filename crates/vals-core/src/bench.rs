//! Escena de stress reproducible.
//!
//! Es la regla de medir del proyecto. Existe para que la medicion de hoy y la
//! de dentro de un ano sean **la misma medicion**: semilla fija, secuencia de
//! disparos fija y ningun input humano de por medio.
//!
//! No usa el mundo entero a proposito: aisla el bucle caliente (mover balas y
//! comprobar colisiones), que es lo unico que se va a optimizar. Meter aqui el
//! jugador, el render o la logica de patrones solo anadiria ruido.
//!
//! Se usa desde dos sitios: `cargo bench -p vals-core` (sin GPU de por medio) y
//! `cargo run -p vals-app --release -- --bench-scene` (con render).

use glam::Vec2;

use crate::bullets::Bullets;
use crate::emitter::{EmitterSpec, fire};
use crate::{ARENA_H, ARENA_W, MAX_BULLETS, Pcg32};

/// Semilla fija. No se toca nunca: cambiarla invalida todo el historico de
/// `docs/PERF.md`.
pub const STRESS_SEED: u64 = 0x5354_5245_5353;

/// Balas por anillo en cada recarga.
const RING_SIZE: u32 = 16;

pub struct Stress {
    pub bullets: Bullets,
    target: usize,
    angle: f32,
    rng: Pcg32,
}

impl Stress {
    /// Escena que se mantiene en torno a `target` balas vivas.
    pub fn new(target: usize) -> Self {
        Self {
            bullets: Bullets::with_capacity(MAX_BULLETS.max(target + RING_SIZE as usize)),
            target: target.min(MAX_BULLETS),
            angle: 0.0,
            rng: Pcg32::new(STRESS_SEED),
        }
    }

    pub fn target(&self) -> usize {
        self.target
    }

    pub fn live_count(&self) -> usize {
        self.bullets.live_count()
    }

    /// Rellena hasta el objetivo y avanza un paso.
    ///
    /// El relleno va antes del update para que el numero de balas vivas sea
    /// estable de un paso a otro: si no, cada medicion cogeria una poblacion
    /// distinta y las cifras no serian comparables.
    pub fn step(&mut self, dt: f32) {
        while self.bullets.live_count() < self.target {
            let origin = Vec2::new(
                self.rng.range_f32(40.0, ARENA_W - 40.0),
                self.rng.range_f32(40.0, ARENA_H - 40.0),
            );
            let spec = EmitterSpec::ring(RING_SIZE, self.rng.range_f32(40.0, 140.0))
                .at_angle(self.angle)
                .with_spin(self.rng.range_f32(-0.4, 0.4));
            self.angle += 0.13;
            if fire(&mut self.bullets, origin, &spec, Vec2::ZERO) == 0 {
                break; // pool lleno
            }
        }
        self.bullets.update(dt);
    }

    /// Comprobacion de colision en el peor caso: un punto donde no hay nada,
    /// que obliga a recorrer el pool entero sin poder salir antes.
    pub fn worst_case_hit_test(&self) -> bool {
        self.bullets
            .hit_circle(Vec2::new(-1000.0, -1000.0), 1.0)
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DT;

    #[test]
    fn la_escena_alcanza_y_mantiene_el_objetivo() {
        let mut s = Stress::new(2000);
        for _ in 0..10 {
            s.step(DT);
        }
        let vivas = s.live_count();
        assert!(
            vivas >= 2000 && vivas < 2000 + RING_SIZE as usize,
            "deberia rondar el objetivo, hay {vivas}"
        );
    }

    #[test]
    fn la_escena_es_reproducible() {
        let paso = |n: usize| {
            let mut s = Stress::new(500);
            for _ in 0..n {
                s.step(DT);
            }
            s.bullets
                .iter_live()
                .map(|v| v.pos.x.to_bits() as u64 ^ ((v.pos.y.to_bits() as u64) << 32))
                .fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b))
        };
        assert_eq!(paso(60), paso(60), "misma escena, mismo resultado");
    }

    #[test]
    fn el_peor_caso_de_colision_no_encuentra_nada() {
        let mut s = Stress::new(1000);
        s.step(DT);
        assert!(!s.worst_case_hit_test());
    }
}
