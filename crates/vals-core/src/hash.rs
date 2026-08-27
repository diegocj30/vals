//! FNV-1a de 64 bits, para las huellas de estado.
//!
//! No se usa `DefaultHasher` porque `std` no garantiza que su algoritmo sea
//! estable entre versiones de Rust, y estos hashes tienen que seguir valiendo
//! dentro de un ano: son lo que permitira comparar un replay grabado hoy con
//! la simulacion de manana.

use glam::Vec2;

pub(crate) struct Fnv1a(u64);

impl Fnv1a {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub(crate) fn new() -> Self {
        Self(Self::OFFSET)
    }

    pub(crate) fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    /// Se hashean los bits, no el valor: `to_bits` es exacto y distingue
    /// `-0.0` de `0.0`, que es justo lo que queremos al comparar estados.
    pub(crate) fn write_f32(&mut self, v: f32) {
        self.write_u64(u64::from(v.to_bits()));
    }

    pub(crate) fn write_vec2(&mut self, v: Vec2) {
        self.write_f32(v.x);
        self.write_f32(v.y);
    }

    pub(crate) fn finish(self) -> u64 {
        self.0
    }
}
