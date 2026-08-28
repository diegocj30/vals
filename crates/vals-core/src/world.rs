//! Estado del mundo y avance de la simulacion.

use glam::Vec2;

use crate::boss::{Boss, BossDef};
use crate::bullets::{Bullets, KIND_NEEDLE, Spawn};
use crate::hash::Fnv1a;
use crate::player::{
    GRAZE_METER, GRAZE_RADIUS, PARRY_RADIUS, PLAYER_HITBOX_RADIUS, SHOT_DAMAGE, SHOT_EVERY,
    SHOT_SPEED, SHOT_SPREAD, SUPER_DAMAGE,
};
use crate::{ARENA_H, ARENA_W, DT, InputFrame, Pcg32, Player};

/// Vidas con las que empieza una partida.
///
/// Tres. Es lo que convierte el boss-rush en una carrera con tension: sin un
/// limite, morir solo cuesta tiempo y el juego deja de tener apuesta.
pub const STARTING_LIVES: u32 = 3;

/// Capacidad del pool de disparos del jugador.
///
/// Muy por encima de lo que cabe en pantalla: a 15 disparos por segundo con un
/// ttl de 2 segundos, nunca pasan de un centenar.
const PLAYER_SHOT_CAPACITY: usize = 512;

/// Todo el estado simulado.
#[derive(Clone, Debug)]
pub struct World {
    /// Ticks simulados desde el inicio. Es el reloj del juego: dentro de
    /// `vals-core` no existe el tiempo real.
    pub tick: u64,
    pub rng: Pcg32,
    pub player: Player,
    /// Balas del jefe. Las que te matan.
    pub bullets: Bullets,
    /// Disparos del jugador. Pool aparte: distinto tamano, distinto objetivo, y
    /// asi la colision de cada bando no tiene que filtrar por equipo.
    pub player_shots: Bullets,
    pub boss: Boss,
    /// Cual de los jefes se esta jugando.
    pub boss_index: usize,
    /// Vidas que quedan.
    pub lives: u32,
    /// Se pone a `true` al caer el ultimo jefe.
    pub victory: bool,
    /// Se pone a `true` al quedarse sin vidas.
    pub defeat: bool,
    /// Los jefes de la partida, en orden.
    boss_defs: Vec<BossDef>,
    /// Input del tick anterior. Hace falta para detectar flancos (el dash
    /// reacciona a la pulsacion, no a mantener la tecla) y vive en el mundo
    /// para que un replay lo reproduzca sin depender de nada externo.
    prev_input: InputFrame,
    /// Si el jefe actua. Ver [`World::sandbox`].
    boss_enabled: bool,
    seed: u64,
}

impl World {
    /// Mundo nuevo a partir de una semilla.
    pub fn new(seed: u64) -> Self {
        let defs = BossDef::default_bosses();
        Self {
            tick: 0,
            rng: Pcg32::new(seed),
            player: Player::new(Self::spawn_pos()),
            bullets: Bullets::default(),
            player_shots: Bullets::with_capacity(PLAYER_SHOT_CAPACITY),
            boss: Boss::from_def(&defs[0]),
            boss_index: 0,
            lives: STARTING_LIVES,
            victory: false,
            defeat: false,
            boss_defs: defs,
            prev_input: InputFrame::NONE,
            boss_enabled: true,
            seed,
        }
    }

    /// Mundo sin nada disparando.
    ///
    /// Sirve para los tests de movimiento —que si no acabarian peleandose con
    /// el jefe en vez de probando lo suyo— y es la base del modo entrenamiento
    /// que tarde o temprano querra existir.
    pub fn sandbox(seed: u64) -> Self {
        Self {
            boss_enabled: false,
            ..Self::new(seed)
        }
    }

    /// Reemplaza los jefes en caliente, conservando el resto de la partida.
    ///
    /// Es lo que usa el hot-reload: al guardar un RON se reconstruye el jefe
    /// actual desde cero y se limpia la pantalla, pero no pierdes la partida ni
    /// las vidas.
    pub fn reload_bosses(&mut self, defs: Vec<BossDef>) {
        if defs.is_empty() {
            return;
        }
        self.boss_index = self.boss_index.min(defs.len() - 1);
        self.boss = Boss::from_def(&defs[self.boss_index]);
        self.boss_defs = defs;
        self.bullets.clear();
        self.victory = false;
    }

    /// Numero de jefes de la partida.
    pub fn boss_count(&self) -> usize {
        self.boss_defs.len()
    }

    /// Si la partida ha terminado, por las buenas o por las malas.
    pub fn is_over(&self) -> bool {
        self.victory || self.defeat
    }

    /// Pasa al jefe siguiente, o declara la victoria si era el ultimo.
    fn advance_boss(&mut self) {
        if self.boss_index + 1 < self.boss_defs.len() {
            self.boss_index += 1;
            self.boss = Boss::from_def(&self.boss_defs[self.boss_index]);
            // Los disparos en vuelo tambien se limpian: heredarlos restaria
            // vida al jefe nuevo antes de que aparezca.
            self.player_shots.clear();
        } else {
            self.victory = true;
        }
    }

    /// Semilla con la que se creo. Necesaria para reproducir un replay.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Donde aparece y reaparece el jugador.
    pub fn spawn_pos() -> Vec2 {
        Vec2::new(ARENA_W * 0.5, ARENA_H * 0.78)
    }

    /// Avanza la simulacion exactamente un tick.
    ///
    /// Es la unica puerta de entrada al mundo. No lee reloj, ni ficheros, ni
    /// nada del entorno: mismo estado + mismo input = mismo resultado, siempre.
    pub fn step(&mut self, input: InputFrame) {
        self.player.update(input, self.prev_input);
        self.player_shoot(input);

        if self.boss_enabled && !self.is_over() {
            self.boss.update(&mut self.bullets, self.player.pos);
        }
        self.bullets.update(DT);
        self.player_shots.update(DT);

        self.resolve_parry();
        self.resolve_graze();
        self.resolve_super(input);
        self.resolve_player_shots();
        self.resolve_player_hit();

        self.prev_input = input;
        self.tick += 1;
    }

    fn player_shoot(&mut self, input: InputFrame) {
        if !input.is_down(InputFrame::SHOOT) || self.player.shot_cooldown > 0 {
            return;
        }
        self.player.shot_cooldown = SHOT_EVERY;
        // Dos chorros paralelos. Uno solo se siente escuchimizado, y dos muy
        // separados obligarian a apuntar, que no es de lo que va este juego.
        for dx in [-SHOT_SPREAD, SHOT_SPREAD] {
            self.player_shots.spawn(Spawn {
                pos: self.player.pos + Vec2::new(dx, -10.0),
                vel: Vec2::new(0.0, -SHOT_SPEED),
                ttl: 2.0,
                kind: KIND_NEEDLE,
                ..Default::default()
            });
        }
    }

    fn resolve_parry(&mut self) {
        if !self.player.is_parrying() {
            return;
        }
        let n = self.bullets.parry_circle(self.player.pos, PARRY_RADIUS);
        if n > 0 {
            self.player.on_parry(n);
        }
    }

    fn resolve_graze(&mut self) {
        // Va antes de la comprobacion de muerte: si una bala te roza y te mata
        // en el mismo tick, al menos te has llevado el roce.
        let n = self.bullets.graze_circle(self.player.pos, GRAZE_RADIUS);
        if n > 0 {
            self.player.grazes += n;
            self.player.add_meter(n as f32 * GRAZE_METER);
        }
    }

    fn resolve_super(&mut self, input: InputFrame) {
        let pulsado =
            input.is_down(InputFrame::SUPER) && !self.prev_input.is_down(InputFrame::SUPER);
        if !pulsado || !self.player.spend_meter() {
            return;
        }
        // Limpia la pantalla y pega fuerte. Es la descarga de todo lo que has
        // arriesgado acercandote, asi que tiene que notarse.
        self.bullets.clear();
        if !self.boss.defeated && self.boss.damage(SUPER_DAMAGE) {
            self.bullets.clear();
            if self.boss.defeated {
                self.victory = true;
            }
        }
    }

    fn resolve_player_shots(&mut self) {
        if self.boss.defeated || self.is_over() {
            return;
        }
        let impactos = self
            .player_shots
            .damage_circle(self.boss.pos, self.boss.radius);
        if impactos == 0 {
            return;
        }
        if self.boss.damage(impactos as i32 * SHOT_DAMAGE) {
            // Cambio de fase o caida: en ambos casos se limpia la pantalla.
            // Heredar la pared de balas de la fase anterior seria una muerte
            // imposible de evitar justo en el momento de celebrar.
            self.bullets.clear();
            if self.boss.defeated {
                self.advance_boss();
            }
        }
    }

    fn resolve_player_hit(&mut self) {
        // La colision va al final, contra las posiciones ya actualizadas de
        // ambos. Comprobarla antes de mover dejaria pasar balas rapidas por
        // encima del jugador dentro del mismo tick.
        if self.player.is_invulnerable() || self.is_over() {
            return;
        }
        if self
            .bullets
            .hit_circle(self.player.pos, PLAYER_HITBOX_RADIUS)
            .is_some()
        {
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.defeat = true;
            }
            self.player.die(Self::spawn_pos());
            // Limpiar la pantalla al morir es lo canonico del genero: sin esto
            // reaparecerias dentro de la misma pared de balas que acaba de
            // matarte.
            self.bullets.clear();
        }
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
        h.write_u64(u64::from(p.dash.cooldown));
        h.write_u64(u64::from(p.dash.buffer));
        h.write_vec2(p.dash.dir);
        h.write_u64(u64::from(p.iframes));
        h.write_u64(u64::from(p.deaths));
        h.write_u64(u64::from(p.parry_window));
        h.write_u64(u64::from(p.parry_cooldown));
        h.write_u64(u64::from(p.super_ticks));
        h.write_f32(p.meter);
        h.write_u64(u64::from(p.parries));
        h.write_u64(u64::from(p.grazes));

        h.write_u64(u64::from(self.boss_enabled));
        h.write_u64(u64::from(self.victory));
        h.write_u64(u64::from(self.defeat));
        h.write_u64(self.boss_index as u64);
        h.write_u64(u64::from(self.lives));
        self.boss.hash_into(&mut h);
        self.bullets.hash_into(&mut h);
        self.player_shots.hash_into(&mut h);

        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::*;

    const DASH: InputFrame = InputFrame::from_bits(InputFrame::DASH);
    const NADA: InputFrame = InputFrame::NONE;

    /// Mundo de pruebas de movimiento: sin nada disparando.
    fn mundo(seed: u64) -> World {
        World::sandbox(seed)
    }

    fn pulsar(bits: u16) -> InputFrame {
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
            .map(|_| InputFrame::from_bits((r.next_u32() & 0x1FF) as u16))
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
        let mut recto = mundo(0);
        let mut diagonal = mundo(0);
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
        let mut w = mundo(0);
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
        let mut w = mundo(0);
        correr(&mut w, pulsar(InputFrame::RIGHT), 30);
        correr(&mut w, NADA, PLAYER_DECEL_TICKS as usize);
        assert_eq!(w.player.vel, Vec2::ZERO);
    }

    #[test]
    fn el_focus_frena_al_jugador() {
        let mut normal = mundo(0);
        let mut focus = mundo(0);
        correr(&mut normal, pulsar(InputFrame::UP), 30);
        correr(&mut focus, pulsar(InputFrame::UP | InputFrame::FOCUS), 30);
        assert!(
            normal.player.vel.length() > focus.player.vel.length() * 2.0,
            "el focus deberia ser bastante mas lento"
        );
    }

    #[test]
    fn el_jugador_no_se_sale_de_la_arena() {
        let mut w = mundo(0);
        correr(&mut w, pulsar(InputFrame::LEFT | InputFrame::UP), 600);
        assert!(w.player.pos.x >= PLAYER_SPRITE_RADIUS - 0.001);
        assert!(w.player.pos.y >= PLAYER_SPRITE_RADIUS - 0.001);

        correr(&mut w, pulsar(InputFrame::RIGHT | InputFrame::DOWN), 600);
        assert!(w.player.pos.x <= ARENA_W - PLAYER_SPRITE_RADIUS + 0.001);
        assert!(w.player.pos.y <= ARENA_H - PLAYER_SPRITE_RADIUS + 0.001);
    }

    #[test]
    fn chocar_con_el_borde_mata_la_velocidad_de_ese_eje() {
        let mut w = mundo(0);
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
        let mut w = mundo(0);
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
        let mut w = mundo(0);
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
        let mut w = mundo(0);
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
        let mut w = mundo(0);
        // Mantener pulsado mucho mas que el enfriamiento.
        correr(&mut w, DASH, (DASH_COOLDOWN_TICKS * 3) as usize);
        assert!(
            !w.player.is_dashing(),
            "el dash reacciona al flanco, no a mantener la tecla"
        );
    }

    #[test]
    fn no_se_puede_hacer_dash_durante_el_enfriamiento() {
        let mut w = mundo(0);
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
        let mut w = mundo(0);
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
        let mut w = mundo(0);
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
        let mut w = mundo(0);
        // Pegarse a la pared izquierda y dashear contra ella.
        correr(&mut w, pulsar(InputFrame::LEFT), 200);
        w.step(pulsar(InputFrame::LEFT | InputFrame::DASH));
        correr(&mut w, pulsar(InputFrame::LEFT), DASH_TICKS as usize);
        assert!(w.player.pos.x >= PLAYER_SPRITE_RADIUS - 0.001);
    }

    #[test]
    fn el_dash_es_mas_rapido_que_correr() {
        let mut w = mundo(0);
        w.step(DASH);
        assert!(w.player.vel.length() > PLAYER_SPEED * 2.0);
    }

    // --- Estela ---

    #[test]
    fn la_estela_se_llena_y_no_crece_sin_limite() {
        let mut w = mundo(0);
        assert!(w.player.trail.is_empty());
        correr(&mut w, pulsar(InputFrame::UP), 1000);
        assert_eq!(w.player.trail.len(), TRAIL_LEN);
        assert_eq!(w.player.trail.iter_newest_first().count(), TRAIL_LEN);
    }

    #[test]
    fn la_estela_va_del_punto_mas_reciente_al_mas_antiguo() {
        let mut w = mundo(0);
        correr(&mut w, pulsar(InputFrame::UP), 60);
        let puntos: Vec<_> = w.player.trail.iter_newest_first().collect();
        // Subiendo, la y decrece con el tiempo: el mas reciente es el de y menor.
        assert!(puntos[0].0.pos.y < puntos[puntos.len() - 1].0.pos.y);
        assert!((puntos[0].0.pos - w.player.pos).length() < 0.001);
    }
    // --- Balas, muerte y respawn ---

    /// Pone una bala en el camino del jugador, sin depender del patron.
    ///
    /// Se anticipa la posicion del siguiente tick, no la actual: durante un
    /// dash el jugador recorre 15 unidades por tick y se saldria de una bala
    /// puesta donde esta ahora antes de que se compruebe la colision.
    fn bala_encima(w: &mut World) {
        let pos = w.player.pos + w.player.vel * DT;
        w.bullets
            .spawn(crate::bullets::Spawn {
                pos,
                ..Default::default()
            })
            .unwrap();
    }

    #[test]
    fn el_patron_provisional_dispara() {
        let mut w = World::new(0);
        correr(&mut w, NADA, 30);
        assert!(
            w.bullets.live_count() > 0,
            "el jefe deberia estar disparando"
        );
    }

    #[test]
    fn una_bala_encima_mata_al_jugador() {
        let mut w = mundo(0);
        // Alejarse del punto de reaparicion para notar que vuelve.
        correr(&mut w, pulsar(InputFrame::LEFT), 40);
        assert!(!w.player.is_invulnerable());
        let muertes = w.player.deaths;

        bala_encima(&mut w);
        w.step(NADA);

        assert_eq!(w.player.deaths, muertes + 1);
        assert_eq!(
            w.player.pos,
            World::spawn_pos(),
            "deberia reaparecer en el inicio"
        );
    }

    #[test]
    fn morir_limpia_la_pantalla() {
        let mut w = mundo(0);
        for i in 0..20 {
            w.bullets
                .spawn(crate::bullets::Spawn {
                    pos: Vec2::new(50.0 + i as f32, 50.0),
                    ..Default::default()
                })
                .unwrap();
        }
        bala_encima(&mut w);
        w.step(NADA);
        assert_eq!(
            w.bullets.live_count(),
            0,
            "reaparecer dentro de la misma pared de balas seria injusto"
        );
    }

    #[test]
    fn al_reaparecer_hay_invulnerabilidad_larga() {
        let mut w = mundo(0);
        bala_encima(&mut w);
        w.step(NADA);
        assert!(w.player.is_invulnerable());
        assert!(
            w.player.iframes > DASH_IFRAME_TICKS,
            "el respawn tiene que dar mucho mas margen que un dash"
        );
    }

    #[test]
    fn los_iframes_del_dash_salvan_de_una_bala() {
        let mut w = mundo(0);
        w.step(DASH);
        assert!(w.player.is_invulnerable());
        let muertes = w.player.deaths;

        bala_encima(&mut w);
        w.step(NADA);
        assert_eq!(
            w.player.deaths, muertes,
            "el dash deberia haberla atravesado"
        );
    }

    #[test]
    fn al_expirar_la_invulnerabilidad_vuelve_a_morir() {
        let mut w = mundo(0);
        bala_encima(&mut w);
        w.step(NADA);
        let muertes = w.player.deaths;

        // Esperar a que se acabe el margen del respawn.
        correr(&mut w, NADA, RESPAWN_IFRAME_TICKS as usize + 1);
        assert!(!w.player.is_invulnerable());

        bala_encima(&mut w);
        w.step(NADA);
        assert_eq!(w.player.deaths, muertes + 1);
    }

    #[test]
    fn morir_cancela_el_dash_en_curso() {
        let mut w = mundo(0);
        w.step(pulsar(InputFrame::LEFT | InputFrame::DASH));
        assert!(w.player.is_dashing());

        // Forzar la muerte pese a los i-frames del dash.
        w.player.iframes = 0;
        bala_encima(&mut w);
        w.step(NADA);

        assert!(
            !w.player.is_dashing(),
            "salir disparado al reaparecer desorienta"
        );
        assert_eq!(w.player.vel, Vec2::ZERO);
    }

    #[test]
    fn el_mundo_sandbox_no_dispara() {
        let mut w = mundo(0);
        correr(&mut w, NADA, 300);
        assert_eq!(w.bullets.live_count(), 0);
        assert_eq!(w.player.deaths, 0);
    }
    // --- Jefe y disparos del jugador ---

    const DISPARAR: InputFrame = InputFrame::from_bits(InputFrame::SHOOT);

    #[test]
    fn el_jugador_dispara_a_la_cadencia_pedida() {
        let mut w = mundo(0);
        // 4 ticks entre disparos y 2 balas por disparo.
        correr(&mut w, DISPARAR, SHOT_EVERY as usize * 3);
        assert_eq!(w.player_shots.live_count(), 6);
    }

    #[test]
    fn sin_pulsar_el_boton_no_sale_nada() {
        let mut w = mundo(0);
        correr(&mut w, NADA, 60);
        assert_eq!(w.player_shots.live_count(), 0);
    }

    #[test]
    fn los_disparos_mueren_al_salir_por_arriba() {
        let mut w = mundo(0);
        w.step(DISPARAR);
        assert!(w.player_shots.live_count() > 0);
        correr(&mut w, NADA, 120);
        assert_eq!(w.player_shots.live_count(), 0);
    }

    #[test]
    fn disparar_al_jefe_le_quita_vida() {
        let mut w = mundo(0);
        // Colocarse justo debajo del jefe para no fallar.
        w.player.pos = w.boss.pos + Vec2::new(0.0, 200.0);
        let vida = w.boss.hp;
        correr(&mut w, DISPARAR, 60);
        assert!(w.boss.hp < vida, "deberia haberle hecho dano");
    }

    #[test]
    fn cambiar_de_fase_limpia_la_pantalla() {
        let mut w = World::new(0);
        correr(&mut w, NADA, 120); // que el jefe llene de balas
        assert!(w.bullets.live_count() > 0);

        // Bajarle la vida a uno y rematarlo con un disparo.
        w.boss.hp = 1;
        w.player.pos = w.boss.pos + Vec2::new(0.0, 150.0);
        let fase = w.boss.phase;
        correr(&mut w, DISPARAR, 40);

        assert_eq!(w.boss.phase, fase + 1, "deberia haber cambiado de fase");
        assert!(!w.victory, "aun quedan fases");
    }

    /// Remata al jefe actual bajandole la vida fase a fase.
    fn rematar_jefe(w: &mut World) {
        let fases = w.boss.phase_count();
        for _ in 0..fases {
            w.player.pos = w.boss.pos + Vec2::new(0.0, 150.0);
            w.boss.hp = 1;
            correr(w, DISPARAR, 40);
        }
    }

    #[test]
    fn caer_el_primer_jefe_da_paso_al_segundo() {
        let mut w = mundo(0);
        assert_eq!(w.boss_index, 0);
        let primero = w.boss.name.clone();

        rematar_jefe(&mut w);

        assert_eq!(w.boss_index, 1, "deberia haber pasado al siguiente");
        assert!(!w.victory, "aun quedan jefes");
        assert!(!w.boss.defeated, "el jefe nuevo llega entero");
        assert_ne!(w.boss.name, primero);
    }

    #[test]
    fn derrotar_a_los_tres_jefes_da_la_victoria() {
        let mut w = mundo(0);
        for _ in 0..w.boss_count() {
            rematar_jefe(&mut w);
        }
        assert!(w.victory);
        assert_eq!(w.boss_index, w.boss_count() - 1);
    }

    #[test]
    fn cambiar_de_jefe_limpia_los_disparos_en_vuelo() {
        let mut w = mundo(0);
        // A un solo golpe de caer, y en su ultima fase.
        w.boss.phase = w.boss.phase_count() - 1;
        w.boss.hp = 1;
        w.player.pos = w.boss.pos + Vec2::new(0.0, 300.0);

        // Hay que mirar justo el tick que lo tumba: si se sigue disparando
        // despues, lo que se cuenta son balas nuevas, no las heredadas.
        for _ in 0..300 {
            w.step(DISPARAR);
            if w.boss_index == 1 {
                break;
            }
        }
        assert_eq!(w.boss_index, 1, "el jefe deberia haber caido");
        assert_eq!(
            w.player_shots.live_count(),
            0,
            "heredarlos restaria vida al jefe nuevo antes de aparecer"
        );
    }

    #[test]
    fn quedarse_sin_vidas_es_derrota() {
        let mut w = mundo(0);
        assert_eq!(w.lives, STARTING_LIVES);
        for i in 0..STARTING_LIVES {
            w.player.iframes = 0;
            bala_encima(&mut w);
            w.step(NADA);
            assert_eq!(w.lives, STARTING_LIVES - i - 1);
        }
        assert!(w.defeat);
        assert!(w.is_over());
    }

    #[test]
    fn tras_la_derrota_el_jefe_se_para() {
        let mut w = World::new(0);
        for _ in 0..STARTING_LIVES {
            w.player.iframes = 0;
            bala_encima(&mut w);
            w.step(NADA);
        }
        assert!(w.defeat);
        correr(&mut w, NADA, 200);
        assert_eq!(
            w.bullets.live_count(),
            0,
            "no deberia salir ni una bala mas"
        );
    }

    #[test]
    fn tras_la_victoria_el_jefe_se_para() {
        let mut w = mundo(0);
        for _ in 0..w.boss_count() {
            rematar_jefe(&mut w);
        }
        let antes = w.bullets.live_count();
        correr(&mut w, NADA, 200);
        assert!(w.bullets.live_count() <= antes);
    }

    /// Dispara `n` ticks con el jugador aparcado debajo del jefe e invulnerable.
    ///
    /// Hace falta cuando el jefe esta activo: si no, el jugador muere,
    /// reaparece lejos y deja de acertar, y el test acaba midiendo su
    /// supervivencia en vez de lo que pretende medir.
    fn disparar_a_bocajarro(w: &mut World, n: usize) {
        for _ in 0..n {
            w.player.pos = w.boss.pos + Vec2::new(0.0, 150.0);
            w.player.iframes = 10;
            w.step(DISPARAR);
        }
    }

    #[test]
    fn el_jefe_siguiente_entra_disparando() {
        let mut w = World::new(0);
        let fases = w.boss.phase_count();
        for _ in 0..fases {
            w.boss.hp = 1;
            disparar_a_bocajarro(&mut w, 40);
        }
        assert_eq!(w.boss_index, 1);
        // Al jefe nuevo se le da un momento para arrancar su patron.
        for _ in 0..60 {
            w.player.iframes = 30;
            w.step(NADA);
        }
        assert!(
            w.bullets.live_count() > 0,
            "el jefe 2 deberia estar disparando"
        );
    }

    #[test]
    fn recargar_los_jefes_los_reinicia_sin_perder_la_partida() {
        let mut w = World::new(0);
        correr(&mut w, NADA, 120);
        w.boss.hp = 50;
        let muertes = w.player.deaths;
        let vidas = w.lives;

        w.reload_bosses(crate::boss::BossDef::default_bosses());

        assert_eq!(w.boss.phase, 0, "el jefe vuelve al principio");
        assert_eq!(w.bullets.live_count(), 0, "y la pantalla se limpia");
        assert_eq!(w.player.deaths, muertes, "pero la partida sigue");
        assert_eq!(w.lives, vidas);
    }

    #[test]
    fn el_mundo_sandbox_tiene_al_jefe_quieto() {
        let mut w = mundo(0);
        let pos = w.boss.pos;
        correr(&mut w, NADA, 300);
        assert_eq!(w.bullets.live_count(), 0);
        assert_eq!(w.boss.pos, pos, "ni dispara ni se mueve");
    }
    // --- Parry, graze y super ---

    const PARRY: InputFrame = InputFrame::from_bits(InputFrame::PARRY);
    const SUPER: InputFrame = InputFrame::from_bits(InputFrame::SUPER);

    /// Pone una bala parryable justo al lado del jugador.
    fn bala_rosa(w: &mut World, dist: f32) {
        let pos = w.player.pos + Vec2::new(dist, 0.0);
        w.bullets
            .spawn(crate::bullets::Spawn {
                pos,
                flags: crate::bullets::FLAG_PARRYABLE,
                ..Default::default()
            })
            .unwrap();
    }

    #[test]
    fn el_parry_neutraliza_las_rosas_y_llena_el_medidor() {
        let mut w = mundo(0);
        bala_rosa(&mut w, 20.0);
        assert_eq!(w.player.meter, 0.0);

        w.step(PARRY);

        assert_eq!(
            w.bullets.live_count(),
            0,
            "la bala rosa deberia desaparecer"
        );
        assert_eq!(w.player.parries, 1);
        assert!((w.player.meter - PARRY_METER).abs() < 1e-3);
    }

    #[test]
    fn el_parry_no_toca_las_balas_normales() {
        let mut w = mundo(0);
        // Normal, sin FLAG_PARRYABLE.
        let pos = w.player.pos + Vec2::new(20.0, 0.0);
        w.bullets
            .spawn(crate::bullets::Spawn {
                pos,
                ..Default::default()
            })
            .unwrap();

        w.step(PARRY);

        assert_eq!(
            w.bullets.live_count(),
            1,
            "el parry no es un limpiapantallas"
        );
        assert_eq!(w.player.parries, 0);
    }

    #[test]
    fn el_parry_no_alcanza_lo_que_esta_lejos() {
        let mut w = mundo(0);
        bala_rosa(&mut w, PARRY_RADIUS + 40.0);
        w.step(PARRY);
        assert_eq!(w.bullets.live_count(), 1);
        assert_eq!(w.player.parries, 0);
    }

    #[test]
    fn un_parry_acertado_da_invulnerabilidad() {
        let mut w = mundo(0);
        bala_rosa(&mut w, 20.0);
        w.step(PARRY);
        assert!(
            w.player.is_invulnerable(),
            "meterse a parriar tiene que protegerte, o nunca compensa"
        );
    }

    #[test]
    fn mantener_el_boton_de_parry_no_lo_encadena() {
        let mut w = mundo(0);
        // Mantener pulsado mucho mas que el enfriamiento.
        correr(&mut w, PARRY, PARRY_COOLDOWN_TICKS as usize * 3);
        bala_rosa(&mut w, 20.0);
        correr(&mut w, PARRY, 10);
        assert_eq!(
            w.player.parries, 0,
            "el parry reacciona al flanco, no a mantener la tecla"
        );
    }

    #[test]
    fn el_parry_respeta_su_enfriamiento() {
        let mut w = mundo(0);
        w.step(PARRY); // falla, no habia nada que parriar

        // Esperar a que cierre la ventana, pero no a que acabe el enfriamiento.
        correr(&mut w, NADA, PARRY_WINDOW_TICKS as usize + 2);
        assert!(
            !w.player.is_parrying(),
            "la ventana ya deberia estar cerrada"
        );
        assert!(w.player.parry_cooldown > 0, "pero el enfriamiento sigue");

        bala_rosa(&mut w, 20.0);
        w.step(PARRY);
        assert_eq!(w.player.parries, 0, "fallar el parry te deja vendido");

        correr(&mut w, NADA, PARRY_COOLDOWN_TICKS as usize);
        w.step(PARRY);
        assert_eq!(w.player.parries, 1, "y luego vuelve a estar listo");
    }

    #[test]
    fn el_graze_llena_el_medidor_a_goteo() {
        let mut w = mundo(0);
        let pos = w.player.pos + Vec2::new(GRAZE_RADIUS * 0.5, 0.0);
        w.bullets
            .spawn(crate::bullets::Spawn {
                pos,
                ..Default::default()
            })
            .unwrap();

        w.step(NADA);
        assert_eq!(w.player.grazes, 1);
        assert!((w.player.meter - GRAZE_METER).abs() < 1e-3);

        // La misma bala quieta no vuelve a pagar.
        correr(&mut w, NADA, 10);
        assert_eq!(w.player.grazes, 1);
    }

    #[test]
    fn el_graze_no_llega_a_lo_que_pasa_lejos() {
        let mut w = mundo(0);
        let pos = w.player.pos + Vec2::new(GRAZE_RADIUS + 60.0, 0.0);
        w.bullets
            .spawn(crate::bullets::Spawn {
                pos,
                ..Default::default()
            })
            .unwrap();
        correr(&mut w, NADA, 5);
        assert_eq!(w.player.grazes, 0);
        assert_eq!(w.player.meter, 0.0);
    }

    #[test]
    fn el_super_necesita_el_medidor_lleno() {
        let mut w = mundo(0);
        w.boss.hp = 500;
        let vida = w.boss.hp;
        w.player.meter = METER_MAX - 1.0;

        w.step(SUPER);
        assert_eq!(w.boss.hp, vida, "sin medidor lleno no sale");
        assert!(w.player.meter > 0.0, "y no se gasta");
    }

    #[test]
    fn el_super_limpia_la_pantalla_y_pega_fuerte() {
        let mut w = mundo(0);
        w.boss.hp = 500;
        for i in 0..30 {
            w.bullets
                .spawn(crate::bullets::Spawn {
                    pos: Vec2::new(60.0 + i as f32 * 4.0, 300.0),
                    ..Default::default()
                })
                .unwrap();
        }
        w.player.meter = METER_MAX;
        let vida = w.boss.hp;

        w.step(SUPER);

        assert_eq!(w.bullets.live_count(), 0);
        assert_eq!(w.boss.hp, vida - SUPER_DAMAGE);
        assert_eq!(w.player.meter, 0.0, "todo o nada");
        assert!(w.player.is_invulnerable(), "y protege mientras dura");
    }

    #[test]
    fn morir_vacia_el_medidor() {
        let mut w = mundo(0);
        w.player.meter = METER_MAX;
        bala_encima(&mut w);
        w.step(NADA);
        assert_eq!(
            w.player.meter, 0.0,
            "si morir no costase el medidor, seria casi gratis"
        );
    }

    #[test]
    fn el_jefe_dispara_balas_parryables() {
        let mut w = World::new(0);
        // El patron mete rosas cada tanto; hay que darle tiempo a la rafaga
        // apuntada, que es la que las lanza.
        correr(&mut w, NADA, 200);
        let rosas = w
            .bullets
            .iter_live()
            .filter(|b| b.flags & crate::bullets::FLAG_PARRYABLE != 0)
            .count();
        assert!(rosas > 0, "sin balas rosas el parry no tendria a que jugar");
    }
}
