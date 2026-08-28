//! El jefe: fases, vida y ejecucion de su patron.
//!
//! Un jefe es una lista de fases. Cada fase tiene su propia vida y su propio
//! patron, y se pasa a la siguiente cuando la vida de la actual llega a cero.
//! Es la estructura canonica del genero, y la razon por la que un boss-rush se
//! puede ampliar indefinidamente sin tocar codigo: un jefe nuevo es un fichero
//! RON nuevo.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::bullets::Bullets;
use crate::hash::Fnv1a;
use crate::pattern::{Pattern, PatternRunner, RunCtx, Step};

/// Los jefes del boss-rush, en orden, embebidos en el binario.
///
/// Se embeben para que el build web funcione sin sistema de ficheros, y para
/// que el juego arranque aunque alguien deje un RON a medias en disco. En
/// nativo se prefieren los ficheros, que es lo que permite el hot-reload.
///
/// **Anadir un jefe es anadir un fichero y una linea aqui.** Ni una linea de
/// codigo mas: esa era toda la razon de ser del interprete de patrones.
/// Los jefes de la partida: **un baile cada uno**.
///
/// El vals entero cabe en `boss1.ron` y el tango en `boss2.ron`, cada uno con
/// una fase por figura suya. Anadir un baile es anadir su fichero y una linea
/// aqui: la pista le pone el nodo sola y la musica lo busca por indice.
pub const DEFAULT_BOSS_RONS: [&str; 2] = [
    include_str!("../../../assets/patterns/boss1.ron"),
    include_str!("../../../assets/patterns/boss2.ron"),
];

/// El primer jefe. Se conserva por comodidad y para los tests.
pub const DEFAULT_BOSS_RON: &str = DEFAULT_BOSS_RONS[0];

/// Ticks que el jefe parpadea al recibir dano.
const HIT_FLASH_TICKS: u32 = 4;

/// Una fase, tal y como se escribe en el RON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhaseDef {
    /// La figura del baile que es esta fase.
    ///
    /// Un baile es un jefe, y cada fase es una figura suya: el paso base, el
    /// espejo, el molinete, la coda. El nombre es lo que lo cuenta en pantalla,
    /// y es lo que impide que las fases se conviertan en "fase 1, fase 2".
    /// Opcional: un RON viejo sin nombre sigue cargando.
    #[serde(default)]
    pub name: String,
    /// Vida de esta fase. Al llegar a cero se pasa a la siguiente.
    pub hp: i32,
    pub steps: Vec<Step>,
}

/// La definicion completa de un jefe.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BossDef {
    pub name: String,
    pub pos: Vec2,
    /// Radio para recibir los disparos del jugador. No colisiona con nada mas.
    pub radius: f32,
    pub phases: Vec<PhaseDef>,
}

impl BossDef {
    /// Lee una definicion desde texto RON.
    ///
    /// Se descarta el BOM inicial si lo hay. Varios editores de Windows —y
    /// PowerShell con `Set-Content -Encoding utf8`— lo escriben por defecto, y
    /// sin esto el parser falla en la posicion 1:1 con un mensaje que no
    /// menciona el BOM por ningun lado. Es un fallo carisimo de diagnosticar
    /// para lo trivial que resulta tolerarlo.
    pub fn from_ron(src: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(src.trim_start_matches('\u{feff}'))
    }

    /// La definicion embebida del primer jefe.
    pub fn default_boss() -> Self {
        // Si esto falla, el fichero de assets esta roto y no hay nada que
        // hacer: es un error de compilacion disfrazado de error de ejecucion.
        Self::from_ron(DEFAULT_BOSS_RON).expect("el RON embebido debe ser valido")
    }

    /// Todos los jefes embebidos, en orden de aparicion.
    pub fn default_bosses() -> Vec<Self> {
        DEFAULT_BOSS_RONS
            .iter()
            .enumerate()
            .map(|(i, ron)| {
                Self::from_ron(ron).unwrap_or_else(|e| {
                    panic!("el RON embebido del jefe {} es invalido: {e}", i + 1)
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct CompiledPhase {
    name: String,
    hp: i32,
    pattern: Pattern,
}

/// El jefe en ejecucion.
#[derive(Clone, Debug)]
pub struct Boss {
    pub name: String,
    pub pos: Vec2,
    /// Posicion del tick anterior, para que el render interpole.
    pub prev_pos: Vec2,
    pub radius: f32,
    pub hp: i32,
    pub phase: usize,
    pub defeated: bool,
    /// Ticks de parpadeo por impacto. Sin esto disparar no da ninguna
    /// sensacion de estar haciendo algo.
    pub hit_flash: u32,
    phases: Vec<CompiledPhase>,
    runner: PatternRunner,
    start_pos: Vec2,
}

impl Boss {
    pub fn from_def(def: &BossDef) -> Self {
        let phases: Vec<CompiledPhase> = def
            .phases
            .iter()
            .map(|p| CompiledPhase {
                name: p.name.clone(),
                hp: p.hp.max(1),
                pattern: Pattern::compile(&p.steps),
            })
            .collect();

        // Un jefe sin fases seria un juego sin juego, pero tampoco merece un
        // panic: se degrada a un jefe inerte con un punto de vida.
        let hp = phases.first().map(|p| p.hp).unwrap_or(1);
        let runner = PatternRunner::new(
            phases
                .first()
                .map(|p| p.pattern.clone())
                .unwrap_or_else(|| Pattern::compile(&[])),
        );

        Self {
            name: def.name.clone(),
            pos: def.pos,
            prev_pos: def.pos,
            radius: def.radius,
            hp,
            phase: 0,
            defeated: phases.is_empty(),
            hit_flash: 0,
            phases,
            runner,
            start_pos: def.pos,
        }
    }

    pub fn phase_count(&self) -> usize {
        self.phases.len()
    }

    /// La figura que se esta bailando ahora mismo.
    pub fn phase_name(&self) -> &str {
        self.phases
            .get(self.phase)
            .map(|p| p.name.as_str())
            .unwrap_or("")
    }

    pub fn phase_max_hp(&self) -> i32 {
        self.phases.get(self.phase).map(|p| p.hp).unwrap_or(1)
    }

    /// Vida de la fase actual en `[0, 1]`. Es lo que dibuja la barra.
    pub fn hp_ratio(&self) -> f32 {
        (self.hp as f32 / self.phase_max_hp() as f32).clamp(0.0, 1.0)
    }

    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev_pos.lerp(self.pos, alpha)
    }

    /// Avanza el patron un tick.
    pub fn update(&mut self, bullets: &mut Bullets, player: Vec2) {
        self.prev_pos = self.pos;
        self.hit_flash = self.hit_flash.saturating_sub(1);
        if self.defeated {
            return;
        }

        let mut ctx = RunCtx {
            bullets,
            origin: &mut self.pos,
            player,
        };
        self.runner.tick(&mut ctx);

        // Un patron que se acaba se reinicia. Asi una fase puede escribirse
        // como una secuencia finita sin tener que envolverla en `Forever`.
        if self.runner.finished() {
            self.runner.restart();
        }
    }

    /// Aplica dano. Devuelve `true` si ha cambiado de fase o ha caido.
    pub fn damage(&mut self, amount: i32) -> bool {
        if self.defeated || amount <= 0 {
            return false;
        }
        self.hp -= amount;
        self.hit_flash = HIT_FLASH_TICKS;
        if self.hp > 0 {
            return false;
        }

        if self.phase + 1 < self.phases.len() {
            self.phase += 1;
            self.hp = self.phases[self.phase].hp;
            self.pos = self.start_pos;
            self.prev_pos = self.start_pos;
            self.runner = PatternRunner::new(self.phases[self.phase].pattern.clone());
        } else {
            self.hp = 0;
            self.defeated = true;
        }
        true
    }

    pub(crate) fn hash_into(&self, h: &mut Fnv1a) {
        h.write_vec2(self.pos);
        h.write_vec2(self.prev_pos);
        h.write_u64(self.hp as i64 as u64);
        h.write_u64(self.phase as u64);
        h.write_u64(u64::from(self.defeated));
        h.write_u64(self.runner.ticks());
        h.write_u64(self.runner.live_threads() as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bullets::Bullets;

    fn def_de_prueba() -> BossDef {
        BossDef {
            name: "Prueba".into(),
            pos: Vec2::new(320.0, 130.0),
            radius: 30.0,
            phases: vec![
                PhaseDef {
                    name: "Primera".into(),
                    hp: 10,
                    steps: vec![Step::Forever(vec![
                        Step::Fire(crate::emitter::EmitterSpec::ring(4, 100.0)),
                        Step::Wait(5),
                    ])],
                },
                PhaseDef {
                    name: "Segunda".into(),
                    hp: 20,
                    steps: vec![Step::Forever(vec![
                        Step::Fire(crate::emitter::EmitterSpec::ring(8, 120.0)),
                        Step::Wait(3),
                    ])],
                },
            ],
        }
    }

    #[test]
    fn los_jefes_embebidos_son_validos() {
        let defs = BossDef::default_bosses();
        // Uno por baile: el vals y el tango.
        assert_eq!(defs.len(), 2);
        for (i, d) in defs.iter().enumerate() {
            assert!(!d.name.is_empty(), "el jefe {} no tiene nombre", i + 1);
            assert!(!d.phases.is_empty(), "el jefe {} no tiene fases", i + 1);
            let b = Boss::from_def(d);
            assert!(!b.defeated);
        }
    }

    #[test]
    fn cada_jefe_tiene_su_nombre() {
        let nombres: Vec<String> = BossDef::default_bosses()
            .iter()
            .map(|d| d.name.clone())
            .collect();
        let unicos: std::collections::BTreeSet<_> = nombres.iter().collect();
        assert_eq!(
            unicos.len(),
            nombres.len(),
            "nombres repetidos: {nombres:?}"
        );
    }

    /// Cuantas balas dispara una fase en `ticks`, sin dejarlas morir.
    ///
    /// Se ejecuta el patron sin llamar a `bullets.update()`: nada se mueve ni
    /// caduca, asi que las vivas al final son exactamente las disparadas.
    pub(super) fn balas_de_la_fase(fase: &PhaseDef, ticks: usize) -> usize {
        use crate::pattern::{Pattern, PatternRunner, RunCtx};
        let mut runner = PatternRunner::new(Pattern::compile(&fase.steps));
        let mut bullets = Bullets::with_capacity(crate::MAX_BULLETS);
        let mut origen = Vec2::new(320.0, 140.0);
        for _ in 0..ticks {
            let mut ctx = RunCtx {
                bullets: &mut bullets,
                origin: &mut origen,
                player: Vec2::new(320.0, 620.0),
            };
            runner.tick(&mut ctx);
        }
        bullets.live_count()
    }

    /// Si en algun sitio del arbol hay un `Turn`, o sea una espiral.
    fn hay_espirales(steps: &[Step]) -> bool {
        steps.iter().any(|paso| match paso {
            Step::Turn(_) => true,
            Step::Repeat { body, .. } | Step::Forever(body) => hay_espirales(body),
            Step::Parallel(ramas) => ramas.iter().any(|r| hay_espirales(r)),
            _ => false,
        })
    }

    /// La regla que separa un baile de otro, y por tanto la que hay que
    /// vigilar.
    ///
    /// Una espiral es un giro continuo: es lo que hace un vals, y el vals las
    /// usa en las cuatro figuras. El tango va en linea recta y cambia de
    /// golpe, asi que no tiene ninguna. Si algun dia se cuela una, las dos
    /// gramaticas empiezan a parecerse y "cada jefe es un baile" deja de ser
    /// verdad sin que nadie se entere.
    #[test]
    fn el_vals_gira_y_el_tango_no() {
        let defs = BossDef::default_bosses();
        let vals = &defs[0];
        let tango = &defs[1];
        assert!(
            vals.phases.iter().all(|f| hay_espirales(&f.steps)),
            "el vals sin espirales ha dejado de ser un vals"
        );
        for f in &tango.phases {
            assert!(
                !hay_espirales(&f.steps),
                "{} tiene una espiral, y eso es un vals",
                f.name
            );
        }
    }

    #[test]
    fn la_ultima_fase_de_cada_jefe_es_la_mas_densa() {
        // Dos veces seguidas, jugando, la fase final resulto ser la mas facil:
        // en El Espejo porque disparaba la MITAD que la primera, y en La Coda
        // porque eran dos espirales limpias. Subir la vida de una fase no la
        // hace mas dificil; esto vigila que el climax lo sea de verdad.
        //
        // Es una condicion necesaria, no suficiente: la legibilidad importa
        // tanto como la densidad, y eso no lo mide un test.
        for def in BossDef::default_bosses() {
            let densidad: Vec<usize> = def
                .phases
                .iter()
                .map(|f| balas_de_la_fase(f, 600))
                .collect();
            let maxima = *densidad.iter().max().unwrap();
            assert_eq!(
                *densidad.last().unwrap(),
                maxima,
                "la ultima fase de {} no es la mas densa: {densidad:?}",
                def.name
            );
        }
    }

    #[test]
    fn los_jefes_se_ponen_mas_duros() {
        let vidas: Vec<i32> = BossDef::default_bosses()
            .iter()
            .map(|d| d.phases.iter().map(|f| f.hp).sum())
            .collect();
        assert!(
            vidas.windows(2).all(|w| w[1] > w[0]),
            "cada jefe deberia tener mas vida que el anterior: {vidas:?}"
        );
    }

    #[test]
    fn el_ron_embebido_es_valido_y_compila() {
        let def = BossDef::default_boss();
        assert!(!def.phases.is_empty(), "el jefe necesita al menos una fase");
        assert!(!def.name.is_empty());
        let boss = Boss::from_def(&def);
        assert!(!boss.defeated);
        assert_eq!(boss.phase_count(), def.phases.len());
    }

    #[test]
    fn el_vals_tiene_sus_cuatro_figuras() {
        let def = BossDef::default_boss();
        assert_eq!(def.phases.len(), 4);
        let nombres: Vec<&str> = def.phases.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            nombres,
            vec!["El paso base", "El espejo", "El molinete", "La coda"]
        );
    }

    #[test]
    fn todas_las_fases_tienen_nombre_de_figura() {
        // Un baile es un jefe y cada fase es una figura suya. Una fase sin
        // nombre es una fase que ha vuelto a ser "fase 3", que es justo lo que
        // este diseno no quiere.
        for def in BossDef::default_bosses() {
            for (i, f) in def.phases.iter().enumerate() {
                assert!(
                    !f.name.is_empty(),
                    "{}: la fase {} no tiene figura",
                    def.name,
                    i + 1
                );
            }
        }
    }

    #[test]
    fn el_jefe_dispara() {
        let mut boss = Boss::from_def(&def_de_prueba());
        let mut b = Bullets::with_capacity(1024);
        for _ in 0..30 {
            boss.update(&mut b, Vec2::new(320.0, 600.0));
        }
        assert!(b.live_count() > 0);
    }

    #[test]
    fn el_dano_cambia_de_fase_al_agotar_la_vida() {
        let mut boss = Boss::from_def(&def_de_prueba());
        assert_eq!(boss.phase, 0);
        assert!(!boss.damage(9), "aun le queda vida");
        assert!(boss.damage(1), "aqui cambia de fase");
        assert_eq!(boss.phase, 1);
        assert_eq!(boss.hp, 20, "la fase nueva trae su propia vida");
        assert!(!boss.defeated);
    }

    #[test]
    fn agotar_la_ultima_fase_lo_derrota() {
        let mut boss = Boss::from_def(&def_de_prueba());
        boss.damage(10);
        boss.damage(20);
        assert!(boss.defeated);
        assert_eq!(boss.hp, 0);
    }

    #[test]
    fn un_jefe_derrotado_deja_de_disparar_y_de_recibir_dano() {
        let mut boss = Boss::from_def(&def_de_prueba());
        boss.damage(10);
        boss.damage(20);
        let mut b = Bullets::with_capacity(1024);
        for _ in 0..60 {
            boss.update(&mut b, Vec2::new(320.0, 600.0));
        }
        assert_eq!(b.live_count(), 0);
        assert!(!boss.damage(5), "ya no acepta dano");
    }

    #[test]
    fn la_barra_de_vida_va_de_uno_a_cero() {
        let mut boss = Boss::from_def(&def_de_prueba());
        assert!((boss.hp_ratio() - 1.0).abs() < 1e-6);
        boss.damage(5);
        assert!((boss.hp_ratio() - 0.5).abs() < 1e-6);
        boss.damage(5);
        // Ya en la fase 2, la barra vuelve a estar llena.
        assert!((boss.hp_ratio() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn el_impacto_enciende_el_parpadeo() {
        let mut boss = Boss::from_def(&def_de_prueba());
        assert_eq!(boss.hit_flash, 0);
        boss.damage(1);
        assert!(boss.hit_flash > 0);
        let mut b = Bullets::with_capacity(64);
        for _ in 0..10 {
            boss.update(&mut b, Vec2::ZERO);
        }
        assert_eq!(boss.hit_flash, 0, "el parpadeo se apaga solo");
    }

    #[test]
    fn un_jefe_sin_fases_no_revienta() {
        let def = BossDef {
            name: "Vacio".into(),
            pos: Vec2::ZERO,
            radius: 10.0,
            phases: vec![],
        };
        let mut boss = Boss::from_def(&def);
        assert!(boss.defeated);
        let mut b = Bullets::with_capacity(16);
        boss.update(&mut b, Vec2::ZERO);
        assert_eq!(b.live_count(), 0);
    }

    #[test]
    fn un_ron_con_bom_se_lee_igual() {
        let con_bom = format!("\u{feff}{DEFAULT_BOSS_RON}");
        let def = BossDef::from_ron(&con_bom).expect("el BOM no deberia estorbar");
        assert_eq!(def.phases.len(), 4);
    }

    #[test]
    fn un_ron_invalido_da_error_en_vez_de_panic() {
        assert!(BossDef::from_ron("esto no es RON valido {{{").is_err());
    }
}

#[cfg(test)]
mod medida {
    use super::tests::balas_de_la_fase;
    use super::*;

    /// Imprime las balas por segundo de cada fase de cada jefe.
    ///
    /// `cargo test -p vals-core densidades -- --nocapture --ignored`
    #[test]
    #[ignore]
    fn densidades() {
        for def in BossDef::default_bosses() {
            println!("{}", def.name);
            for f in &def.phases {
                let n = balas_de_la_fase(f, 600);
                println!(
                    "  {:<14} hp {:>5}  {:>6.0} balas/s",
                    f.name,
                    f.hp,
                    n as f32 / 10.0
                );
            }
        }
    }
}
