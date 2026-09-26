//! Grabacion y reproduccion de partidas.
//!
//! Un replay es **una semilla mas la lista de inputs**. Nada mas. Como la
//! simulacion es determinista, reproducir esos inputs sobre un mundo nuevo
//! recrea la partida entera tick a tick, y no hace falta guardar ni una
//! posicion.
//!
//! Esto es la **red de seguridad del proyecto**, y por eso va antes que la
//! optimizacion del render: con un replay grabado y las huellas de estado
//! guardadas, se puede reescribir el bucle caliente con SIMD o con hilos y
//! demostrar que el comportamiento no cambio ni un frame. Sin esto, optimizar
//! da miedo y no se hace.
//!
//! De propina salen tres cosas por el mismo precio: tests de regresion de
//! jugabilidad, material para GIFs de portfolio y un modo atractor.

use crate::boss::DEFAULT_BOSS_RONS;
use crate::hash::Fnv1a;
use crate::{InputFrame, Mode, World};

/// Cabecera del formato. El ultimo digito es la version.
/// Cabecera del formato. El ultimo digito es la version; subio a 2 al anadir
/// el modo de juego, porque un replay de plataformas no se puede reproducir
/// como si fuera de vuelo.
const MAGIC: &[u8; 8] = b"VALSRPL3";

/// Cada cuantos ticks se guarda una huella de estado.
///
/// Uno por segundo. Guardar una por tick multiplicaria por ocho el tamano del
/// fichero para localizar la divergencia con una precision que no hace falta:
/// con el tick del checkpoint ya sabes en que segundo mirar.
pub const DEFAULT_CHECKPOINT_EVERY: u32 = 60;

/// Una huella del mundo en un tick concreto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub tick: u64,
    pub hash: u64,
}

/// Una partida grabada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replay {
    pub seed: u64,
    /// Huella del patron del jefe con el que se grabo.
    ///
    /// Se guarda porque el jefe se carga desde un RON que se puede editar en
    /// caliente. Sin esto, reproducir un replay tras tocar el patron daria una
    /// divergencia desconcertante en vez de un mensaje que explica que ha
    /// cambiado el jefe, no el motor.
    pub pattern_hash: u64,
    pub checkpoint_every: u32,
    /// Volando o pisando el suelo. Sin esto, un replay de plataformas se
    /// reproduciria con las reglas del otro modo y divergeria en el tick uno.
    pub mode: Mode,
    /// Que baile se estaba bailando.
    ///
    /// Misma leccion que el modo, y se aprendio dos veces: en cuanto una
    /// partida puede empezar en algo que no sea el primer jefe, el numero que
    /// dice **en cual** empieza forma parte de lo que hace falta para
    /// reproducirla. Sin esto, un replay del charleston se reproducia contra el
    /// vals y divergia en el primer tick.
    pub baile: u8,
    pub inputs: Vec<InputFrame>,
    pub checkpoints: Vec<Checkpoint>,
}

/// Huella de **todos** los patrones embebidos.
///
/// Cubre los tres jefes, no solo el primero: un replay recorre el boss-rush
/// entero, asi que tocar el jefe 3 tambien lo invalida.
pub fn default_pattern_hash() -> u64 {
    hash_patterns(DEFAULT_BOSS_RONS)
}

/// La huella de unos patrones, **sin contar los finales de linea**.
///
/// Un RON guardado con finales de Windows y el mismo con finales de Unix
/// describen el jefe identico, asi que no pueden dar huellas distintas: si no,
/// el replay dorado se rompe segun con que editor —o con que script— se toco el
/// fichero por ultima vez. Costo un CI en rojo aprenderlo: se grabo en Windows
/// con CRLF y fallo en Linux con LF, sin que el jefe hubiese cambiado en nada.
fn hash_patterns<'a>(rons: impl IntoIterator<Item = &'a str>) -> u64 {
    let mut h = Fnv1a::new();
    for ron in rons {
        // Partir por '\r' y escribir los trozos es escribir la cadena sin
        // ningun retorno de carro, y sin reservar memoria.
        for trozo in ron.split('\r') {
            h.write_str(trozo);
        }
    }
    h.finish()
}

impl Replay {
    pub fn ticks(&self) -> u64 {
        self.inputs.len() as u64
    }

    /// Duracion en segundos de simulacion.
    pub fn seconds(&self) -> f32 {
        self.ticks() as f32 * crate::DT
    }

    /// Serializa a un formato binario compacto.
    ///
    /// Se escribe a mano en vez de tirar de serde porque el formato importa: a
    /// dos bytes por tick, media hora de partida ocupa 216 KB, y un formato
    /// textual multiplicaria eso por diez sin ganar nada. Ademas asi el
    /// fichero no depende de como serialice serde un `enum` este ano.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + self.inputs.len() * 2 + self.checkpoints.len() * 16);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.seed.to_le_bytes());
        out.extend_from_slice(&self.pattern_hash.to_le_bytes());
        out.extend_from_slice(&self.checkpoint_every.to_le_bytes());
        out.push(self.mode.as_u8());
        out.push(self.baile);
        out.extend_from_slice(&(self.inputs.len() as u32).to_le_bytes());
        for i in &self.inputs {
            out.extend_from_slice(&i.bits().to_le_bytes());
        }
        out.extend_from_slice(&(self.checkpoints.len() as u32).to_le_bytes());
        for c in &self.checkpoints {
            out.extend_from_slice(&c.tick.to_le_bytes());
            out.extend_from_slice(&c.hash.to_le_bytes());
        }
        out
    }

    /// Lee un replay. Nunca entra en panico con un fichero corrupto.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplayError> {
        let mut c = Cursor::new(bytes);
        if c.take(8)? != MAGIC {
            return Err(ReplayError::BadMagic);
        }
        let seed = c.u64()?;
        let pattern_hash = c.u64()?;
        let checkpoint_every = c.u32()?;
        let mode = Mode::from_u8(c.u8()?);
        let baile = c.u8()?;

        let n_inputs = c.u32()? as usize;
        // Se comprueba que los bytes existan antes de reservar: sin esto, un
        // fichero corrupto que diga "cuatro mil millones de inputs" reservaria
        // gigabytes antes de fallar.
        c.ensure(n_inputs * 2)?;
        let mut inputs = Vec::with_capacity(n_inputs);
        for _ in 0..n_inputs {
            inputs.push(InputFrame::from_bits(c.u16()?));
        }

        let n_checks = c.u32()? as usize;
        c.ensure(n_checks * 16)?;
        let mut checkpoints = Vec::with_capacity(n_checks);
        for _ in 0..n_checks {
            checkpoints.push(Checkpoint {
                tick: c.u64()?,
                hash: c.u64()?,
            });
        }

        Ok(Self {
            seed,
            pattern_hash,
            checkpoint_every: checkpoint_every.max(1),
            mode,
            baile,
            inputs,
            checkpoints,
        })
    }

    /// Reproduce el replay y comprueba que el mundo pasa por las mismas
    /// huellas.
    ///
    /// Es la funcion que da sentido a todo el modulo: si esto pasa, el cambio
    /// que acabas de hacer en el motor no ha alterado la jugabilidad.
    pub fn verify(&self) -> Result<VerifyReport, VerifyError> {
        if self.pattern_hash != default_pattern_hash() {
            return Err(VerifyError::PatternChanged);
        }
        if self.checkpoints.is_empty() {
            return Err(VerifyError::NoCheckpoints);
        }

        let mut world = World::empezar_en(self.seed, self.mode, self.baile as usize);
        let mut siguiente = 0usize;

        for input in self.inputs.iter().copied() {
            world.step(input);
            let Some(cp) = self.checkpoints.get(siguiente) else {
                continue;
            };
            if cp.tick != world.tick {
                continue;
            }
            let actual = world.state_hash();
            if actual != cp.hash {
                return Err(VerifyError::Divergence {
                    tick: world.tick,
                    expected: cp.hash,
                    actual,
                });
            }
            siguiente += 1;
        }

        Ok(VerifyReport {
            ticks: world.tick,
            checkpoints: siguiente,
            final_hash: world.state_hash(),
        })
    }
}

/// Resultado de una verificacion correcta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifyReport {
    pub ticks: u64,
    pub checkpoints: usize,
    pub final_hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayError {
    BadMagic,
    Truncated,
}

impl core::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadMagic => write!(f, "no parece un replay de VALS"),
            Self::Truncated => write!(f, "el fichero esta cortado o corrupto"),
        }
    }
}

impl std::error::Error for ReplayError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    /// El mundo se salio del guion. `tick` es donde se noto.
    Divergence {
        tick: u64,
        expected: u64,
        actual: u64,
    },
    /// El RON del jefe ha cambiado desde que se grabo.
    PatternChanged,
    NoCheckpoints,
}

impl core::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Divergence {
                tick,
                expected,
                actual,
            } => write!(
                f,
                "divergencia en el tick {tick}: se esperaba {expected:#018x} y salio {actual:#018x}"
            ),
            Self::PatternChanged => write!(
                f,
                "el patron del jefe ha cambiado desde que se grabo este replay"
            ),
            Self::NoCheckpoints => write!(f, "el replay no tiene huellas que comprobar"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Graba una partida mientras se juega.
#[derive(Clone, Debug)]
pub struct Recorder {
    replay: Replay,
}

impl Recorder {
    pub fn new(seed: u64, mode: Mode, baile: usize) -> Self {
        Self::with_interval(seed, mode, baile, DEFAULT_CHECKPOINT_EVERY)
    }

    /// Graba lo que haga falta para reproducir este mundo.
    pub fn for_world(world: &World) -> Self {
        Self::new(world.seed(), world.mode, world.baile)
    }

    pub fn with_interval(seed: u64, mode: Mode, baile: usize, checkpoint_every: u32) -> Self {
        Self {
            replay: Replay {
                seed,
                pattern_hash: default_pattern_hash(),
                checkpoint_every: checkpoint_every.max(1),
                mode,
                baile: baile.min(u8::MAX as usize) as u8,
                inputs: Vec::new(),
                checkpoints: Vec::new(),
            },
        }
    }

    /// Anota un tick. Hay que llamarlo **justo despues** de `world.step(input)`
    /// y con el mismo input.
    pub fn record(&mut self, input: InputFrame, world: &World) {
        self.replay.inputs.push(input);
        if world
            .tick
            .is_multiple_of(u64::from(self.replay.checkpoint_every))
        {
            self.replay.checkpoints.push(Checkpoint {
                tick: world.tick,
                hash: world.state_hash(),
            });
        }
    }

    pub fn ticks(&self) -> u64 {
        self.replay.ticks()
    }

    pub fn is_empty(&self) -> bool {
        self.replay.inputs.is_empty()
    }

    pub fn replay(&self) -> &Replay {
        &self.replay
    }

    pub fn finish(self) -> Replay {
        self.replay
    }
}

/// Graba una partida a partir de una lista de inputs, sin jugarla.
///
/// La usan los tests y el generador del replay dorado.
pub fn record_scripted(
    seed: u64,
    mode: Mode,
    baile: usize,
    inputs: &[InputFrame],
    checkpoint_every: u32,
) -> Replay {
    let mut world = World::empezar_en(seed, mode, baile);
    // Se graba el modo del mundo y no el pedido: un baile de suelo se baila en
    // el suelo aunque se pida vuelo, y el fichero tiene que decir la verdad.
    let mut rec = Recorder::with_interval(seed, world.mode, baile, checkpoint_every);
    for input in inputs.iter().copied() {
        world.step(input);
        rec.record(input, &world);
    }
    rec.finish()
}

/// El replay dorado, versionado en el repo.
///
/// Es una partida guardada cuyas huellas se calcularon con el codigo de un dia
/// concreto. El test que lo reproduce **no comprueba que el juego este bien**:
/// comprueba que el juego no ha *cambiado*. Es exactamente lo que hace falta
/// para poder meterse a optimizar el bucle caliente sin miedo.
///
/// Si tocas la jugabilidad o el patron del jefe, este test falla, y eso es
/// correcto: hay que volver a grabarlo a proposito con
/// `cargo run -p vals-core --example replay_tool -- record-golden`.
pub const GOLDEN_REPLAY: &[u8] = include_bytes!("../../../assets/replays/golden.valsrpl");

/// Ticks del replay dorado. Treinta segundos.
pub const GOLDEN_TICKS: usize = 1800;

/// Semilla del replay dorado. No se toca.
pub const GOLDEN_SEED: u64 = 0x601D_5EED;

/// Inputs del replay dorado.
///
/// No son aleatorios del todo a proposito: dispara siempre, cambia de rumbo
/// cada tanto y suelta dash, parry y super de vez en cuando. Asi el replay
/// ejercita las mecanicas de verdad en vez de dar espasmos, y encima se puede
/// usar como modo atractor.
pub fn golden_inputs(ticks: usize) -> Vec<InputFrame> {
    let mut r = crate::Pcg32::new(GOLDEN_SEED);
    let mut out = Vec::with_capacity(ticks);
    let mut rumbo = InputFrame::NONE;

    for i in 0..ticks {
        if i % 24 == 0 {
            rumbo = InputFrame::NONE;
            let d = r.next_u32() % 9;
            if d < 3 {
                rumbo.set(InputFrame::LEFT, true);
            } else if d < 6 {
                rumbo.set(InputFrame::RIGHT, true);
            }
            if d.is_multiple_of(3) {
                rumbo.set(InputFrame::UP, true);
            } else if d % 3 == 1 {
                rumbo.set(InputFrame::DOWN, true);
            }
        }
        let mut f = rumbo;
        f.set(InputFrame::SHOOT, true);
        f.set(InputFrame::FOCUS, r.next_f32() < 0.18);
        f.set(InputFrame::DASH, r.next_f32() < 0.025);
        f.set(InputFrame::PARRY, r.next_f32() < 0.06);
        f.set(InputFrame::SUPER, r.next_f32() < 0.012);
        out.push(f);
    }
    out
}

/// Graba el replay dorado desde cero.
pub fn record_golden() -> Replay {
    // El dorado baila el primero. Es el que corre de fondo en el menu.
    record_scripted(
        GOLDEN_SEED,
        Mode::Flight,
        0,
        &golden_inputs(GOLDEN_TICKS),
        DEFAULT_CHECKPOINT_EVERY,
    )
}

/// Lector con comprobacion de limites. Un fichero corrupto da error, no panico.
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn ensure(&self, n: usize) -> Result<(), ReplayError> {
        if self.bytes.len().saturating_sub(self.pos) < n {
            return Err(ReplayError::Truncated);
        }
        Ok(())
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ReplayError> {
        self.ensure(n)?;
        let s = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, ReplayError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ReplayError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, ReplayError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, ReplayError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pcg32;

    /// Inputs plausibles y reproducibles: se mueve, dispara, dashea y parria.
    fn inputs_de_prueba(seed: u64, n: usize) -> Vec<InputFrame> {
        let mut r = Pcg32::new(seed);
        (0..n)
            .map(|_| InputFrame::from_bits((r.next_u32() & 0x1FF) as u16))
            .collect()
    }

    fn replay_de_prueba(ticks: usize) -> Replay {
        record_scripted(
            0x5641_4C53,
            Mode::Flight,
            0,
            &inputs_de_prueba(11, ticks),
            30,
        )
    }

    #[test]
    fn un_replay_recien_grabado_se_verifica() {
        let r = replay_de_prueba(600);
        let rep = r.verify().expect("deberia verificar");
        assert_eq!(rep.ticks, 600);
        assert_eq!(rep.checkpoints, 20, "600 ticks con huella cada 30");
    }

    #[test]
    fn el_formato_binario_va_y_vuelve() {
        let r = replay_de_prueba(300);
        let bytes = r.to_bytes();
        let vuelta = Replay::from_bytes(&bytes).expect("deberia leerse");

        assert_eq!(vuelta.seed, r.seed);
        assert_eq!(vuelta.pattern_hash, r.pattern_hash);
        assert_eq!(vuelta.checkpoint_every, r.checkpoint_every);
        assert_eq!(vuelta.mode, r.mode);
        assert_eq!(vuelta.inputs.len(), r.inputs.len());
        assert_eq!(vuelta.checkpoints, r.checkpoints);
        vuelta.verify().expect("y verificar igual");
    }

    #[test]
    fn el_replay_es_diminuto() {
        let r = replay_de_prueba(3600); // un minuto
        let bytes = r.to_bytes().len();
        assert!(
            bytes < 10_000,
            "un minuto de partida deberia caber de sobra en 10 KB, ocupa {bytes}"
        );
    }

    #[test]
    fn tocar_un_input_produce_divergencia() {
        let mut r = replay_de_prueba(300);
        // Cambiar un solo tick a mitad de la partida.
        r.inputs[150] = InputFrame::from_bits(InputFrame::LEFT | InputFrame::DASH);

        match r.verify() {
            Err(VerifyError::Divergence { tick, .. }) => {
                assert!(tick > 150, "la divergencia se nota despues del cambio");
                assert!(
                    tick <= 180,
                    "y en el primer checkpoint posterior, no mucho mas tarde"
                );
            }
            otro => panic!("deberia haber divergido, dio {otro:?}"),
        }
    }

    #[test]
    fn un_replay_de_otro_patron_se_detecta_como_tal() {
        let mut r = replay_de_prueba(120);
        r.pattern_hash ^= 1;
        assert_eq!(r.verify(), Err(VerifyError::PatternChanged));
    }

    #[test]
    fn un_replay_sin_huellas_no_pasa_por_verificado() {
        let mut r = replay_de_prueba(120);
        r.checkpoints.clear();
        assert_eq!(r.verify(), Err(VerifyError::NoCheckpoints));
    }

    #[test]
    fn un_fichero_que_no_es_un_replay_da_error() {
        assert_eq!(
            Replay::from_bytes(b"esto no es un replay en absoluto"),
            Err(ReplayError::BadMagic)
        );
    }

    #[test]
    fn un_fichero_cortado_da_error_en_vez_de_panic() {
        let bytes = replay_de_prueba(200).to_bytes();
        // Cortar por muchos sitios distintos: ninguno debe entrar en panico.
        for corte in [0, 4, 8, 16, 24, 30, 100, bytes.len() - 1] {
            let r = Replay::from_bytes(&bytes[..corte]);
            assert!(r.is_err(), "un fichero cortado en {corte} deberia fallar");
        }
    }

    #[test]
    fn una_cabecera_que_miente_no_reserva_memoria_absurda() {
        // Cabecera valida, pero declarando cuatro mil millones de inputs.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&0u64.to_le_bytes()); // seed
        bytes.extend_from_slice(&0u64.to_le_bytes()); // pattern_hash
        bytes.extend_from_slice(&60u32.to_le_bytes()); // checkpoint_every
        bytes.push(0); // modo
        bytes.extend_from_slice(&u32::MAX.to_le_bytes()); // n_inputs
        assert_eq!(Replay::from_bytes(&bytes), Err(ReplayError::Truncated));
    }

    #[test]
    fn dos_grabaciones_de_los_mismos_inputs_son_identicas() {
        let inputs = inputs_de_prueba(3, 400);
        let a = record_scripted(7, Mode::Flight, 0, &inputs, 60).to_bytes();
        let b = record_scripted(7, Mode::Flight, 0, &inputs, 60).to_bytes();
        assert_eq!(a, b, "el formato tambien tiene que ser determinista");
    }

    #[test]
    fn los_finales_de_linea_no_cambian_la_huella() {
        let unix = "(\n  name: \"X\",\n)\n";
        let windows = unix.replace('\n', "\r\n");
        assert_eq!(hash_patterns([unix]), hash_patterns([windows.as_str()]));
    }

    #[test]
    fn el_replay_dorado_sigue_valiendo() {
        let r =
            Replay::from_bytes(GOLDEN_REPLAY).expect("el replay dorado del repo deberia leerse");

        match r.verify() {
            Ok(rep) => {
                assert_eq!(rep.ticks, GOLDEN_TICKS as u64);
                assert!(rep.checkpoints >= 29, "deberia haber ~30 huellas");
            }
            Err(VerifyError::PatternChanged) => panic!(
                "ha cambiado assets/patterns/boss1.ron. Si el cambio es \
                 intencionado, vuelve a grabar el dorado:\n  cargo run -p \
                 vals-core --example replay_tool -- record-golden"
            ),
            Err(e) => panic!(
                "la jugabilidad ha cambiado: {e}\nSi el cambio es intencionado, \
                 vuelve a grabar el dorado:\n  cargo run -p vals-core --example \
                 replay_tool -- record-golden"
            ),
        }
    }

    #[test]
    fn un_replay_de_plataformas_se_reproduce_como_tal() {
        let inputs = inputs_de_prueba(5, 300);
        let r = record_scripted(3, Mode::Platform, 0, &inputs, 30);
        assert_eq!(r.mode, Mode::Platform);
        r.verify().expect("deberia verificar en su propio modo");

        // Y el formato conserva el modo al ir y volver.
        let vuelta = Replay::from_bytes(&r.to_bytes()).unwrap();
        assert_eq!(vuelta.mode, Mode::Platform);
        vuelta.verify().expect("y tras pasar por bytes tambien");
    }

    /// Un baile de suelo se graba y se verifica en el suelo, aunque se pida
    /// vuelo: el modo lo decide el baile dentro de `empezar_en`, y `verify`
    /// pasa por el mismo sitio.
    #[test]
    fn un_replay_de_un_baile_de_suelo_se_verifica() {
        let defs = crate::boss::BossDef::default_bosses();
        let baile = defs
            .iter()
            .position(|d| d.suelo)
            .expect("deberia haber algun baile de suelo");
        let inputs = inputs_de_prueba(6, 600);
        let r = record_scripted(8, Mode::Flight, baile, &inputs, 30);
        assert_eq!(r.mode, Mode::Platform, "el fichero dice el modo de verdad");
        r.verify().expect("deberia verificar");
        let vuelta = Replay::from_bytes(&r.to_bytes()).unwrap();
        vuelta.verify().expect("y tras pasar por bytes tambien");
    }

    #[test]
    fn los_dos_modos_no_producen_la_misma_partida() {
        let inputs = inputs_de_prueba(9, 200);
        let vuelo = record_scripted(4, Mode::Flight, 0, &inputs, 30);
        let plataformas = record_scripted(4, Mode::Platform, 0, &inputs, 30);
        assert_ne!(
            vuelo.checkpoints, plataformas.checkpoints,
            "con gravedad la partida tiene que ser otra"
        );
    }

    #[test]
    fn el_replay_dorado_sigue_siendo_util() {
        // Un dorado que muere a los tres segundos no prueba casi nada. Si un
        // reajuste de dificultad lo acorta mucho, conviene enterarse.
        let r = Replay::from_bytes(GOLDEN_REPLAY).unwrap();
        let mut w = World::new(r.seed);
        let mut fin = None;
        for (i, input) in r.inputs.iter().copied().enumerate() {
            w.step(input);
            if w.is_over() && fin.is_none() {
                fin = Some(i as u64);
            }
        }
        let jugados = fin.unwrap_or(r.ticks());
        assert!(
            jugados > r.ticks() / 2,
            "el dorado solo aguanta {jugados} de {} ticks; se ha vuelto poco util",
            r.ticks()
        );
        assert!(w.player.parries > 0, "deberia parriar alguna vez");
        assert!(w.player.grazes > 0, "y rozar balas");
    }

    #[test]
    fn los_inputs_dorados_ejercitan_las_mecanicas() {
        let inputs = golden_inputs(GOLDEN_TICKS);
        let usa = |b: u16| inputs.iter().filter(|i| i.is_down(b)).count();
        assert!(
            usa(InputFrame::SHOOT) > 1700,
            "deberia disparar casi siempre"
        );
        assert!(usa(InputFrame::DASH) > 10, "y dashear de vez en cuando");
        assert!(usa(InputFrame::PARRY) > 30, "y parriar");
        assert!(usa(InputFrame::SUPER) > 5, "y soltar algun super");
        assert!(
            usa(InputFrame::LEFT) + usa(InputFrame::RIGHT) > 500,
            "y moverse"
        );
    }

    #[test]
    fn el_dorado_se_regenera_igual() {
        let a = record_golden().to_bytes();
        let b = record_golden().to_bytes();
        assert_eq!(a, b);
    }

    #[test]
    fn el_recorder_lleva_la_cuenta() {
        let mut world = World::new(1);
        let mut rec = Recorder::new(1, Mode::Flight, 0);
        assert!(rec.is_empty());
        for _ in 0..90 {
            world.step(InputFrame::NONE);
            rec.record(InputFrame::NONE, &world);
        }
        assert_eq!(rec.ticks(), 90);
        let r = rec.finish();
        assert_eq!(r.checkpoints.len(), 1, "una huella por segundo");
        assert_eq!(r.checkpoints[0].tick, 60);
    }
}
