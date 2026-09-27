//! El paseo: el nivel de correr y disparar que se anda antes de cada jefe.
//!
//! La idea: niveles de correr y disparar, como en Cuphead, **ademas** de los
//! jefes: un paseo por la calle de cada baile, con enemigos de su mundo, que
//! acaba en la puerta del salon donde espera el jefe. Esto es su simulacion.
//!
//! Vive aparte del `World` a proposito. El mundo del jefe es una arena fija de
//! 640 x 800 con un pool de 32k balas que se matan al salirse de ella, y un
//! paseo es una calle de varios miles de unidades que se recorre con la camara.
//! Meter las dos cosas en el mismo sitio obligaba a tocar la simulacion del
//! jefe, y esa tiene un replay dorado que no se toca. Aqui hay
//! pocas cosas vivas a la vez —una docena de enemigos, unos cuantos platos—,
//! asi que bastan unos `Vec` con la capacidad reservada al empezar.
//!
//! Las reglas del core valen igual: ticks fijos, sin reloj ni ficheros, el
//! azar sale de una `Pcg32` sembrada y la trigonometria de `math`. Mismo estado
//! mas mismo input da el mismo paseo, y hay test que lo vigila.
//!
//! El nivel es **datos**: un RON en `assets/paseos/`, igual que los jefes. Los
//! otros tres paseos seran tres ficheros mas.

use glam::Vec2;
use serde::Deserialize;

use crate::events::Events;
use crate::hash::Fnv1a;
use crate::math::{TAU, sin, sin_cos};
use crate::player::{
    AIR_CONTROL, COYOTE_TICKS, DASH_COOLDOWN_TICKS, DASH_IFRAME_TICKS, DASH_SPEED, DASH_TICKS,
    Dash, FOCUS_RAMP_TICKS, GRAVITY, INPUT_BUFFER_TICKS, JUMP_BUFFER_TICKS, JUMP_CUT, JUMP_SPEED,
    PARRY_COOLDOWN_TICKS, PARRY_RADIUS, PARRY_WINDOW_TICKS, PLAYER_ACCEL_TICKS, PLAYER_DECEL_TICKS,
    PLAYER_SPEED, SHOT_EVERY, SHOT_SPEED, mover_hacia,
};
use crate::{DT, InputFrame, Pcg32, Player};

/// Los paseos que trae el juego, embebidos como los jefes.
///
/// Cada uno dice delante de que jefe va (`jefe`), asi que el orden de esta
/// lista no significa nada: un baile sin paseo se entra directo, como antes.
pub const PASEO_RONS: [&str; 2] = [
    include_str!("../../../assets/paseos/viena.ron"),
    include_str!("../../../assets/paseos/chicago.ron"),
];

/// Lo que se ve de la calle, en unidades logicas.
///
/// **Apaisado**, al reves que la arena de los jefes. En un danmaku las balas
/// bajan y el alto es tiempo de reaccion; aqui se corre de lado, y lo que hace
/// falta es ver lo que viene por delante.
pub const VISTA_W: f32 = 960.0;
pub const VISTA_H: f32 = 540.0;

/// Donde pisan los pies en la acera.
pub const SUELO_Y: f32 = 470.0;

/// Del centro del cuerpo a las suelas. `Player::pos` es el centro, como en los
/// combates, asi que el dibujo no tiene que saber que aqui se anda.
pub const PIES: f32 = 30.0;

/// Golpes que aguanta la jugadora: tres, como las vidas de un combate.
pub const VIDAS: u32 = 3;

/// Invulnerabilidad tras un golpe. Menos que los 120 del respawn del jefe: aqui
/// no se limpia la pantalla, pero tampoco hay una pared de balas encima, y un
/// segundo y medio basta para salir de donde te han dado.
pub const INVULNERABLE_TRAS_GOLPE: u32 = 90;

/// El cuerpo con el que se recibe: una capsula vertical, no el punto de 2,5 del
/// danmaku. Aqui no se esquiva al pixel entre balas, se salta por encima de
/// una pareja, y con un punto se atravesarian enemigos enteros.
const CUERPO_RADIO: f32 = 10.0;
/// Media altura de la capsula, sin contar el radio.
const CUERPO_MEDIO: f32 = 16.0;

/// A que distancia por delante de la jugadora despierta un enemigo. Con la
/// camara llevandola al 38 % de la vista, el borde derecho queda a unas 595:
/// despiertan justo fuera de cuadro y entran andando.
const ACTIVAR: f32 = 640.0;
/// A que distancia por detras se olvida. Lo que se queda atras en un
/// run-and-gun no vuelve.
const OLVIDAR: f32 = 900.0;
/// Cuanto por debajo de la acera se da por caida en un foso.
const CAIDA: f32 = 180.0;

/// El alcance de un disparo: lo que vive, a `SHOT_SPEED`. Unas 630 unidades,
/// dos tercios de la vista: dispara a lo que ves, no a lo que aun no ha salido.
const DISPARO_TTL: f32 = 0.7;
const DISPARO_RADIO: f32 = 6.0;
/// Tope de disparos vivos. A 15 por segundo con 0,7 de vida nunca pasan de 11.
const MAX_DISPAROS: usize = 16;

/// Los platos caen mas despacio que la jugadora: un arco que se lee con
/// tiempo, no una pedrada.
const GRAVEDAD_PLATO: f32 = 900.0;
/// Lo que tarda un plato en llegar a donde estabas al tirarlo.
const VUELO_PLATO: f32 = 1.05;
const RADIO_PLATO: f32 = 9.0;
/// Cada cuantos ticks tira un camarero, y hasta donde ve.
pub const CADENCIA_CAMARERO: u32 = 96;
const ALCANCE_CAMARERO: f32 = 620.0;

/// Las perlas de la flapper: cuantas suelta a la vez, repartidas por el
/// collar, y a que velocidad salen. Rectas y sin gravedad: son un collar que
/// gira, no una piedra, y lo que se lee es el giro.
pub const PERLAS_POR_GOLPE: usize = 3;
const VEL_PERLA: f32 = 200.0;
/// Hasta donde ven la flapper y el saxofonista. Menos que el camarero: los
/// dos estan en pantalla antes de empezar, y lo que se esquiva tiene que salir
/// de algo que ya se ha visto.
const ALCANCE_CHICAGO: f32 = 560.0;
/// Las notas del saxo: una cada `SEPARACION_SAXO` ticks dentro de la rafaga.
const SEPARACION_SAXO: u64 = 6;
const VEL_SAXO: f32 = 250.0;
/// Los brincos del saltarin: alto en los dos golpes de tres, bajo en el de
/// dos, y lo que avanza en cada uno. Los ultimos ticks de cada golpe los pasa
/// en el suelo, que es el unico momento en que se le puede saltar por encima.
const BRINCO_LARGO: f32 = 120.0;
const BRINCO_CORTO: f32 = 45.0;
const AVANCE_LARGO: f32 = 150.0;
const AVANCE_CORTO: f32 = 50.0;
pub const EN_EL_SUELO: u64 = 6;

/// Lo que llena el medidor cada disparo que entra. El parry sigue siendo el
/// chorro; esto es el goteo, igual que el graze en los combates.
const METER_POR_IMPACTO: f32 = 0.8;
/// Lo que quita el super a cada enemigo en pantalla.
const DANO_SUPER: i32 = 12;

/// Un tramo de acera, de `.0` a `.1`. Entre dos tramos hay un foso.
pub type Tramo = (f32, f32);

/// Una mesa de cafe o un balcon. Se atraviesa desde abajo y se pisa desde
/// arriba, que es lo que se espera de una plataforma en un run-and-gun: saltar
/// y no darse con la cabeza.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Plataforma {
    /// Borde izquierdo.
    pub x: f32,
    /// La superficie, donde se pisa.
    pub y: f32,
    pub ancho: f32,
}

/// Los enemigos de los paseos. Los tres primeros son de Viena, y cada uno pide
/// una cosa distinta: saltar, ponerse a tiro y apuntar arriba. Cada ciudad
/// trae los suyos con la misma regla.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum Tipo {
    /// Una pareja que baila por la acera hacia ti, girando: paso largo en el
    /// uno del compas y dos cortos. Se salta o se tumba.
    Pareja,
    /// Un camarero plantado que tira platos en arco a donde estas. Uno de cada
    /// tres es rosa y se puede parriar.
    Camarero,
    /// Una nota que cruza volando en onda. Las rosas se parrian.
    Nota,
    // --- Chicago. El charleston gira y va en clave 3-3-2, y
    // sus tres enemigos tambien.
    /// Una flapper plantada, bailando el charleston en el sitio, con el collar
    /// de perlas dando vueltas como un lazo. En cada golpe de la clave suelta
    /// tres perlas por donde va el collar: una espiral que se lee mirandola.
    /// En el golpe de dos, una es rosa. Pide el parry.
    Flapper,
    /// Un saxofonista en una escalera de incendios, por encima de la calle.
    /// Sopla rafagas de tres, tres y dos notas apuntadas a ti: se esquiva
    /// andando y se le tumba plantada, apuntando arriba.
    Saxo,
    /// Un dandi de canotier que viene brincando el charleston: dos brincos
    /// altos y uno bajo por compas. Por debajo de los altos se pasa corriendo;
    /// el bajo se salta o se tumba.
    Saltarin,
}

impl Tipo {
    /// Vida. Una nota cae de un par de disparos; un camarero aguanta medio
    /// segundo de fuego, que es lo que hace que merezca la pena esquivarle.
    fn vida(self) -> i32 {
        match self {
            Tipo::Pareja => 6,
            Tipo::Camarero => 8,
            Tipo::Nota => 2,
            Tipo::Flapper => 8,
            Tipo::Saxo => 10,
            Tipo::Saltarin => 7,
        }
    }

    /// Radio de su cuerpo, para los disparos y para el contacto.
    pub fn radio(self) -> f32 {
        match self {
            Tipo::Pareja => 24.0,
            Tipo::Camarero => 22.0,
            Tipo::Nota => 14.0,
            Tipo::Flapper | Tipo::Saxo | Tipo::Saltarin => 22.0,
        }
    }

    /// Cuanto por encima de donde pisa esta su centro.
    fn alto(self) -> f32 {
        match self {
            Tipo::Pareja => 38.0,
            Tipo::Camarero => 36.0,
            Tipo::Nota => 0.0,
            Tipo::Flapper => 38.0,
            Tipo::Saxo | Tipo::Saltarin => 36.0,
        }
    }

    fn como_u8(self) -> u8 {
        match self {
            Tipo::Pareja => 0,
            Tipo::Camarero => 1,
            Tipo::Nota => 2,
            Tipo::Flapper => 3,
            Tipo::Saxo => 4,
            Tipo::Saltarin => 5,
        }
    }
}

/// La clave 3-3-2 del charleston, contada en corcheas: golpes en la 0, la 3 y
/// la 6 de cada compas de ocho, igual que en el jefe (`boss3.ron`). Devuelve
/// que golpe suena (0, 1 o 2), cuantos ticks lleva sonando y cuanto dura.
///
/// Va con el tick del paseo y no con la edad de cada enemigo: la musica es una
/// para toda la calle, y el dibujo late con el mismo tick.
pub fn clave(tick: u64, tiempo: u32) -> (usize, u64, u64) {
    let corchea = u64::from((tiempo / 2).max(1));
    let fase = tick % (8 * corchea);
    let (golpe, desde, largo) = if fase < 3 * corchea {
        (0, 0, 3)
    } else if fase < 6 * corchea {
        (1, 3, 3)
    } else {
        (2, 6, 2)
    };
    (golpe, fase - desde * corchea, largo * corchea)
}

/// Por donde va el collar de la flapper en el tick `t`. Siete decimos de vuelta
/// por compas: asi un golpe no cae nunca donde el del compas anterior y la
/// espiral avanza. El dibujo usa esta misma cuenta, asi que las perlas salen
/// de donde se ve el collar.
pub fn giro_collar(t: f32, tiempo: u32) -> f32 {
    TAU * 0.7 * t / (4 * tiempo.max(1)) as f32
}

/// Que es lo que vuela. El dibujo lo necesita, y la fisica tambien: un plato
/// cae y una perla no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forma {
    Plato,
    /// Del collar de la flapper.
    Perla,
    /// Del saxo.
    Corchea,
}

impl Forma {
    fn radio(self) -> f32 {
        match self {
            Forma::Plato => RADIO_PLATO,
            Forma::Perla => 7.0,
            Forma::Corchea => 8.0,
        }
    }
}

/// Donde sale un enemigo.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Aparicion {
    pub tipo: Tipo,
    pub x: f32,
    /// La altura de vuelo de una nota, o la superficie donde esta de pie un
    /// camarero. Cero o sin poner es la acera.
    #[serde(default)]
    pub y: f32,
    /// Solo en las notas: si se puede parriar.
    #[serde(default)]
    pub rosa: bool,
}

/// Un paseo, tal como viene del RON.
#[derive(Clone, Debug, Deserialize)]
pub struct PaseoDef {
    pub nombre: String,
    /// El jefe al que lleva, por nombre, igual que el guardado.
    pub jefe: String,
    /// La linea de debajo del nombre en la cartela de entrada.
    pub subtitulo: String,
    /// Ticks por tiempo de su musica. Las parejas bailan con esto.
    pub tiempo: u32,
    pub largo: f32,
    pub salida: f32,
    /// La puerta del salon: llegar aqui acaba el paseo.
    pub puerta: f32,
    pub suelo: Vec<Tramo>,
    pub plataformas: Vec<Plataforma>,
    pub enemigos: Vec<Aparicion>,
}

impl PaseoDef {
    pub fn from_ron(src: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(src.trim_start_matches('\u{feff}'))
    }

    /// Los paseos embebidos.
    ///
    /// # Panics
    ///
    /// Si alguno no parsea. Estan embebidos, asi que un fallo aqui es un bug
    /// del build y no algo que pueda pasar jugando; lo vigila un test.
    pub fn de_serie() -> Vec<Self> {
        PASEO_RONS
            .iter()
            .map(|r| Self::from_ron(r).expect("paseo embebido invalido"))
            .collect()
    }

    /// Si hay acera debajo de `x`.
    pub fn hay_suelo(&self, x: f32) -> bool {
        self.suelo.iter().any(|&(a, b)| x >= a && x <= b)
    }

    /// Si en `x` hay algo que pisar a la altura `y`: la acera o una
    /// plataforma. Es lo que mira quien anda por una azotea para no tirarse.
    pub fn hay_pie(&self, x: f32, y: f32) -> bool {
        (y >= SUELO_Y - 0.5 && self.hay_suelo(x))
            || self
                .plataformas
                .iter()
                .any(|pl| (pl.y - y).abs() < 0.5 && x >= pl.x && x <= pl.x + pl.ancho)
    }
}

/// Un enemigo despierto.
#[derive(Clone, Debug)]
pub struct Enemigo {
    pub tipo: Tipo,
    pub pos: Vec2,
    pub vida: i32,
    /// Hacia donde mira: -1 o 1.
    pub dir: f32,
    /// Tick en que desperto. Su baile va contado desde aqui, asi que dos
    /// parejas iguales no se mueven en bloque.
    pub nacio: u64,
    pub rosa: bool,
    /// Ticks de parpadeo tras recibir un disparo. Solo lo lee el dibujo.
    pub golpe: u32,
    /// Ticks que le faltan para tirar (camarero). El dibujo lo usa para
    /// echar el brazo atras justo antes.
    pub reloj: u32,
    /// Platos tirados (camarero), para saber cual toca rosa.
    lanzados: u32,
    /// Altura de vuelo (nota), o donde esta su centro con los pies en el
    /// suelo (saltarin, que vuelve ahi de cada brinco).
    base_y: f32,
    /// Lo que avanza por segundo en el brinco en curso (saltarin). Se decide
    /// al despegar, que es cuando mira si al otro lado hay donde caer.
    avance: f32,
}

/// Algo que le tiran a la jugadora: los platos del camarero.
#[derive(Clone, Copy, Debug)]
pub struct Proyectil {
    pub pos: Vec2,
    pub vel: Vec2,
    pub rosa: bool,
    pub forma: Forma,
}

/// Un disparo de la jugadora.
#[derive(Clone, Copy, Debug)]
pub struct Disparo {
    pub pos: Vec2,
    pub vel: Vec2,
    ttl: f32,
}

/// Todo el estado de un paseo.
#[derive(Clone, Debug)]
pub struct Paseo {
    pub def: PaseoDef,
    pub tick: u64,
    /// La misma `Player` de los combates, para que el dibujo —la pose, la
    /// falda, el parpadeo de los i-frames— valga sin cambiar nada. Lo que no se
    /// reusa es su `update`, que esta atado a la arena de 640 y a un suelo
    /// plano (ver `mover`).
    pub jugadora: Player,
    pub vidas: u32,
    /// Hacia donde dispara: de frente, o arriba si se planta y apunta.
    pub apunta: Vec2,
    pub enemigos: Vec<Enemigo>,
    pub proyectiles: Vec<Proyectil>,
    pub disparos: Vec<Disparo>,
    /// Tick en que se llego a la puerta.
    pub completado: Option<u64>,
    pub derrota: bool,
    /// Lo que ha pasado en el ultimo tick. Es el mismo `Events` de los
    /// combates, asi que el sonido y el zumo lo entienden sin traducir nada:
    /// `boss_hit` es "un disparo ha dado a un enemigo", `player_died` "te han
    /// dado" y `victory` "has llegado a la puerta".
    ///
    /// `boss_down` no se usa: en el combate es el final de un jefe, con la
    /// sacudida y el frenazo mas gordos del juego, y aqui caen veinte enemigos
    /// por calle. Para eso esta `caidos`.
    pub events: Events,
    /// Enemigos que han caido en el ultimo tick. Como `events`, no es estado.
    pub caidos: u32,
    /// Donde fue el ultimo impacto o la ultima caida de un enemigo, para las
    /// chispas. Como `events`, no es estado: no entra en la huella.
    pub impacto: Vec2,
    /// La siguiente aparicion por despertar. Las apariciones van ordenadas por
    /// x, asi que basta un indice.
    siguiente: usize,
    /// El ultimo sitio de acera pisado, de donde se vuelve tras caer a un foso.
    seguro: Vec2,
    prev_input: InputFrame,
    rng: Pcg32,
}

impl Paseo {
    pub fn new(mut def: PaseoDef, seed: u64) -> Self {
        // Ordenadas por x: es lo que deja despertarlas con un solo indice. Con
        // `total_cmp` para que un NaN en el RON no cambie el orden segun la
        // plataforma, y estable para que dos en la misma x salgan como se
        // escribieron.
        def.enemigos.sort_by(|a, b| a.x.total_cmp(&b.x));
        let salida = Vec2::new(def.salida, SUELO_Y - PIES);
        let mut jugadora = Player::new(salida);
        jugadora.facing = Vec2::new(1.0, 0.0);
        jugadora.on_ground = true;
        Self {
            def,
            tick: 0,
            jugadora,
            vidas: VIDAS,
            apunta: Vec2::new(1.0, 0.0),
            enemigos: Vec::with_capacity(32),
            proyectiles: Vec::with_capacity(32),
            disparos: Vec::with_capacity(MAX_DISPAROS),
            completado: None,
            derrota: false,
            events: Events::default(),
            caidos: 0,
            impacto: salida,
            siguiente: 0,
            seguro: salida,
            prev_input: InputFrame::NONE,
            rng: Pcg32::new(seed),
        }
    }

    /// Rehace el nivel con otra definicion, con la jugadora donde estaba.
    ///
    /// Es el hot-reload: al afinar el tercer tramo no se quiere volver a
    /// andar los dos primeros cada vez que se guarda. Lo que queda por detras
    /// no se despierta.
    pub fn recargar(&mut self, def: PaseoDef) {
        let (pos, vidas) = (self.jugadora.pos, self.vidas);
        let seguro = self.seguro;
        *self = Self::new(def, self.rng.next_u32().into());
        self.jugadora.pos = pos;
        self.jugadora.prev_pos = pos;
        self.vidas = vidas;
        self.seguro = seguro;
        while self
            .def
            .enemigos
            .get(self.siguiente)
            .is_some_and(|a| a.x < pos.x)
        {
            self.siguiente += 1;
        }
    }

    /// Si ya no se juega: llegaste a la puerta o te quedaste sin vidas.
    pub fn terminado(&self) -> bool {
        self.completado.is_some() || self.derrota
    }

    /// Cuanto del camino a la puerta llevas, de 0 a 1.
    pub fn progreso(&self) -> f32 {
        let total = (self.def.puerta - self.def.salida).max(1.0);
        ((self.jugadora.pos.x - self.def.salida) / total).clamp(0.0, 1.0)
    }

    /// Da el paseo por andado. La trampa de F3, igual que saltarse un baile:
    /// ver como avanza el juego no deberia costar pasarse la calle cada vez.
    pub fn saltar(&mut self) {
        if self.terminado() {
            return;
        }
        let p = Vec2::new(self.def.puerta, SUELO_Y - PIES);
        self.jugadora.pos = p;
        self.jugadora.prev_pos = p;
        self.llegar();
    }

    /// Avanza un tick. Mismo estado mas mismo input, mismo resultado.
    pub fn step(&mut self, input: InputFrame) {
        self.events = Events::default();
        self.caidos = 0;
        if self.terminado() {
            // Se queda quieta en la puerta o donde cayo; el resto del mundo
            // tambien, que la escena ya ha acabado.
            let p = &mut self.jugadora;
            p.prev_pos = p.pos;
            p.vel = Vec2::ZERO;
            self.tick += 1;
            return;
        }

        self.mover(input);
        self.caida();
        self.disparar(input);
        self.parry();
        self.super_(input);
        self.despertar();
        self.enemigos_actuan();
        self.mover_proyectiles();
        self.mover_disparos();
        self.recibir();
        self.olvidar();
        if self.jugadora.pos.x >= self.def.puerta {
            self.llegar();
        }

        self.prev_input = input;
        self.tick += 1;
    }

    fn llegar(&mut self) {
        self.completado = Some(self.tick);
        self.events.victory = true;
        // Lo que venia volando ya no llega: se acabo la calle.
        self.proyectiles.clear();
    }

    /// La fisica de la jugadora en la calle.
    ///
    /// Es la de `Player::update_gravity` —los mismos numeros de `player.rs`:
    /// gravedad, salto variable, coyote, buffer, dash— con dos cambios que no
    /// caben alli sin tocar el combate: el suelo tiene fosos y plataformas, y
    /// no hay paredes a 640. Copiar la fisica y no reusarla tiene un precio
    /// (afinar el salto en un sitio no lo afina en el otro), pero la
    /// alternativa era meter geometria en la simulacion del jefe, que tiene un
    /// replay dorado detras. Los numeros son los mismos porque son los mismos
    /// `const`.
    fn mover(&mut self, input: InputFrame) {
        const ACCEL: f32 = PLAYER_SPEED / (PLAYER_ACCEL_TICKS * DT);
        const DECEL: f32 = PLAYER_SPEED / (PLAYER_DECEL_TICKS * DT);
        let prev = self.prev_input;
        let flanco = |b: u16| input.is_down(b) && !prev.is_down(b);
        let p = &mut self.jugadora;
        p.prev_pos = p.pos;

        // Los contadores bajan al principio, como en `Player::update`.
        p.dash.ticks_left = p.dash.ticks_left.saturating_sub(1);
        p.dash.cooldown = p.dash.cooldown.saturating_sub(1);
        p.dash.buffer = p.dash.buffer.saturating_sub(1);
        p.iframes = p.iframes.saturating_sub(1);
        p.shot_cooldown = p.shot_cooldown.saturating_sub(1);
        p.parry_window = p.parry_window.saturating_sub(1);
        p.parry_cooldown = p.parry_cooldown.saturating_sub(1);
        p.super_ticks = p.super_ticks.saturating_sub(1);
        p.coyote = p.coyote.saturating_sub(1);
        p.jump_buffer = p.jump_buffer.saturating_sub(1);

        // FOCUS es **plantarse**, el boton de fijar de Cuphead: quieta en el
        // suelo y apuntando con la cruceta. Hace falta un boton aparte porque
        // arriba ya es saltar, y apuntar arriba sin plantarse haria saltar a
        // la vez. Plantada, arriba apunta y no salta.
        let plantada = input.is_down(InputFrame::FOCUS);
        p.focused = plantada;
        let rampa = 1.0 / FOCUS_RAMP_TICKS;
        p.focus_t = if plantada {
            (p.focus_t + rampa).min(1.0)
        } else {
            (p.focus_t - rampa).max(0.0)
        };

        let (ax, ay) = input.axis();
        if ax != 0.0 {
            p.facing = Vec2::new(ax, 0.0);
        }
        self.apunta = if plantada && ay < 0.0 {
            Vec2::new(ax, -1.0).normalize()
        } else {
            Vec2::new(p.facing.x, 0.0)
        };

        if flanco(InputFrame::PARRY) && p.parry_cooldown == 0 {
            p.parry_window = PARRY_WINDOW_TICKS;
            p.parry_cooldown = PARRY_COOLDOWN_TICKS;
        }
        if flanco(InputFrame::DASH) {
            p.dash.buffer = INPUT_BUFFER_TICKS;
        }
        if p.dash.buffer > 0 && p.dash.ticks_left == 0 && p.dash.cooldown == 0 {
            p.dash.buffer = 0;
            p.dash.ticks_left = DASH_TICKS;
            p.dash.cooldown = DASH_COOLDOWN_TICKS;
            p.iframes = p.iframes.max(DASH_IFRAME_TICKS);
            p.dash.dir = Vec2::new(p.facing.x, 0.0);
        }
        if flanco(InputFrame::JUMP) && !plantada {
            p.jump_buffer = JUMP_BUFFER_TICKS;
        }

        if p.is_dashing() {
            // Horizontal y sin gravedad: cruzar un foso con dash es la jugada.
            p.vel = Vec2::new(p.dash.dir.x * DASH_SPEED, 0.0);
        } else {
            let quieta = plantada && p.on_ground;
            let objetivo = if quieta { 0.0 } else { ax * PLAYER_SPEED };
            let rate = if ax == 0.0 || quieta {
                DECEL
            } else if p.on_ground {
                ACCEL
            } else {
                ACCEL * AIR_CONTROL
            };
            p.vel.x = mover_hacia(p.vel.x, objetivo, rate * DT);

            if p.jump_buffer > 0 && (p.on_ground || p.coyote > 0) {
                p.vel.y = -JUMP_SPEED;
                p.jump_buffer = 0;
                p.coyote = 0;
                p.on_ground = false;
            }
            let g = if p.vel.y < 0.0 && !input.is_down(InputFrame::JUMP) {
                GRAVITY * JUMP_CUT
            } else {
                GRAVITY
            };
            // Con tope de caida: en un foso hondo la velocidad creceria sin
            // limite y un tick se saltaria una plataforma entera.
            p.vel.y = (p.vel.y + g * DT).min(JUMP_SPEED * 1.6);
        }

        // Primero en x, con los fosos haciendo de pared: con los pies por
        // debajo de la acera, un tramo de acera es un muro. El muro se mide
        // con el centro y no con el ancho del cuerpo: con el ancho, al bajar
        // del borde andando, el cuerpo aun solapa la acera y el muro la
        // empujaba de golpe hacia el foso.
        const PARED: f32 = 2.0;
        let def = &self.def;
        p.pos.x = (p.pos.x + p.vel.x * DT).clamp(CUERPO_RADIO, def.largo - CUERPO_RADIO);
        if p.pos.y + PIES > SUELO_Y + 1.0 {
            for &(a, b) in &def.suelo {
                if p.pos.x + PARED > a && p.pos.x - PARED < b {
                    p.pos.x = if p.prev_pos.x <= a {
                        a - PARED
                    } else {
                        b + PARED
                    };
                    p.vel.x = 0.0;
                }
            }
        }

        // Luego en y. Solo se aterriza bajando y cruzando la superficie en este
        // tick: es lo que deja atravesar las plataformas desde abajo.
        let pies_antes = p.prev_pos.y + PIES;
        p.pos.y += p.vel.y * DT;
        p.on_ground = false;
        if p.vel.y >= 0.0 {
            let pies = p.pos.y + PIES;
            let cruza = |y: f32| pies_antes <= y + 0.5 && pies >= y;
            let x = p.pos.x;
            let mut pisa = None;
            if cruza(SUELO_Y) && def.hay_suelo(x) {
                pisa = Some(SUELO_Y);
            }
            for pl in &def.plataformas {
                if cruza(pl.y) && x >= pl.x && x <= pl.x + pl.ancho {
                    // Si hay dos, la de mas arriba: es la que se ha cruzado
                    // primero.
                    pisa = Some(pisa.map_or(pl.y, |y: f32| y.min(pl.y)));
                }
            }
            if let Some(y) = pisa {
                p.pos.y = y - PIES;
                p.vel.y = 0.0;
                p.on_ground = true;
                p.coyote = COYOTE_TICKS;
                if y == SUELO_Y
                    && let Some(&(a, b)) = def.suelo.iter().find(|&&(a, b)| x >= a && x <= b)
                {
                    // El sitio seguro, metido hacia dentro del tramo: volver
                    // al borde exacto del foso seria volver a caerse.
                    let margen = 50.0_f32.min((b - a) * 0.5);
                    self.seguro = Vec2::new(x.clamp(a + margen, b - margen), SUELO_Y - PIES);
                } else if !def.hay_suelo(x)
                    && let Some(pl) = def
                        .plataformas
                        .iter()
                        .find(|pl| pl.y == y && x >= pl.x && x <= pl.x + pl.ancho)
                {
                    // Sobre el vacio, la plataforma es el suelo: en las azoteas
                    // de Chicago no hay acera en toda la manzana, y caerse entre
                    // dos tejados no puede devolverte al pie de la escalera de
                    // incendios. Se vuelve al ultimo tejado pisado, tambien
                    // metido hacia dentro.
                    let margen = 40.0_f32.min(pl.ancho * 0.5);
                    self.seguro =
                        Vec2::new(x.clamp(pl.x + margen, pl.x + pl.ancho - margen), y - PIES);
                }
            }
        }
    }

    /// Caer a un foso cuesta un golpe y te devuelve a la ultima acera pisada.
    fn caida(&mut self) {
        if self.jugadora.pos.y + PIES < SUELO_Y + CAIDA {
            return;
        }
        let p = &mut self.jugadora;
        p.pos = self.seguro;
        p.prev_pos = self.seguro;
        p.vel = Vec2::ZERO;
        p.on_ground = true;
        self.golpe();
    }

    fn golpe(&mut self) {
        self.vidas = self.vidas.saturating_sub(1);
        self.events.player_died = true;
        let p = &mut self.jugadora;
        p.iframes = INVULNERABLE_TRAS_GOLPE;
        p.dash = Dash::default();
        if self.vidas == 0 {
            self.derrota = true;
            self.events.defeat = true;
        }
    }

    fn disparar(&mut self, input: InputFrame) {
        let p = &mut self.jugadora;
        if !input.is_down(InputFrame::SHOOT)
            || p.shot_cooldown > 0
            || self.disparos.len() >= MAX_DISPAROS
        {
            return;
        }
        p.shot_cooldown = SHOT_EVERY;
        self.events.player_shot = true;
        // Sale de la mano, a media altura: de frente pasa por encima de una
        // nota baja y le da a una pareja en el pecho.
        self.disparos.push(Disparo {
            pos: self.boca(),
            vel: self.apunta * SHOT_SPEED,
            ttl: DISPARO_TTL,
        });
    }

    /// De donde salen los disparos. El dibujo la usa para el fogonazo.
    pub fn boca(&self) -> Vec2 {
        self.jugadora.pos + Vec2::new(0.0, -6.0) + self.apunta * 18.0
    }

    /// El parry de los combates, contra lo rosa de la calle: los platos rosas
    /// y las notas rosas. Si se acierta en el aire, rebota, que es el gesto del
    /// parry de Cuphead y lo que deja encadenar dos.
    fn parry(&mut self) {
        let p = &mut self.jugadora;
        if !p.is_parrying() {
            return;
        }
        let centro = p.pos;
        let mut n = 0u32;
        self.proyectiles.retain(|q| {
            let dentro = q.rosa && q.pos.distance_squared(centro) <= PARRY_RADIUS * PARRY_RADIUS;
            n += u32::from(dentro);
            !dentro
        });
        for e in &mut self.enemigos {
            let r = PARRY_RADIUS + e.tipo.radio();
            if e.rosa && e.vida > 0 && e.pos.distance_squared(centro) <= r * r {
                e.vida = 0;
                n += 1;
            }
        }
        if n == 0 {
            return;
        }
        p.on_parry(n);
        if !p.on_ground {
            p.vel.y = -JUMP_SPEED * 0.8;
        }
        self.events.parried += n;
        self.impacto = centro;
        self.retirar_caidos();
    }

    /// El super, igual que en los combates: medidor lleno, todo o nada. Aqui
    /// limpia lo que vuela y pega a todo lo que se ve.
    fn super_(&mut self, input: InputFrame) {
        let pulsado =
            input.is_down(InputFrame::SUPER) && !self.prev_input.is_down(InputFrame::SUPER);
        if !pulsado || !self.jugadora.spend_meter() {
            return;
        }
        self.events.super_fired = true;
        self.proyectiles.clear();
        let x = self.jugadora.pos.x;
        for e in &mut self.enemigos {
            if (e.pos.x - x).abs() < VISTA_W * 0.6 {
                e.vida -= DANO_SUPER;
                e.golpe = 8;
            }
        }
        self.retirar_caidos();
    }

    /// Despierta lo que tiene la jugadora a tiro de vista.
    fn despertar(&mut self) {
        let frente = self.jugadora.pos.x + ACTIVAR;
        while let Some(a) = self.def.enemigos.get(self.siguiente).copied() {
            if a.x > frente {
                break;
            }
            self.siguiente += 1;
            let pisa = if a.y > 0.0 { a.y } else { SUELO_Y };
            let y = match a.tipo {
                Tipo::Nota => a.y,
                t => pisa - t.alto(),
            };
            // El primer plato sale con un poco de azar: dos camareros que se
            // despiertan juntos no deben tirar a la vez.
            let reloj = CADENCIA_CAMARERO / 2 + self.rng.next_u32() % 30;
            self.enemigos.push(Enemigo {
                tipo: a.tipo,
                pos: Vec2::new(a.x, y),
                vida: a.tipo.vida(),
                dir: -1.0,
                nacio: self.tick,
                rosa: a.rosa && a.tipo == Tipo::Nota,
                golpe: 0,
                reloj,
                lanzados: 0,
                base_y: y,
                avance: 0.0,
            });
        }
    }

    fn enemigos_actuan(&mut self) {
        let objetivo = self.jugadora.pos;
        let tiempo = u64::from(self.def.tiempo.max(1));
        for e in &mut self.enemigos {
            e.golpe = e.golpe.saturating_sub(1);
            let edad = self.tick - e.nacio;
            match e.tipo {
                Tipo::Pareja => {
                    // Un vals: en el uno del compas se vuelve hacia ti y da el
                    // paso largo; en el dos y el tres, pasitos. Es el tres por
                    // cuatro hecho andar, y se lee: sabes cuando va a acelerar.
                    let fase = edad % (3 * tiempo);
                    if fase == 0 && objetivo.x != e.pos.x {
                        e.dir = (objetivo.x - e.pos.x).signum();
                    }
                    let v = if fase < tiempo { 170.0 } else { 40.0 };
                    let nx = e.pos.x + e.dir * v * DT;
                    // No se tira al canal: en el borde de la acera da la vuelta.
                    if self.def.hay_suelo(nx + e.dir * e.tipo.radio()) {
                        e.pos.x = nx;
                    } else {
                        e.dir = -e.dir;
                    }
                }
                Tipo::Camarero => {
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    if dx.abs() > ALCANCE_CAMARERO {
                        continue;
                    }
                    e.reloj = e.reloj.saturating_sub(1);
                    if e.reloj > 0 {
                        continue;
                    }
                    e.reloj = CADENCIA_CAMARERO;
                    e.lanzados += 1;
                    // El arco se calcula para caer donde estabas al tirarlo, en
                    // un tiempo fijo. Si te mueves, falla: se esquiva andando,
                    // que es lo que se le pide a un plato.
                    let origen = e.pos + Vec2::new(e.dir * 14.0, -26.0);
                    let t = VUELO_PLATO;
                    let vx = ((objetivo.x - origen.x) / t).clamp(-460.0, 460.0);
                    let dy = SUELO_Y - 6.0 - origen.y;
                    let vy = (dy - 0.5 * GRAVEDAD_PLATO * t * t) / t;
                    self.proyectiles.push(Proyectil {
                        pos: origen,
                        vel: Vec2::new(vx, vy),
                        rosa: e.lanzados % 3 == 0,
                        forma: Forma::Plato,
                    });
                }
                Tipo::Nota => {
                    // Hacia la izquierda en onda: un periodo cada cuatro
                    // tiempos, asi que tambien van con la musica.
                    e.pos.x -= 140.0 * DT;
                    let t = edad as f32 / (4 * tiempo) as f32;
                    e.pos.y = e.base_y + 50.0 * sin(TAU * t);
                    e.dir = -1.0;
                }
                Tipo::Flapper => {
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    let (golpe, dentro, _) = clave(self.tick, self.def.tiempo);
                    if dentro != 0 || dx.abs() > ALCANCE_CHICAGO {
                        continue;
                    }
                    // El collar gira alrededor del cuello, y en el golpe se
                    // sueltan las perlas por donde pasa. Tres, repartidas: una
                    // espiral que avanza a saltos de tres, tres y dos.
                    let giro = giro_collar(self.tick as f32, self.def.tiempo);
                    let cuello = e.pos + Vec2::new(0.0, -12.0);
                    for k in 0..PERLAS_POR_GOLPE {
                        let a = giro + TAU * k as f32 / PERLAS_POR_GOLPE as f32;
                        let (s, c) = sin_cos(a);
                        let dir = Vec2::new(c, s);
                        self.proyectiles.push(Proyectil {
                            pos: cuello + dir * 24.0,
                            vel: dir * VEL_PERLA,
                            rosa: golpe == 2 && k == 0,
                            forma: Forma::Perla,
                        });
                    }
                    e.lanzados += 1;
                }
                Tipo::Saxo => {
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    if dx.abs() > ALCANCE_CHICAGO {
                        continue;
                    }
                    // Tres, tres y dos notas: una rafaga por golpe de clave,
                    // cada nota apuntada a donde estas al soplarla. Se esquiva
                    // andando, igual que el plato.
                    let (golpe, dentro, _) = clave(self.tick, self.def.tiempo);
                    let notas = if golpe == 2 { 2 } else { 3 };
                    if dentro % SEPARACION_SAXO != 0 || dentro / SEPARACION_SAXO >= notas {
                        continue;
                    }
                    let boca = e.pos + Vec2::new(e.dir * 20.0, 6.0);
                    let dir = (objetivo - boca).normalize_or(Vec2::new(e.dir, 0.0));
                    // La ultima del compas, rosa: una por compas.
                    let rosa = golpe == 2 && dentro / SEPARACION_SAXO == notas - 1;
                    self.proyectiles.push(Proyectil {
                        pos: boca,
                        vel: dir * VEL_SAXO,
                        rosa,
                        forma: Forma::Corchea,
                    });
                    e.lanzados += 1;
                }
                Tipo::Saltarin => {
                    let (golpe, dentro, largo) = clave(self.tick, self.def.tiempo);
                    let alto = if golpe == 2 {
                        BRINCO_CORTO
                    } else {
                        BRINCO_LARGO
                    };
                    let vuelo = largo.saturating_sub(EN_EL_SUELO).max(1);
                    if dentro == 0 {
                        // Al despegar se vuelve hacia ti y mira si al otro
                        // lado hay donde caer. Si no, brinca en el sitio: no se
                        // tira de una azotea, igual que la pareja no se tira al
                        // canal.
                        if objetivo.x != e.pos.x {
                            e.dir = (objetivo.x - e.pos.x).signum();
                        }
                        let paso = if golpe == 2 {
                            AVANCE_CORTO
                        } else {
                            AVANCE_LARGO
                        };
                        let pies = e.base_y + e.tipo.alto();
                        let cae = e.pos.x + e.dir * (paso + e.tipo.radio());
                        e.avance = if self.def.hay_pie(cae, pies) {
                            e.dir * paso / (vuelo as f32 * DT)
                        } else {
                            0.0
                        };
                    }
                    if dentro < vuelo {
                        // Una parabola exacta en funcion del tiempo, no una
                        // integracion: toca el suelo justo al acabar el vuelo.
                        let f = dentro as f32 / vuelo as f32;
                        e.pos.y = e.base_y - 4.0 * alto * f * (1.0 - f);
                        e.pos.x += e.avance * DT;
                    } else {
                        e.pos.y = e.base_y;
                    }
                }
            }
        }
    }

    fn mover_proyectiles(&mut self) {
        for q in &mut self.proyectiles {
            if q.forma == Forma::Plato {
                q.vel.y += GRAVEDAD_PLATO * DT;
            }
            q.pos += q.vel * DT;
        }
        // Un plato que llega a la acera se rompe. Sobre un foso sigue cayendo
        // hasta perderse, que no hace dano a nadie. Lo que vuela recto se
        // rompe igual contra la acera, y sobre el vacio se va al salir de la
        // vista; por los lados lo quita `olvidar`.
        let def = &self.def;
        self.proyectiles.retain(|q| match q.forma {
            Forma::Plato => q.pos.y < SUELO_Y,
            _ => {
                let en_acera = q.pos.y >= SUELO_Y && def.hay_suelo(q.pos.x);
                q.pos.y > -40.0 && q.pos.y < VISTA_H + 40.0 && !en_acera
            }
        });
    }

    fn mover_disparos(&mut self) {
        for d in &mut self.disparos {
            d.pos += d.vel * DT;
            d.ttl -= DT;
        }
        self.disparos.retain(|d| d.ttl > 0.0);

        let mut i = 0;
        while i < self.disparos.len() {
            let d = self.disparos[i];
            let blanco = self.enemigos.iter_mut().find(|e| {
                let r = e.tipo.radio() + DISPARO_RADIO;
                e.vida > 0 && e.pos.distance_squared(d.pos) <= r * r
            });
            if let Some(e) = blanco {
                e.vida -= 1;
                e.golpe = 6;
                self.events.boss_hit = true;
                self.impacto = d.pos;
                self.jugadora.add_meter(METER_POR_IMPACTO);
                self.disparos.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.retirar_caidos();
    }

    /// Quita a los enemigos sin vida, avisando de cada caida.
    fn retirar_caidos(&mut self) {
        let mut caido = None;
        let mut n = 0;
        self.enemigos.retain(|e| {
            if e.vida <= 0 {
                caido = Some(e.pos);
                n += 1;
            }
            e.vida > 0
        });
        self.caidos += n;
        if let Some(pos) = caido {
            self.impacto = pos;
        }
    }

    /// Si la capsula de la jugadora toca un circulo.
    fn toca(&self, c: Vec2, r: f32) -> bool {
        let p = self.jugadora.pos;
        let y = c.y.clamp(p.y - CUERPO_MEDIO, p.y + CUERPO_MEDIO);
        let rr = r + CUERPO_RADIO;
        Vec2::new(p.x, y).distance_squared(c) <= rr * rr
    }

    /// Los golpes que se llevan: platos y contacto con enemigos. Uno por tick
    /// como mucho, y ninguno durante los i-frames.
    fn recibir(&mut self) {
        if self.jugadora.is_invulnerable() {
            return;
        }
        let plato = self
            .proyectiles
            .iter()
            .position(|q| self.toca(q.pos, q.forma.radio()));
        let tocado = plato.is_some()
            || self
                .enemigos
                .iter()
                .any(|e| self.toca(e.pos, e.tipo.radio() * 0.8));
        if let Some(i) = plato {
            self.proyectiles.swap_remove(i);
        }
        if tocado {
            // Un empujon hacia atras y arriba: que se vea que te han dado y
            // que te saque de encima de la pareja que te ha pisado.
            let p = &mut self.jugadora;
            p.vel = Vec2::new(-p.facing.x * 160.0, -420.0);
            p.on_ground = false;
            self.golpe();
        }
    }

    /// Lo que se ha quedado muy atras se va.
    fn olvidar(&mut self) {
        let x = self.jugadora.pos.x;
        self.enemigos.retain(|e| e.pos.x > x - OLVIDAR);
        self.proyectiles.retain(|q| (q.pos.x - x).abs() < OLVIDAR);
    }

    /// Huella del estado, para el test de determinismo. Como la del mundo,
    /// deja fuera lo derivado: la estela, los eventos y el punto de impacto.
    pub fn huella(&self) -> u64 {
        let mut h = Fnv1a::new();
        h.write_u64(self.tick);
        let (s, i) = self.rng.raw_state();
        h.write_u64(s);
        h.write_u64(i);
        h.write_u64(u64::from(self.prev_input.bits()));
        let p = &self.jugadora;
        h.write_vec2(p.pos);
        h.write_vec2(p.vel);
        h.write_vec2(p.facing);
        h.write_u64(u64::from(p.iframes));
        h.write_u64(u64::from(p.dash.ticks_left));
        h.write_u64(u64::from(p.dash.cooldown));
        h.write_u64(u64::from(p.on_ground));
        h.write_u64(u64::from(p.coyote));
        h.write_u64(u64::from(p.jump_buffer));
        h.write_u64(u64::from(p.parry_window));
        h.write_f32(p.meter);
        h.write_u64(u64::from(self.vidas));
        h.write_vec2(self.apunta);
        h.write_vec2(self.seguro);
        h.write_u64(self.siguiente as u64);
        for e in &self.enemigos {
            h.write_u64(u64::from(e.tipo.como_u8()));
            h.write_vec2(e.pos);
            h.write_u64(e.vida as u64);
            h.write_f32(e.dir);
            h.write_u64(u64::from(e.reloj));
            h.write_f32(e.avance);
        }
        for q in &self.proyectiles {
            h.write_vec2(q.pos);
            h.write_vec2(q.vel);
            h.write_u64(u64::from(q.rosa));
            h.write_u64(q.forma as u64);
        }
        for d in &self.disparos {
            h.write_vec2(d.pos);
            h.write_f32(d.ttl);
        }
        h.write_u64(self.completado.unwrap_or(u64::MAX));
        h.write_u64(u64::from(self.derrota));
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DERECHA: u16 = InputFrame::RIGHT;
    const DISPARA: u16 = InputFrame::SHOOT;

    fn pulsar(bits: u16) -> InputFrame {
        InputFrame::from_bits(bits)
    }

    /// Una calle de pruebas: acera de 0 a 3000, sin fosos ni nadie.
    fn calle() -> PaseoDef {
        PaseoDef {
            nombre: "Prueba".into(),
            jefe: "El Vals".into(),
            subtitulo: String::new(),
            tiempo: 21,
            largo: 3200.0,
            salida: 100.0,
            puerta: 3000.0,
            suelo: vec![(0.0, 3200.0)],
            plataformas: vec![],
            enemigos: vec![],
        }
    }

    fn con(enemigos: Vec<Aparicion>) -> PaseoDef {
        PaseoDef {
            enemigos,
            ..calle()
        }
    }

    fn correr(p: &mut Paseo, bits: u16, n: usize) {
        for _ in 0..n {
            p.step(pulsar(bits));
        }
    }

    /// El paseo de serie que va antes de `jefe`.
    fn de_serie(jefe: &str) -> PaseoDef {
        PaseoDef::de_serie()
            .into_iter()
            .find(|d| d.jefe == jefe)
            .unwrap_or_else(|| panic!("no hay paseo antes de {jefe}"))
    }

    #[test]
    fn los_paseos_de_serie_cargan_y_tienen_sentido() {
        let defs = PaseoDef::de_serie();
        assert_eq!(defs.len(), PASEO_RONS.len());
        for d in &defs {
            assert!(
                d.hay_suelo(d.salida),
                "{}: se empieza pisando acera",
                d.nombre
            );
            assert!(d.hay_suelo(d.puerta), "{}: la puerta en la acera", d.nombre);
            assert!(d.puerta < d.largo);
            // Los tramos en orden y sin pisarse: si se solapasen, un foso
            // podria quedar tapado sin que el dibujo lo supiera.
            for par in d.suelo.windows(2) {
                assert!(par[0].1 < par[1].0, "{}: tramos desordenados", d.nombre);
            }
            assert!(d.suelo.len() >= 2, "{}: sin fosos no hay paseo", d.nombre);
            assert!(
                d.enemigos.iter().any(|a| a.rosa),
                "{}: sin nada rosa no hay parry en la calle",
                d.nombre
            );
            // Dos paseos para el mismo jefe: el segundo no se andaria nunca.
            let mismos = defs.iter().filter(|o| o.jefe == d.jefe).count();
            assert_eq!(mismos, 1, "dos paseos antes de {}", d.jefe);
        }

        let viena = de_serie("El Vals");
        assert!(viena.suelo.len() >= 3, "hace falta algun foso que saltar");
        for t in [Tipo::Pareja, Tipo::Camarero, Tipo::Nota] {
            assert!(
                viena.enemigos.iter().any(|a| a.tipo == t),
                "falta {t:?} en Viena"
            );
        }

        // Chicago se cruza por arriba: tiene que haber un tramo largo sin
        // calle, y la clave del charleston en su `tiempo`.
        let chicago = de_serie("El Charleston");
        let vacio = chicago
            .suelo
            .windows(2)
            .map(|par| par[1].0 - par[0].1)
            .fold(0.0, f32::max);
        assert!(vacio > 2000.0, "Chicago no tiene tejados: {vacio}");
        assert_eq!(chicago.tiempo, 36, "el Maple Leaf va a 100");
        for t in [Tipo::Flapper, Tipo::Saxo, Tipo::Saltarin, Tipo::Nota] {
            assert!(
                chicago.enemigos.iter().any(|a| a.tipo == t),
                "falta {t:?} en Chicago"
            );
        }
        // Cada flapper, saxo y saltarin, de pie sobre algo: uno puesto en el
        // aire flotaria, y un saltarin en el aire no sabria donde caer.
        for a in &chicago.enemigos {
            if matches!(a.tipo, Tipo::Flapper | Tipo::Saxo | Tipo::Saltarin) {
                let pisa = if a.y > 0.0 { a.y } else { SUELO_Y };
                assert!(chicago.hay_pie(a.x, pisa), "{a:?} flota");
            }
        }
    }

    #[test]
    fn un_ron_roto_se_rechaza() {
        assert!(PaseoDef::from_ron("(nombre: \"a\",").is_err());
    }

    #[test]
    fn la_jugadora_aterriza_en_la_acera_y_se_queda() {
        let mut p = Paseo::new(calle(), 1);
        // Se la suelta desde arriba para que tenga que caer.
        p.jugadora.pos.y -= 120.0;
        p.jugadora.on_ground = false;
        correr(&mut p, 0, 90);
        assert!(p.jugadora.on_ground);
        assert_eq!(p.jugadora.pos.y + PIES, SUELO_Y, "los pies en la acera");
        correr(&mut p, DERECHA, 60);
        assert!(p.jugadora.on_ground, "andar no la hunde");
        assert_eq!(p.jugadora.pos.y + PIES, SUELO_Y);
    }

    #[test]
    fn saltar_sube_y_vuelve_a_la_acera() {
        let mut p = Paseo::new(calle(), 1);
        p.step(pulsar(InputFrame::JUMP));
        correr(&mut p, InputFrame::JUMP, 15);
        assert!(p.jugadora.pos.y + PIES < SUELO_Y - 100.0, "no ha subido");
        correr(&mut p, 0, 60);
        assert!(p.jugadora.on_ground);
    }

    #[test]
    fn las_plataformas_se_atraviesan_por_debajo_y_se_pisan_por_encima() {
        let mut def = calle();
        def.plataformas = vec![Plataforma {
            x: 50.0,
            y: 400.0,
            ancho: 100.0,
        }];
        let mut p = Paseo::new(def, 1);
        p.step(pulsar(InputFrame::JUMP));
        correr(&mut p, InputFrame::JUMP, 60);
        assert!(p.jugadora.on_ground);
        assert_eq!(p.jugadora.pos.y + PIES, 400.0, "tenia que acabar encima");
    }

    #[test]
    fn caer_a_un_foso_cuesta_una_vida_y_devuelve_a_la_acera() {
        let mut def = calle();
        def.suelo = vec![(0.0, 400.0), (700.0, 3200.0)];
        let mut p = Paseo::new(def, 1);
        let mut caida = false;
        for _ in 0..240 {
            p.step(pulsar(DERECHA));
            caida |= p.events.player_died;
            if caida {
                break;
            }
        }
        assert!(caida, "tenia que caerse al foso");
        assert_eq!(p.vidas, VIDAS - 1);
        assert!(p.def.hay_suelo(p.jugadora.pos.x), "vuelve a la acera");
        assert!(p.jugadora.pos.x < 400.0, "a la de antes del foso");
        assert!(p.jugadora.is_invulnerable(), "y con margen para respirar");
    }

    #[test]
    fn un_foso_es_una_pared_desde_dentro() {
        // Con los pies por debajo de la acera no se puede andar a traves de un
        // tramo de suelo como si fuera aire.
        let mut def = calle();
        def.suelo = vec![(0.0, 400.0), (600.0, 3200.0)];
        let mut p = Paseo::new(def, 1);
        p.jugadora.pos = Vec2::new(500.0, SUELO_Y + 40.0);
        p.jugadora.on_ground = false;
        for _ in 0..8 {
            p.step(pulsar(DERECHA));
            assert!(p.jugadora.pos.x < 600.0, "ha atravesado la acera");
        }
    }

    #[test]
    fn los_disparos_van_de_frente_y_tumban_a_una_pareja() {
        let mut p = Paseo::new(
            con(vec![Aparicion {
                tipo: Tipo::Pareja,
                x: 450.0,
                y: 0.0,
                rosa: false,
            }]),
            1,
        );
        p.step(pulsar(DISPARA));
        assert!(p.events.player_shot);
        assert!(p.disparos[0].vel.x > 0.0 && p.disparos[0].vel.y == 0.0);
        let mut cayo = false;
        for _ in 0..90 {
            p.step(pulsar(DISPARA));
            cayo |= p.caidos > 0;
        }
        assert!(cayo, "la pareja tenia que caer");
        assert!(p.enemigos.is_empty());
        assert_eq!(p.vidas, VIDAS, "sin dejarla llegar");
    }

    #[test]
    fn plantada_apunta_arriba_y_no_salta() {
        let mut p = Paseo::new(calle(), 1);
        let arriba = InputFrame::FOCUS | InputFrame::UP | InputFrame::JUMP | DISPARA;
        correr(&mut p, arriba, 10);
        assert!(p.jugadora.on_ground, "plantada no salta");
        assert_eq!(p.apunta, Vec2::new(0.0, -1.0));
        assert!(p.disparos.iter().all(|d| d.vel.y < 0.0 && d.vel.x == 0.0));
    }

    #[test]
    fn tras_un_golpe_hay_invulnerabilidad() {
        // Una pareja que viene bailando hacia ella, y la jugadora quieta.
        let mut p = Paseo::new(
            con(vec![Aparicion {
                tipo: Tipo::Pareja,
                x: 220.0,
                y: 0.0,
                rosa: false,
            }]),
            1,
        );
        let mut n = 0;
        while !p.events.player_died {
            p.step(InputFrame::NONE);
            n += 1;
            assert!(n < 600, "la pareja no ha llegado nunca");
        }
        assert_eq!(p.vidas, VIDAS - 1);
        // Mientras dura, la pareja puede pasarle por encima sin hacer nada.
        for _ in 1..INVULNERABLE_TRAS_GOLPE {
            p.step(InputFrame::NONE);
            assert!(!p.events.player_died, "golpe durante la invulnerabilidad");
        }
        // Y se acaba cuando dice: al siguiente tick ya se puede recibir (y si
        // la pareja sigue encima, recibe).
        p.step(InputFrame::NONE);
        assert!(
            !p.jugadora.is_invulnerable() || p.events.player_died,
            "la invulnerabilidad dura de mas"
        );
    }

    #[test]
    fn sin_vidas_se_acaba() {
        let mut p = Paseo::new(calle(), 1);
        for _ in 0..VIDAS {
            p.jugadora.iframes = 0;
            p.golpe();
        }
        assert!(p.derrota && p.terminado());
        assert!(p.events.defeat);
    }

    #[test]
    fn parriar_un_plato_rosa_lo_quita_y_llena_el_medidor() {
        let mut p = Paseo::new(calle(), 1);
        let pos = p.jugadora.pos + Vec2::new(20.0, -10.0);
        p.proyectiles.push(Proyectil {
            pos,
            vel: Vec2::ZERO,
            rosa: true,
            forma: Forma::Plato,
        });
        p.step(pulsar(InputFrame::PARRY));
        assert_eq!(p.events.parried, 1);
        assert!(p.proyectiles.is_empty());
        assert!(p.jugadora.meter > 0.0);
        assert_eq!(p.vidas, VIDAS, "parriado no hace dano");
    }

    #[test]
    fn el_camarero_tira_platos_en_arco_y_uno_de_cada_tres_es_rosa() {
        let mut p = Paseo::new(
            con(vec![Aparicion {
                tipo: Tipo::Camarero,
                x: 600.0,
                y: 0.0,
                rosa: false,
            }]),
            1,
        );
        // Invulnerable, para contar platos sin que la maten.
        let mut rosas = 0;
        let mut vistos = 0;
        let mut antes = 0;
        for _ in 0..(CADENCIA_CAMARERO * 7) {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            let ahora = p.enemigos[0].lanzados;
            if ahora > antes {
                vistos += 1;
                let q = p.proyectiles.last().expect("acaba de tirar");
                assert!(q.vel.y < 0.0, "un plato sale hacia arriba");
                rosas += u32::from(q.rosa);
                antes = ahora;
            }
        }
        assert!(vistos >= 6, "tira con cadencia: {vistos}");
        assert_eq!(rosas, vistos / 3);
    }

    #[test]
    fn llegar_a_la_puerta_completa_el_paseo() {
        let mut p = Paseo::new(calle(), 1);
        let mut llego = false;
        for _ in 0..(60 * 20) {
            p.step(pulsar(DERECHA));
            llego |= p.events.victory;
        }
        assert!(llego && p.completado.is_some());
        assert_eq!(p.progreso(), 1.0);
        // Y ya no se mueve: la escena ha acabado.
        let x = p.jugadora.pos.x;
        correr(&mut p, DERECHA, 30);
        assert_eq!(p.jugadora.pos.x, x);
    }

    #[test]
    fn el_mismo_input_da_el_mismo_paseo() {
        // Un minuto largo de cada paseo con input sembrado: dos paseos iguales
        // tienen que ir tick a tick a la par, enemigos incluidos.
        for def in PaseoDef::de_serie() {
            mismo_input_mismo_paseo(def);
        }
    }

    fn mismo_input_mismo_paseo(def: PaseoDef) {
        let mut r = Pcg32::new(7);
        let inputs: Vec<InputFrame> = (0..4000)
            .map(|_| {
                // Mucha derecha y disparo, para que se ande y pase de todo.
                let azar = (r.next_u32() & 0x3FF) as u16;
                pulsar(
                    azar | if r.next_f32() < 0.8 {
                        DERECHA | DISPARA
                    } else {
                        0
                    },
                )
            })
            .collect();
        let mut a = Paseo::new(def.clone(), 0x5641_4C53);
        let mut b = Paseo::new(def.clone(), 0x5641_4C53);
        for (i, input) in inputs.iter().enumerate() {
            a.step(*input);
            b.step(*input);
            assert_eq!(a.huella(), b.huella(), "{}: divergencia en {i}", def.nombre);
        }
        assert!(
            a.jugadora.pos.x > def.salida + 500.0,
            "el input no ha andado"
        );

        // Y la huella ve lo que pasa: otro input, otro paseo.
        let mut c = Paseo::new(def, 0x5641_4C53);
        c.step(pulsar(InputFrame::LEFT));
        let mut d = Paseo::new(c.def.clone(), 0x5641_4C53);
        d.step(pulsar(DERECHA));
        assert_ne!(c.huella(), d.huella());
    }

    #[test]
    fn la_clave_va_tres_tres_dos() {
        // A 36 ticks por tiempo la corchea son 18: golpes en 0, 54 y 108.
        let golpes: Vec<u64> = (0..288).filter(|&t| clave(t, 36).1 == 0).collect();
        assert_eq!(golpes, [0, 54, 108, 144, 198, 252]);
        assert_eq!(clave(110, 36), (2, 2, 36));
    }

    fn uno(tipo: Tipo, x: f32, y: f32) -> PaseoDef {
        con(vec![Aparicion {
            tipo,
            x,
            y,
            rosa: false,
        }])
    }

    #[test]
    fn la_flapper_suelta_perlas_en_la_clave_y_el_collar_gira() {
        let mut def = uno(Tipo::Flapper, 500.0, 0.0);
        def.tiempo = 36;
        let mut p = Paseo::new(def, 1);
        let mut angulos = Vec::new();
        let mut rosas = 0;
        for _ in 0..(144 * 3) {
            p.jugadora.iframes = 10;
            let tick = p.tick;
            let antes = p.proyectiles.len();
            p.step(InputFrame::NONE);
            let nuevas = &p.proyectiles[antes.min(p.proyectiles.len())..];
            if nuevas.is_empty() {
                continue;
            }
            // Solo en los golpes de la clave, y de tres en tres.
            assert_eq!(clave(tick, 36).1, 0, "perlas fuera de la clave en {tick}");
            assert_eq!(nuevas.len(), PERLAS_POR_GOLPE);
            assert!(nuevas.iter().all(|q| q.forma == Forma::Perla));
            rosas += nuevas.iter().filter(|q| q.rosa).count();
            let v = nuevas[0].vel;
            angulos.push(crate::math::atan2(v.y, v.x));
        }
        assert_eq!(angulos.len(), 9, "tres golpes por compas");
        assert_eq!(rosas, 3, "una rosa por compas, en el golpe de dos");
        // El collar gira: dos golpes seguidos nunca salen por el mismo sitio.
        for par in angulos.windows(2) {
            assert!((par[0] - par[1]).abs() > 0.2, "el collar no gira: {par:?}");
        }
        // Y las perlas vuelan rectas: sin gravedad, la velocidad no cambia.
        let v = p.proyectiles[0].vel;
        p.step(InputFrame::NONE);
        assert_eq!(p.proyectiles[0].vel, v);
    }

    #[test]
    fn el_saxo_sopla_rafagas_de_tres_tres_dos_apuntadas() {
        // En una escalera por encima de la jugadora.
        let mut def = uno(Tipo::Saxo, 400.0, 330.0);
        def.tiempo = 36;
        let mut p = Paseo::new(def, 1);
        let mut rafagas: Vec<usize> = Vec::new();
        let mut rosas = 0;
        for _ in 0..(144 * 2) {
            p.jugadora.iframes = 10;
            let (_, dentro, _) = clave(p.tick, 36);
            let antes = p.enemigos.first().map_or(0, |e| e.lanzados);
            p.step(InputFrame::NONE);
            if p.enemigos[0].lanzados == antes {
                continue;
            }
            let q = *p.proyectiles.last().expect("acaba de soplar");
            // Apuntada a ella: hacia abajo y hacia la izquierda.
            let a_ella = (p.jugadora.pos - q.pos).normalize();
            assert!(q.vel.normalize().dot(a_ella) > 0.98, "no apunta");
            assert!(q.vel.y > 0.0 && q.forma == Forma::Corchea);
            rosas += usize::from(q.rosa);
            if dentro == 0 {
                rafagas.push(0);
            }
            *rafagas.last_mut().expect("rafaga empezada") += 1;
        }
        assert_eq!(rafagas, [3, 3, 2, 3, 3, 2]);
        assert_eq!(rosas, 2, "una rosa por compas");
        // Y despierto pero con la jugadora fuera de su alcance, calla.
        let lejos = p.def.salida + ALCANCE_CHICAGO + 40.0;
        let mut p = Paseo::new(uno(Tipo::Saxo, lejos, 330.0), 1);
        correr(&mut p, 0, 144);
        assert_eq!(p.enemigos.len(), 1, "tenia que estar despierto");
        assert!(p.proyectiles.is_empty(), "sopla sin nadie delante");
    }

    #[test]
    fn el_saltarin_brinca_alto_alto_bajo_y_no_se_tira() {
        // En una azotea corta, sin acera debajo, y la jugadora lejos a su
        // izquierda: tiene que venir brincando y pararse en el borde.
        let mut def = uno(Tipo::Saltarin, 700.0, 300.0);
        def.tiempo = 36;
        def.suelo = vec![(0.0, 200.0), (900.0, 3200.0)];
        def.plataformas = vec![Plataforma {
            x: 300.0,
            y: 300.0,
            ancho: 500.0,
        }];
        let mut p = Paseo::new(def, 1);
        p.jugadora.iframes = 10;
        let pie = 300.0 - Tipo::Saltarin.alto();
        let mut cumbres = [0.0_f32; 3];
        let mut xs = Vec::new();
        for _ in 0..(144 * 4) {
            p.jugadora.iframes = 10;
            let (golpe, _, _) = clave(p.tick, 36);
            p.step(InputFrame::NONE);
            let e = &p.enemigos[0];
            cumbres[golpe] = cumbres[golpe].max(pie - e.pos.y);
            xs.push(e.pos.x);
        }
        // Dos brincos por encima de ella y uno que hay que saltar.
        let cabeza = PIES + CUERPO_MEDIO + CUERPO_RADIO;
        assert!(cumbres[0] > 100.0 && cumbres[1] > 100.0, "{cumbres:?}");
        assert!(
            cumbres[2] < cabeza,
            "el bajo se pasaba por debajo: {cumbres:?}"
        );
        // Ha venido hacia ella, y sin salirse de su tejado.
        let e = &p.enemigos[0];
        assert!(xs.iter().all(|&x| (300.0..=800.0).contains(&x)), "se tiro");
        assert!(e.pos.x < 420.0, "no ha venido: {}", e.pos.x);
        assert_eq!(e.pos.y, pie, "acaba con los pies en el tejado");
    }

    #[test]
    fn caer_desde_una_azotea_devuelve_a_la_azotea() {
        // Dos tejados sin calle debajo. Se cae entre los dos: tiene que volver
        // al primero, no a la acera de la salida.
        let mut def = calle();
        def.suelo = vec![(0.0, 300.0), (2000.0, 3200.0)];
        def.plataformas = vec![
            Plataforma {
                x: 300.0,
                y: 470.0,
                ancho: 400.0,
            },
            Plataforma {
                x: 1000.0,
                y: 470.0,
                ancho: 400.0,
            },
        ];
        let mut p = Paseo::new(def, 1);
        let mut caida = false;
        for _ in 0..240 {
            p.step(pulsar(DERECHA));
            caida |= p.events.player_died;
            if caida {
                break;
            }
        }
        assert!(caida, "tenia que caerse entre los tejados");
        let x = p.jugadora.pos.x;
        assert!((340.0..=660.0).contains(&x), "vuelve al tejado: {x}");
        assert_eq!(p.jugadora.pos.y + PIES, 470.0);
    }

    /// Un bot invulnerable que corre y dispara, y salta cuando tiene un hueco
    /// delante o algo que subir. Como en Viena, no prueba que sea facil: prueba
    /// que se puede. En Chicago eso incluye subir la escalera de incendios.
    fn bot_anda(def: PaseoDef) -> Paseo {
        let mut p = Paseo::new(def, 3);
        let mut salto = 0;
        for _ in 0..(60 * 90) {
            p.jugadora.iframes = 5;
            let j = &p.jugadora;
            let (x, pies) = (j.pos.x, j.pos.y + PIES);
            // Hueco: nada que pisar un poco por delante, ni a la altura de los
            // pies ni en los 200 de debajo.
            let hay_debajo = |dx: f32| {
                (pies <= SUELO_Y && p.def.hay_suelo(x + dx) && SUELO_Y - pies < 200.0)
                    || p.def.plataformas.iter().any(|pl| {
                        pl.y >= pies - 0.5
                            && pl.y - pies < 200.0
                            && x + dx >= pl.x
                            && x + dx <= pl.x + pl.ancho
                    })
            };
            // Algo que subir: una plataforma por encima, a tiro de salto, y
            // donde se caeria corriendo (unas 130 por delante) dentro de ella.
            let subir = p.def.plataformas.iter().any(|pl| {
                let cae = x + 130.0;
                pies - pl.y > 20.0
                    && pies - pl.y < 140.0
                    && cae > pl.x + 10.0
                    && cae < pl.x + pl.ancho - 10.0
            });
            if j.on_ground && (!hay_debajo(30.0) || subir) {
                salto = 22;
            }
            let bits = DERECHA | DISPARA | if salto > 0 { InputFrame::JUMP } else { 0 };
            salto = if salto > 0 { salto - 1 } else { 0 };
            p.step(pulsar(bits));
            if p.completado.is_some() {
                break;
            }
        }
        p
    }

    #[test]
    fn chicago_se_puede_andar_entera() {
        let p = bot_anda(de_serie("El Charleston"));
        assert!(p.completado.is_some(), "atascada en x {}", p.jugadora.pos.x);
        assert_eq!(p.vidas, VIDAS, "no se ha caido de ningun tejado");
    }

    #[test]
    fn viena_se_puede_andar_entera() {
        // Un bot que corre, dispara y salta cuando tiene un foso delante. Es
        // invulnerable, asi que no prueba que sea facil: prueba que ningun foso
        // es mas ancho que un salto y que no hay nada que tape la puerta.
        let def = de_serie("El Vals");
        let mut p = Paseo::new(def, 3);
        let mut salto = 0;
        for _ in 0..(60 * 60) {
            p.jugadora.iframes = 5;
            let x = p.jugadora.pos.x;
            let foso_delante = p.jugadora.on_ground && !p.def.hay_suelo(x + 24.0);
            if foso_delante {
                salto = 20;
            }
            let bits = DERECHA | DISPARA | if salto > 0 { InputFrame::JUMP } else { 0 };
            salto = if salto > 0 { salto - 1 } else { 0 };
            p.step(pulsar(bits));
            if p.completado.is_some() {
                break;
            }
        }
        assert!(p.completado.is_some(), "atascada en x {}", p.jugadora.pos.x);
        assert_eq!(p.vidas, VIDAS, "no ha caido a ningun foso");
    }
}
