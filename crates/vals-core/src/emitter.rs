//! Emisores: las formas basicas con las que se construye un patron.
//!
//! Con anillo, abanico, apuntado y capas concentricas se cubre el 90% del
//! danmaku clasico. El 10% restante sale de **encadenar** estas formas en el
//! tiempo, que es exactamente lo que hara el interprete de patrones de H3.
//!
//! La espiral no es una forma aparte: es un anillo o un abanico cuyo angulo
//! base avanza un poco en cada disparo. Por eso `Aim::Fixed` acepta el angulo
//! y lo gestiona quien dispara.

use std::f32::consts::TAU;

use glam::Vec2;

use crate::bullets::{Bullets, KIND_SMALL, Spawn};

/// Hacia donde se orienta el emisor.
#[derive(Clone, Copy, Debug)]
pub enum Aim {
    /// Angulo absoluto en radianes.
    Fixed(f32),
    /// Al jugador en el momento del disparo.
    ///
    /// Esto es lo que hace que un patron obligue a moverse: un patron sin
    /// nada apuntado se esquiva quedandose quieto en un hueco.
    AtPlayer,
}

/// Descripcion de una tanda de balas.
#[derive(Clone, Copy, Debug)]
pub struct EmitterSpec {
    /// Balas por capa.
    pub count: u32,
    /// Radianes que abarca el abanico. `TAU` o mas es un anillo completo.
    pub arc: f32,
    pub aim: Aim,
    pub speed: f32,
    /// Capas concentricas, cada una con `speed_step` mas de velocidad.
    pub rings: u32,
    pub speed_step: f32,
    /// Desfase angular entre capas. Con un valor pequeno las capas quedan
    /// escalonadas en vez de alineadas, que se lee mucho mejor.
    pub ring_offset: f32,
    pub accel: f32,
    pub spin: f32,
    pub ttl: f32,
    pub kind: u8,
    pub flags: u8,
}

impl Default for EmitterSpec {
    fn default() -> Self {
        Self {
            count: 8,
            arc: TAU,
            aim: Aim::Fixed(0.0),
            speed: 120.0,
            rings: 1,
            speed_step: 40.0,
            ring_offset: 0.0,
            accel: 0.0,
            spin: 0.0,
            ttl: 30.0,
            kind: KIND_SMALL,
            flags: 0,
        }
    }
}

impl EmitterSpec {
    /// Anillo completo de `count` balas.
    pub fn ring(count: u32, speed: f32) -> Self {
        Self {
            count,
            arc: TAU,
            speed,
            ..Default::default()
        }
    }

    /// Abanico de `count` balas repartidas en `arc`, centrado en la mira.
    pub fn fan(count: u32, arc: f32, speed: f32) -> Self {
        Self {
            count,
            arc,
            speed,
            ..Default::default()
        }
    }

    pub fn aimed(mut self) -> Self {
        self.aim = Aim::AtPlayer;
        self
    }

    pub fn at_angle(mut self, radians: f32) -> Self {
        self.aim = Aim::Fixed(radians);
        self
    }

    pub fn with_kind(mut self, kind: u8) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_spin(mut self, spin: f32) -> Self {
        self.spin = spin;
        self
    }

    pub fn with_accel(mut self, accel: f32) -> Self {
        self.accel = accel;
        self
    }

    pub fn with_rings(mut self, rings: u32, speed_step: f32, ring_offset: f32) -> Self {
        self.rings = rings;
        self.speed_step = speed_step;
        self.ring_offset = ring_offset;
        self
    }
}

/// Dispara una tanda. Devuelve cuantas balas salieron de verdad.
///
/// Puede salir menos de lo pedido si el pool esta lleno; el emisor no lo trata
/// como un error, simplemente se queda corto.
pub fn fire(bullets: &mut Bullets, origin: Vec2, spec: &EmitterSpec, player: Vec2) -> u32 {
    if spec.count == 0 {
        return 0;
    }

    let base = match spec.aim {
        Aim::Fixed(a) => a,
        Aim::AtPlayer => {
            let d = player - origin;
            // Si el jugador esta justo encima, no hay direccion definida:
            // se dispara hacia abajo en vez de propagar un NaN.
            if d.length_squared() < 1e-6 {
                std::f32::consts::FRAC_PI_2
            } else {
                d.to_angle()
            }
        }
    };

    let anillo_completo = spec.arc >= TAU - 1e-4;
    // En un anillo completo el paso es arc/count, porque la primera y la
    // ultima bala coincidirian. En un abanico es arc/(count-1), para que los
    // extremos caigan justo en los bordes del arco.
    let (inicio, paso) = if anillo_completo {
        (base, TAU / spec.count as f32)
    } else if spec.count == 1 {
        (base, 0.0)
    } else {
        (base - spec.arc * 0.5, spec.arc / (spec.count - 1) as f32)
    };

    let mut salidas = 0;
    for capa in 0..spec.rings.max(1) {
        let speed = spec.speed + spec.speed_step * capa as f32;
        let offset = spec.ring_offset * capa as f32;
        for i in 0..spec.count {
            let angulo = inicio + paso * i as f32 + offset;
            let vel = Vec2::from_angle(angulo) * speed;
            if bullets
                .spawn(Spawn {
                    pos: origin,
                    vel,
                    accel: spec.accel,
                    spin: spec.spin,
                    ttl: spec.ttl,
                    kind: spec.kind,
                    flags: spec.flags,
                })
                .is_some()
            {
                salidas += 1;
            } else {
                // Pool lleno: no tiene sentido seguir intentandolo.
                return salidas;
            }
        }
    }
    salidas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bullets::Bullets;

    fn origen() -> Vec2 {
        Vec2::new(320.0, 120.0)
    }

    #[test]
    fn el_anillo_reparte_las_balas_por_igual() {
        let mut b = Bullets::with_capacity(64);
        let n = fire(&mut b, origen(), &EmitterSpec::ring(8, 100.0), Vec2::ZERO);
        assert_eq!(n, 8);

        let mut angulos: Vec<f32> = b.iter_live().map(|v| v.vel.to_angle()).collect();
        angulos.sort_by(f32::total_cmp);
        for par in angulos.windows(2) {
            let d = par[1] - par[0];
            assert!((d - TAU / 8.0).abs() < 0.001, "separacion irregular: {d}");
        }
    }

    #[test]
    fn todas_las_balas_del_anillo_van_a_la_misma_velocidad() {
        let mut b = Bullets::with_capacity(64);
        fire(&mut b, origen(), &EmitterSpec::ring(12, 150.0), Vec2::ZERO);
        for v in b.iter_live() {
            assert!((v.vel.length() - 150.0).abs() < 0.01);
        }
    }

    #[test]
    fn el_abanico_queda_centrado_en_la_mira() {
        let mut b = Bullets::with_capacity(64);
        let mira = 1.0;
        let arco = 0.8;
        let spec = EmitterSpec::fan(5, arco, 100.0).at_angle(mira);
        fire(&mut b, origen(), &spec, Vec2::ZERO);

        let mut angulos: Vec<f32> = b.iter_live().map(|v| v.vel.to_angle()).collect();
        angulos.sort_by(f32::total_cmp);
        assert_eq!(angulos.len(), 5);
        assert!((angulos[0] - (mira - arco * 0.5)).abs() < 0.001);
        assert!((angulos[4] - (mira + arco * 0.5)).abs() < 0.001);
        assert!(
            (angulos[2] - mira).abs() < 0.001,
            "la del medio va a la mira"
        );
    }

    #[test]
    fn una_sola_bala_va_exactamente_a_la_mira() {
        let mut b = Bullets::with_capacity(8);
        let spec = EmitterSpec::fan(1, 0.8, 100.0).at_angle(0.4);
        fire(&mut b, origen(), &spec, Vec2::ZERO);
        let v = b.iter_live().next().unwrap();
        assert!((v.vel.to_angle() - 0.4).abs() < 0.001);
    }

    #[test]
    fn el_emisor_apuntado_va_hacia_el_jugador() {
        let mut b = Bullets::with_capacity(8);
        let jugador = Vec2::new(500.0, 600.0);
        let spec = EmitterSpec::fan(1, 0.0, 100.0).aimed();
        fire(&mut b, origen(), &spec, jugador);

        let v = b.iter_live().next().unwrap();
        let esperado = (jugador - origen()).normalize();
        assert!(
            (v.vel.normalize() - esperado).length() < 0.001,
            "deberia apuntar al jugador"
        );
    }

    #[test]
    fn apuntar_al_propio_origen_no_produce_nan() {
        let mut b = Bullets::with_capacity(8);
        let spec = EmitterSpec::fan(3, 0.5, 100.0).aimed();
        fire(&mut b, origen(), &spec, origen());
        for v in b.iter_live() {
            assert!(v.vel.is_finite(), "velocidad no finita: {}", v.vel);
            assert!(v.vel.length() > 1.0);
        }
    }

    #[test]
    fn las_capas_salen_a_velocidades_distintas() {
        let mut b = Bullets::with_capacity(64);
        let spec = EmitterSpec::ring(4, 100.0).with_rings(3, 50.0, 0.1);
        let n = fire(&mut b, origen(), &spec, Vec2::ZERO);
        assert_eq!(n, 12);

        let mut velocidades: Vec<f32> = b.iter_live().map(|v| v.vel.length()).collect();
        velocidades.sort_by(f32::total_cmp);
        assert!((velocidades[0] - 100.0).abs() < 0.01);
        assert!((velocidades[11] - 200.0).abs() < 0.01);
    }

    #[test]
    fn si_el_pool_se_llena_se_dispara_lo_que_quepa() {
        let mut b = Bullets::with_capacity(5);
        let n = fire(&mut b, origen(), &EmitterSpec::ring(20, 100.0), Vec2::ZERO);
        assert_eq!(n, 5, "deberia haber disparado solo lo que cabia");
        assert_eq!(b.live_count(), 5);
    }

    #[test]
    fn count_cero_no_dispara_nada() {
        let mut b = Bullets::with_capacity(8);
        let spec = EmitterSpec {
            count: 0,
            ..Default::default()
        };
        assert_eq!(fire(&mut b, origen(), &spec, Vec2::ZERO), 0);
    }

    #[test]
    fn una_espiral_es_un_anillo_con_el_angulo_avanzando() {
        let mut b = Bullets::with_capacity(256);
        let mut angulo = 0.0f32;
        for _ in 0..8 {
            let spec = EmitterSpec::ring(3, 100.0).at_angle(angulo);
            fire(&mut b, origen(), &spec, Vec2::ZERO);
            angulo += 0.2;
        }
        assert_eq!(b.live_count(), 24);
        // Con el angulo avanzando, no hay dos tandas identicas.
        let angulos: Vec<f32> = b.iter_live().map(|v| v.vel.to_angle()).collect();
        let distintos = angulos
            .iter()
            .map(|a| (a * 1000.0) as i32)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(distintos.len() > 20, "deberian estar escalonadas");
    }
}
