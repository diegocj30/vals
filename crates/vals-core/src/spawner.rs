//! Patron **provisional**.
//!
//! Existe solo para que H2 sea jugable y medible: sin nada disparando no hay
//! forma de saber si el sistema de balas funciona ni cuanto cuesta. H3 lo
//! sustituye entero por el interprete de patrones cargado desde RON.
//!
//! Aun asi el patron esta pensado, no puesto al azar: una espiral constante
//! que llena el espacio y hay que leer, mas una rafaga apuntada cada segundo y
//! medio que impide quedarse quieto en un hueco.

use glam::Vec2;

use crate::bullets::{Bullets, KIND_MEDIUM, KIND_NEEDLE};
use crate::emitter::{EmitterSpec, fire};
use crate::{ARENA_W, hash::Fnv1a};

/// De donde salen las balas mientras no haya un jefe de verdad.
pub const BOSS_POS: Vec2 = Vec2::new(ARENA_W * 0.5, 130.0);

/// Ticks entre disparos de la espiral.
const SPIRAL_EVERY: u64 = 6;
/// Cuanto avanza el angulo de la espiral en cada disparo.
const SPIRAL_STEP: f32 = 0.37;
/// Ticks entre rafagas apuntadas.
const AIMED_EVERY: u64 = 90;

#[derive(Clone, Debug, Default)]
pub struct Spawner {
    angle: f32,
}

impl Spawner {
    pub fn new() -> Self {
        Self { angle: 0.0 }
    }

    pub fn update(&mut self, tick: u64, bullets: &mut Bullets, player: Vec2) {
        // Espiral: un anillo pequeno cuyo angulo base avanza cada vez. Es la
        // forma mas barata de generar algo que se lee como un patron y no como
        // ruido.
        if tick.is_multiple_of(SPIRAL_EVERY) {
            let spec = EmitterSpec::ring(5, 150.0)
                .at_angle(self.angle)
                .with_kind(KIND_NEEDLE);
            fire(bullets, BOSS_POS, &spec, player);
            self.angle += SPIRAL_STEP;
        }

        // Rafaga apuntada: obliga a moverse. Un patron sin nada apuntado se
        // esquiva quedandose quieto en un hueco.
        if tick.is_multiple_of(AIMED_EVERY) {
            let spec = EmitterSpec::fan(5, 0.6, 190.0)
                .aimed()
                .with_kind(KIND_MEDIUM);
            fire(bullets, BOSS_POS, &spec, player);
        }
    }

    pub(crate) fn hash_into(&self, h: &mut Fnv1a) {
        h.write_f32(self.angle);
    }
}
