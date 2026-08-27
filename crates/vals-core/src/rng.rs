//! PRNG propio (PCG32).
//!
//! Son treinta lineas y nos evita depender de `rand`, cuya API se mueve entre
//! versiones mayores. Mas importante: al ser nuestro, sabemos con certeza que
//! no hay nada no determinista dentro, que es de lo que depende todo el
//! sistema de replays.
//!
//! Referencia: <https://www.pcg-random.org/>

const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// Generador PCG32, sembrado explicitamente.
#[derive(Clone, Debug)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    /// Crea un generador a partir de una semilla.
    ///
    /// La misma semilla produce siempre exactamente la misma secuencia.
    pub fn new(seed: u64) -> Self {
        let mut r = Self {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    /// Siguiente entero de 32 bits.
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Flotante en `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        // 24 bits de mantisa: suficiente para f32 y sin sesgo de redondeo.
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    /// Flotante en `[min, max)`.
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    /// Estado interno, para el hash de determinismo.
    pub(crate) fn raw_state(&self) -> (u64, u64) {
        (self.state, self.inc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_misma_semilla_da_la_misma_secuencia() {
        let mut a = Pcg32::new(1234);
        let mut b = Pcg32::new(1234);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn semillas_distintas_divergen() {
        let mut a = Pcg32::new(1);
        let mut b = Pcg32::new(2);
        let distintos = (0..100).filter(|_| a.next_u32() != b.next_u32()).count();
        assert!(distintos > 90, "las secuencias deberian divergir enseguida");
    }

    #[test]
    fn next_f32_se_queda_dentro_del_rango() {
        let mut r = Pcg32::new(99);
        for _ in 0..10_000 {
            let v = r.next_f32();
            assert!((0.0..1.0).contains(&v), "fuera de rango: {v}");
        }
    }
}
