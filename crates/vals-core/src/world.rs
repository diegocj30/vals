//! Estado del mundo y avance de la simulacion.

use glam::Vec2;

use crate::{ARENA_H, ARENA_W, DT, InputFrame, Pcg32};

/// Velocidad normal del jugador, en unidades logicas por segundo.
pub const PLAYER_SPEED: f32 = 300.0;

/// Velocidad en modo focus.
pub const PLAYER_FOCUS_SPEED: f32 = 130.0;

/// Hitbox real: diminuta, como manda el genero. Esquivar consiste en saber
/// que casi todo tu personaje es decorativo.
pub const PLAYER_HITBOX_RADIUS: f32 = 2.5;

/// Radio de lo que se dibuja. No colisiona con nada.
pub const PLAYER_SPRITE_RADIUS: f32 = 11.0;

/// El jugador.
#[derive(Clone, Debug)]
pub struct Player {
    /// Posicion al final del ultimo tick simulado.
    pub pos: Vec2,
    /// Posicion al final del tick anterior. Solo existe para que el render
    /// pueda interpolar; la simulacion no la lee.
    pub prev_pos: Vec2,
    pub focused: bool,
}

impl Player {
    /// Posicion a dibujar, interpolada entre el tick anterior y el actual.
    ///
    /// `alpha` es la fraccion de tick ya consumida por el acumulador del bucle
    /// principal. Esto es lo que hace que a 144 Hz se vea suave aunque la
    /// simulacion vaya a 60.
    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev_pos.lerp(self.pos, alpha)
    }
}

/// Todo el estado simulado.
#[derive(Clone, Debug)]
pub struct World {
    /// Ticks simulados desde el inicio. Es el reloj del juego: dentro de
    /// `vals-core` no existe el tiempo real.
    pub tick: u64,
    pub rng: Pcg32,
    pub player: Player,
    seed: u64,
}

impl World {
    /// Mundo nuevo a partir de una semilla.
    pub fn new(seed: u64) -> Self {
        let start = Vec2::new(ARENA_W * 0.5, ARENA_H * 0.78);
        Self {
            tick: 0,
            rng: Pcg32::new(seed),
            player: Player {
                pos: start,
                prev_pos: start,
                focused: false,
            },
            seed,
        }
    }

    /// Semilla con la que se creo. Necesaria para reproducir un replay.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Avanza la simulacion exactamente un tick.
    ///
    /// Es la unica puerta de entrada al mundo. No lee reloj, ni ficheros, ni
    /// nada del entorno: mismo estado + mismo input = mismo resultado, siempre.
    pub fn step(&mut self, input: InputFrame) {
        self.player.prev_pos = self.player.pos;
        self.player.focused = input.is_down(InputFrame::FOCUS);

        let (ax, ay) = input.axis();
        // Normalizar es lo que impide que moverse en diagonal sea un 41% mas
        // rapido, que es el bug de movimiento mas viejo del mundo.
        let dir = Vec2::new(ax, ay).normalize_or_zero();
        let speed = if self.player.focused {
            PLAYER_FOCUS_SPEED
        } else {
            PLAYER_SPEED
        };

        self.player.pos += dir * speed * DT;
        self.clamp_player_to_arena();

        self.tick += 1;
    }

    fn clamp_player_to_arena(&mut self) {
        let m = PLAYER_SPRITE_RADIUS;
        self.player.pos.x = self.player.pos.x.clamp(m, ARENA_W - m);
        self.player.pos.y = self.player.pos.y.clamp(m, ARENA_H - m);
    }

    /// Huella del estado completo, para tests de determinismo.
    ///
    /// Este metodo es la red de seguridad del proyecto: cuando en H∞ se
    /// reescriba el bucle caliente con SIMD o con hilos, esto es lo que
    /// demuestra que el comportamiento no cambio ni un frame.
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv1a::new();
        h.write_u64(self.tick);
        h.write_u64(self.seed);
        let (s, i) = self.rng.raw_state();
        h.write_u64(s);
        h.write_u64(i);
        h.write_f32(self.player.pos.x);
        h.write_f32(self.player.pos.y);
        h.write_f32(self.player.prev_pos.x);
        h.write_f32(self.player.prev_pos.y);
        h.write_u64(u64::from(self.player.focused));
        h.finish()
    }
}

/// FNV-1a de 64 bits.
///
/// No usamos `DefaultHasher` porque `std` no garantiza que su algoritmo sea
/// estable entre versiones de Rust, y aqui el hash tiene que seguir valiendo
/// dentro de un ano.
struct Fnv1a(u64);

impl Fnv1a {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    /// Se hashean los bits, no el valor: `to_bits` es exacto y `-0.0` y `0.0`
    /// se distinguen, que es justo lo que queremos al comparar estados.
    fn write_f32(&mut self, v: f32) {
        self.write_u64(u64::from(v.to_bits()));
    }

    fn finish(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Genera una secuencia de inputs plausible y reproducible.
    fn inputs_de_prueba(seed: u64, n: usize) -> Vec<InputFrame> {
        let mut r = Pcg32::new(seed);
        (0..n)
            .map(|_| InputFrame::from_bits((r.next_u32() & 0xFF) as u8))
            .collect()
    }

    #[test]
    fn mismo_seed_y_mismos_inputs_dan_el_mismo_estado() {
        let inputs = inputs_de_prueba(7, 3600); // un minuto de juego
        let mut a = World::new(0x5641_4C53);
        let mut b = World::new(0x5641_4C53);

        for (i, input) in inputs.iter().copied().enumerate() {
            a.step(input);
            b.step(input);
            assert_eq!(a.state_hash(), b.state_hash(), "divergencia en el tick {i}");
        }
    }

    #[test]
    fn inputs_distintos_producen_estados_distintos() {
        let mut a = World::new(1);
        let mut b = World::new(1);
        a.step(InputFrame::from_bits(InputFrame::LEFT));
        b.step(InputFrame::from_bits(InputFrame::RIGHT));
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn la_diagonal_no_es_mas_rapida_que_la_recta() {
        let mut recto = World::new(0);
        let mut diagonal = World::new(0);

        for _ in 0..60 {
            recto.step(InputFrame::from_bits(InputFrame::RIGHT));
            diagonal.step(InputFrame::from_bits(InputFrame::RIGHT | InputFrame::UP));
        }

        let d_recto = recto.player.pos.distance(World::new(0).player.pos);
        let d_diag = diagonal.player.pos.distance(World::new(0).player.pos);
        assert!(
            (d_recto - d_diag).abs() < 0.01,
            "recta {d_recto} vs diagonal {d_diag}"
        );
    }

    #[test]
    fn el_focus_frena_al_jugador() {
        let mut normal = World::new(0);
        let mut focus = World::new(0);
        for _ in 0..30 {
            normal.step(InputFrame::from_bits(InputFrame::UP));
            focus.step(InputFrame::from_bits(InputFrame::UP | InputFrame::FOCUS));
        }
        let inicio = World::new(0).player.pos;
        assert!(
            normal.player.pos.distance(inicio) > focus.player.pos.distance(inicio) * 2.0,
            "el modo focus deberia ser bastante mas lento"
        );
    }

    #[test]
    fn el_jugador_no_se_sale_de_la_arena() {
        let mut w = World::new(0);
        // Empujar 10 segundos contra la esquina superior izquierda.
        for _ in 0..600 {
            w.step(InputFrame::from_bits(InputFrame::LEFT | InputFrame::UP));
        }
        assert!(w.player.pos.x >= PLAYER_SPRITE_RADIUS - 0.001);
        assert!(w.player.pos.y >= PLAYER_SPRITE_RADIUS - 0.001);

        for _ in 0..600 {
            w.step(InputFrame::from_bits(InputFrame::RIGHT | InputFrame::DOWN));
        }
        assert!(w.player.pos.x <= ARENA_W - PLAYER_SPRITE_RADIUS + 0.001);
        assert!(w.player.pos.y <= ARENA_H - PLAYER_SPRITE_RADIUS + 0.001);
    }
}
