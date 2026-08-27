//! Estado del mundo y avance de la simulacion.

use glam::Vec2;

use crate::{ARENA_H, ARENA_W, InputFrame, Pcg32, Player};

/// Todo el estado simulado.
#[derive(Clone, Debug)]
pub struct World {
    /// Ticks simulados desde el inicio. Es el reloj del juego: dentro de
    /// `vals-core` no existe el tiempo real.
    pub tick: u64,
    pub rng: Pcg32,
    pub player: Player,
    /// Input del tick anterior. Hace falta para detectar flancos (el dash
    /// reacciona a la pulsacion, no a mantener la tecla) y vive en el mundo
    /// para que un replay lo reproduzca sin depender de nada externo.
    prev_input: InputFrame,
    seed: u64,
}

impl World {
    /// Mundo nuevo a partir de una semilla.
    pub fn new(seed: u64) -> Self {
        Self {
            tick: 0,
            rng: Pcg32::new(seed),
            player: Player::new(Vec2::new(ARENA_W * 0.5, ARENA_H * 0.78)),
            prev_input: InputFrame::NONE,
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
        self.player.update(input, self.prev_input);
        self.prev_input = input;
        self.tick += 1;
    }

    /// Huella del estado completo, para tests de determinismo.
    ///
    /// Es la red de seguridad del proyecto: cuando se reescriba el bucle
    /// caliente con SIMD o con hilos, esto es lo que demuestra que el
    /// comportamiento no cambio ni un frame.
    ///
    /// La estela queda fuera a proposito: es informacion derivada de las
    /// posiciones y puramente cosmetica, asi que hashearla no anadiria ninguna
    /// garantia y solo haria el hash mas caro.
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv1a::new();
        h.write_u64(self.tick);
        h.write_u64(self.seed);
        let (s, i) = self.rng.raw_state();
        h.write_u64(s);
        h.write_u64(i);
        h.write_u64(u64::from(self.prev_input.bits()));

        let p = &self.player;
        h.write_vec2(p.pos);
        h.write_vec2(p.prev_pos);
        h.write_vec2(p.vel);
        h.write_vec2(p.facing);
        h.write_f32(p.focus_t);
        h.write_u64(u64::from(p.focused));
        h.write_u64(u64::from(p.dash.ticks_left));
        h.write_u64(u64::from(p.dash.iframes));
        h.write_u64(u64::from(p.dash.cooldown));
        h.write_u64(u64::from(p.dash.buffer));
        h.write_vec2(p.dash.dir);

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

    /// Se hashean los bits, no el valor: `to_bits` es exacto y distingue
    /// `-0.0` de `0.0`, que es justo lo que queremos al comparar estados.
    fn write_f32(&mut self, v: f32) {
        self.write_u64(u64::from(v.to_bits()));
    }

    fn write_vec2(&mut self, v: Vec2) {
        self.write_f32(v.x);
        self.write_f32(v.y);
    }

    fn finish(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::*;

    const DASH: InputFrame = InputFrame::from_bits(InputFrame::DASH);
    const NADA: InputFrame = InputFrame::NONE;

    fn pulsar(bits: u8) -> InputFrame {
        InputFrame::from_bits(bits)
    }

    /// Avanza `n` ticks con el mismo input.
    fn correr(w: &mut World, input: InputFrame, n: usize) {
        for _ in 0..n {
            w.step(input);
        }
    }

    /// Genera una secuencia de inputs plausible y reproducible.
    fn inputs_de_prueba(seed: u64, n: usize) -> Vec<InputFrame> {
        let mut r = Pcg32::new(seed);
        (0..n)
            .map(|_| InputFrame::from_bits((r.next_u32() & 0xFF) as u8))
            .collect()
    }

    // --- Determinismo ---

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
        a.step(pulsar(InputFrame::LEFT));
        b.step(pulsar(InputFrame::RIGHT));
        assert_ne!(a.state_hash(), b.state_hash());
    }

    // --- Movimiento ---

    #[test]
    fn la_diagonal_no_es_mas_rapida_que_la_recta() {
        let mut recto = World::new(0);
        let mut diagonal = World::new(0);
        correr(&mut recto, pulsar(InputFrame::RIGHT), 30);
        correr(
            &mut diagonal,
            pulsar(InputFrame::RIGHT | InputFrame::UP),
            30,
        );

        let v_recto = recto.player.vel.length();
        let v_diag = diagonal.player.vel.length();
        assert!(
            (v_recto - v_diag).abs() < 0.01,
            "recta {v_recto} vs diagonal {v_diag}"
        );
        assert!((v_recto - PLAYER_SPEED).abs() < 0.01);
    }

    #[test]
    fn alcanza_la_velocidad_plena_en_los_ticks_previstos() {
        let mut w = World::new(0);
        correr(
            &mut w,
            pulsar(InputFrame::RIGHT),
            PLAYER_ACCEL_TICKS as usize,
        );
        assert!(
            (w.player.vel.length() - PLAYER_SPEED).abs() < 0.01,
            "tras {PLAYER_ACCEL_TICKS} ticks deberia ir a tope, va a {}",
            w.player.vel.length()
        );
    }

    #[test]
    fn se_para_del_todo_al_soltar() {
        let mut w = World::new(0);
        correr(&mut w, pulsar(InputFrame::RIGHT), 30);
        correr(&mut w, NADA, PLAYER_DECEL_TICKS as usize);
        assert_eq!(w.player.vel, Vec2::ZERO);
    }

    #[test]
    fn el_focus_frena_al_jugador() {
        let mut normal = World::new(0);
        let mut focus = World::new(0);
        correr(&mut normal, pulsar(InputFrame::UP), 30);
        correr(&mut focus, pulsar(InputFrame::UP | InputFrame::FOCUS), 30);
        assert!(
            normal.player.vel.length() > focus.player.vel.length() * 2.0,
            "el focus deberia ser bastante mas lento"
        );
    }

    #[test]
    fn el_jugador_no_se_sale_de_la_arena() {
        let mut w = World::new(0);
        correr(&mut w, pulsar(InputFrame::LEFT | InputFrame::UP), 600);
        assert!(w.player.pos.x >= PLAYER_SPRITE_RADIUS - 0.001);
        assert!(w.player.pos.y >= PLAYER_SPRITE_RADIUS - 0.001);

        correr(&mut w, pulsar(InputFrame::RIGHT | InputFrame::DOWN), 600);
        assert!(w.player.pos.x <= ARENA_W - PLAYER_SPRITE_RADIUS + 0.001);
        assert!(w.player.pos.y <= ARENA_H - PLAYER_SPRITE_RADIUS + 0.001);
    }

    #[test]
    fn chocar_con_el_borde_mata_la_velocidad_de_ese_eje() {
        let mut w = World::new(0);
        // Contra la pared izquierda el tiempo suficiente para llegar y seguir.
        correr(&mut w, pulsar(InputFrame::LEFT), 300);
        assert_eq!(
            w.player.vel.x, 0.0,
            "no deberia acumular velocidad contra la pared"
        );
        // Y al soltar no debe salir disparado.
        let x_antes = w.player.pos.x;
        correr(&mut w, NADA, 5);
        assert!((w.player.pos.x - x_antes).abs() < 0.001);
    }

    // --- Dash ---

    #[test]
    fn el_dash_dura_los_ticks_previstos() {
        let mut w = World::new(0);
        w.step(DASH);
        let mut ticks = 0;
        while w.player.is_dashing() {
            ticks += 1;
            w.step(NADA);
        }
        assert_eq!(ticks, DASH_TICKS);
    }

    #[test]
    fn los_iframes_duran_exactamente_lo_declarado() {
        let mut w = World::new(0);
        w.step(DASH);
        let mut ticks = 0;
        while w.player.is_invulnerable() {
            ticks += 1;
            w.step(NADA);
        }
        assert_eq!(ticks, DASH_IFRAME_TICKS);
    }

    #[test]
    fn los_iframes_sobreviven_al_final_del_dash() {
        // La invariante DASH_IFRAME_TICKS > DASH_TICKS se comprueba al
        // compilar en player.rs. Aqui se comprueba el efecto observable.
        let mut w = World::new(0);
        w.step(DASH);
        correr(&mut w, NADA, DASH_TICKS as usize);
        assert!(!w.player.is_dashing(), "el dash ya deberia haber acabado");
        assert!(
            w.player.is_invulnerable(),
            "pero deberia seguir siendo invulnerable"
        );
    }

    #[test]
    fn mantener_la_tecla_de_dash_no_encadena_dashes() {
        let mut w = World::new(0);
        // Mantener pulsado mucho mas que el enfriamiento.
        correr(&mut w, DASH, (DASH_COOLDOWN_TICKS * 3) as usize);
        assert!(
            !w.player.is_dashing(),
            "el dash reacciona al flanco, no a mantener la tecla"
        );
    }

    #[test]
    fn no_se_puede_hacer_dash_durante_el_enfriamiento() {
        let mut w = World::new(0);
        w.step(DASH);
        // Soltar y volver a pulsar demasiado pronto: fuera de la ventana del
        // buffer, para aislar el efecto del enfriamiento.
        correr(&mut w, NADA, INPUT_BUFFER_TICKS as usize + 2);
        let cooldown_antes = w.player.dash.cooldown;
        assert!(cooldown_antes > 0, "aun deberia estar enfriando");

        w.step(DASH);
        assert!(
            !w.player.is_dashing(),
            "no deberia haber arrancado otro dash"
        );
    }

    #[test]
    fn el_buffer_de_input_recupera_una_pulsacion_temprana() {
        let mut w = World::new(0);
        w.step(DASH);
        // Esperar hasta justo dentro de la ventana del buffer antes de que
        // acabe el enfriamiento.
        let espera = DASH_COOLDOWN_TICKS - INPUT_BUFFER_TICKS + 1;
        correr(&mut w, NADA, espera as usize);
        assert!(w.player.dash.cooldown > 0, "todavia enfriando");

        // Pulsar "demasiado pronto": deberia guardarse y salir sola.
        w.step(DASH);
        assert!(!w.player.is_dashing(), "aun no puede salir");

        correr(&mut w, NADA, INPUT_BUFFER_TICKS as usize);
        assert!(
            w.player.is_dashing(),
            "el buffer deberia haber disparado el dash al acabar el enfriamiento"
        );
    }

    #[test]
    fn el_dash_sin_direccion_usa_la_ultima_encarada() {
        let mut w = World::new(0);
        correr(&mut w, pulsar(InputFrame::LEFT), 10);
        let encarada = w.player.facing;
        correr(&mut w, NADA, 5);

        w.step(DASH);
        assert!(
            (w.player.dash.dir - encarada).length() < 0.001,
            "deberia haber dasheado hacia {encarada}, fue hacia {}",
            w.player.dash.dir
        );
    }

    #[test]
    fn el_dash_no_saca_al_jugador_de_la_arena() {
        let mut w = World::new(0);
        // Pegarse a la pared izquierda y dashear contra ella.
        correr(&mut w, pulsar(InputFrame::LEFT), 200);
        w.step(pulsar(InputFrame::LEFT | InputFrame::DASH));
        correr(&mut w, pulsar(InputFrame::LEFT), DASH_TICKS as usize);
        assert!(w.player.pos.x >= PLAYER_SPRITE_RADIUS - 0.001);
    }

    #[test]
    fn el_dash_es_mas_rapido_que_correr() {
        let mut w = World::new(0);
        w.step(DASH);
        assert!(w.player.vel.length() > PLAYER_SPEED * 2.0);
    }

    // --- Estela ---

    #[test]
    fn la_estela_se_llena_y_no_crece_sin_limite() {
        let mut w = World::new(0);
        assert!(w.player.trail.is_empty());
        correr(&mut w, pulsar(InputFrame::UP), 1000);
        assert_eq!(w.player.trail.len(), TRAIL_LEN);
        assert_eq!(w.player.trail.iter_newest_first().count(), TRAIL_LEN);
    }

    #[test]
    fn la_estela_va_del_punto_mas_reciente_al_mas_antiguo() {
        let mut w = World::new(0);
        correr(&mut w, pulsar(InputFrame::UP), 60);
        let puntos: Vec<_> = w.player.trail.iter_newest_first().collect();
        // Subiendo, la y decrece con el tiempo: el mas reciente es el de y menor.
        assert!(puntos[0].0.pos.y < puntos[puntos.len() - 1].0.pos.y);
        assert!((puntos[0].0.pos - w.player.pos).length() < 0.001);
    }
}
