//! El jugador: movimiento, focus, dash y estela.
//!
//! Todo el "feel" del juego vive aqui. Los numeros de este modulo son los que
//! hay que tocar cuando algo se sienta mal, y estan expresados en **ticks**
//! en vez de en segundos justamente para eso: "el dash dura 9 ticks" se razona
//! mucho mejor que "el dash dura 0.15 s" cuando piensas en frame data.

use glam::Vec2;

use crate::{ARENA_H, ARENA_W, DT, InputFrame};

/// Velocidad normal, en unidades logicas por segundo.
pub const PLAYER_SPEED: f32 = 300.0;

/// Velocidad en modo focus.
pub const PLAYER_FOCUS_SPEED: f32 = 130.0;

/// Hitbox real: diminuta, como manda el genero. Esquivar consiste en aprender
/// que casi todo tu personaje es decorativo.
pub const PLAYER_HITBOX_RADIUS: f32 = 2.5;

/// Radio de lo que se dibuja. No colisiona con nada.
pub const PLAYER_SPRITE_RADIUS: f32 = 11.0;

/// Ticks en alcanzar la velocidad plena desde parado.
///
/// Deliberadamente cortisimo. En un danmaku la respuesta instantanea es lo
/// correcto —micro-esquivar con movimiento "con inercia" es horrible— pero un
/// on/off puro se siente robotico y no deja estela decente. Tres ticks (50 ms)
/// es el punto donde deja de parecer un robot sin que se pierda precision.
pub const PLAYER_ACCEL_TICKS: f32 = 3.0;

/// Ticks en pararse del todo. Menos que acelerar: soltar tiene que responder
/// mas rapido que arrancar, o el personaje parece que patina.
pub const PLAYER_DECEL_TICKS: f32 = 2.0;

/// Ticks que el anillo de focus tarda en abrirse o cerrarse.
pub const FOCUS_RAMP_TICKS: f32 = 6.0;

/// Velocidad durante el dash.
pub const DASH_SPEED: f32 = 900.0;

/// Duracion del dash.
pub const DASH_TICKS: u32 = 9;

/// Invulnerabilidad del dash.
///
/// Dura **mas** que el propio dash a proposito. Es un truco de game feel
/// clasico: perdona al jugador que calcule el dash un pelin corto, y hace que
/// atravesar una bala se sienta justo en vez de tacano.
pub const DASH_IFRAME_TICKS: u32 = 12;

/// Enfriamiento desde que empieza un dash hasta que se puede encadenar otro.
pub const DASH_COOLDOWN_TICKS: u32 = 26;

/// Ventana del buffer de input.
///
/// Si pulsas dash mientras aun estas en enfriamiento, se recuerda la pulsacion
/// y sale sola en cuanto se puede. Sin esto el juego "se come" entradas y la
/// culpa parece del jugador cuando en realidad es del programador.
pub const INPUT_BUFFER_TICKS: u32 = 8;

/// Invulnerabilidad al reaparecer tras morir.
///
/// Dos segundos enteros. Es larguisimo comparado con los i-frames del dash, y
/// tiene que serlo: al morir la pantalla se limpia, pero el jefe sigue
/// disparando y reaparecer directamente dentro de un patron seria una muerte
/// que el jugador no puede evitar.
pub const RESPAWN_IFRAME_TICKS: u32 = 120;

/// Ticks entre disparos del jugador.
///
/// Cuatro ticks son 15 disparos por segundo. Lo bastante rapido para que
/// mantener el boton se sienta como una manguera y no como pulsar un boton.
pub const SHOT_EVERY: u32 = 4;

/// Velocidad de los disparos del jugador.
pub const SHOT_SPEED: f32 = 900.0;

/// Dano por bala.
pub const SHOT_DAMAGE: i32 = 1;

/// Separacion entre los dos chorros, a cada lado del personaje.
pub const SHOT_SPREAD: f32 = 9.0;

/// Puntos que guarda la estela.
pub const TRAIL_LEN: usize = 20;

// Invariante de diseno, comprobada al compilar: la invulnerabilidad tiene que
// sobrevivir al final del desplazamiento. Si alguien afina los numeros y se
// carga el margen de perdon, el build falla en vez de degradarse en silencio.
const _: () = assert!(DASH_IFRAME_TICKS > DASH_TICKS);

const ACCEL: f32 = PLAYER_SPEED / (PLAYER_ACCEL_TICKS * DT);
const DECEL: f32 = PLAYER_SPEED / (PLAYER_DECEL_TICKS * DT);

/// Una posicion pasada del jugador.
#[derive(Clone, Copy, Debug, Default)]
pub struct TrailPoint {
    pub pos: Vec2,
    /// Si en ese tick estaba haciendo dash. El render lo usa para pintar la
    /// estela del dash distinta de la del movimiento normal.
    pub dashing: bool,
}

/// Estela del jugador, en un buffer circular de tamano fijo.
///
/// Se muestrea a ritmo de tick y no de frame: asi la estela se ve igual a 60
/// que a 144 Hz. Y al ser un array fijo, no hay ni una allocation.
#[derive(Clone, Debug)]
pub struct Trail {
    points: [TrailPoint; TRAIL_LEN],
    head: usize,
    filled: usize,
}

impl Trail {
    fn new(pos: Vec2) -> Self {
        Self {
            points: [TrailPoint {
                pos,
                dashing: false,
            }; TRAIL_LEN],
            head: 0,
            filled: 0,
        }
    }

    fn push(&mut self, pos: Vec2, dashing: bool) {
        self.points[self.head] = TrailPoint { pos, dashing };
        self.head = (self.head + 1) % TRAIL_LEN;
        self.filled = (self.filled + 1).min(TRAIL_LEN);
    }

    /// Recorre la estela del punto mas reciente al mas antiguo, junto con su
    /// edad normalizada en `[0, 1)`. El render la usa para el desvanecido.
    pub fn iter_newest_first(&self) -> impl Iterator<Item = (TrailPoint, f32)> + '_ {
        (0..self.filled).map(move |i| {
            let idx = (self.head + TRAIL_LEN - 1 - i) % TRAIL_LEN;
            (self.points[idx], i as f32 / TRAIL_LEN as f32)
        })
    }

    pub fn len(&self) -> usize {
        self.filled
    }

    pub fn is_empty(&self) -> bool {
        self.filled == 0
    }
}

/// Estado del dash. Todo son contadores en ticks que bajan solos.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dash {
    /// Ticks que le quedan al dash en curso.
    pub ticks_left: u32,
    /// Ticks hasta poder volver a hacer dash.
    pub cooldown: u32,
    /// Ticks que le quedan de vida a una pulsacion guardada en el buffer.
    pub buffer: u32,
    /// Direccion del dash en curso.
    pub dir: Vec2,
}

/// El jugador.
#[derive(Clone, Debug)]
pub struct Player {
    /// Posicion al final del ultimo tick simulado.
    pub pos: Vec2,
    /// Posicion al final del tick anterior. Existe solo para que el render
    /// interpole; la simulacion no la lee.
    pub prev_pos: Vec2,
    pub vel: Vec2,
    /// Ultima direccion de movimiento no nula. Da direccion al dash cuando se
    /// pulsa sin tocar el stick.
    pub facing: Vec2,
    pub focused: bool,
    /// Transicion del focus en `[0, 1]`. Solo la usa el render.
    pub focus_t: f32,
    /// Ticks de invulnerabilidad restantes.
    ///
    /// Vive en el jugador y no en el dash porque tiene dos fuentes: el dash y
    /// el respawn. Un unico contador evita la pregunta de cual manda cuando
    /// coinciden.
    pub iframes: u32,
    pub deaths: u32,
    /// Ticks hasta el siguiente disparo.
    pub shot_cooldown: u32,
    pub dash: Dash,
    pub trail: Trail,
}

impl Player {
    pub(crate) fn new(pos: Vec2) -> Self {
        Self {
            pos,
            prev_pos: pos,
            vel: Vec2::ZERO,
            facing: Vec2::new(0.0, -1.0),
            focused: false,
            focus_t: 0.0,
            iframes: 0,
            deaths: 0,
            shot_cooldown: 0,
            dash: Dash::default(),
            trail: Trail::new(pos),
        }
    }

    /// Posicion a dibujar, interpolada entre el tick anterior y el actual.
    ///
    /// `alpha` es la fraccion de tick que el bucle principal aun no ha
    /// consumido. Es lo que permite ver 144 fps con la simulacion a 60.
    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev_pos.lerp(self.pos, alpha)
    }

    pub fn is_dashing(&self) -> bool {
        self.dash.ticks_left > 0
    }

    /// Si ahora mismo las balas le atraviesan sin hacerle nada.
    pub fn is_invulnerable(&self) -> bool {
        self.iframes > 0
    }

    /// Mata al jugador y lo devuelve a `respawn` con invulnerabilidad larga.
    ///
    /// Se cancela el dash en curso: reaparecer y salir disparado por inercia
    /// hacia donde ibas antes de morir seria desorientante.
    pub(crate) fn die(&mut self, respawn: Vec2) {
        self.deaths += 1;
        self.pos = respawn;
        self.prev_pos = respawn;
        self.vel = Vec2::ZERO;
        self.dash = Dash::default();
        self.shot_cooldown = 0;
        self.iframes = RESPAWN_IFRAME_TICKS;
        self.trail = Trail::new(respawn);
    }

    /// Avanza al jugador un tick.
    ///
    /// `prev` es el input del tick anterior, y hace falta para detectar
    /// flancos: el dash reacciona a la pulsacion, no a mantener la tecla.
    pub(crate) fn update(&mut self, input: InputFrame, prev: InputFrame) {
        self.prev_pos = self.pos;

        // Los contadores bajan al PRINCIPIO del tick, antes de poder arrancar
        // un dash nuevo. Asi "12 i-frames" significa exactamente 12 ticks de
        // invulnerabilidad y no 11: si se decrementara al final, el tick en el
        // que arranca el dash se comeria uno.
        self.dash.ticks_left = self.dash.ticks_left.saturating_sub(1);
        self.dash.cooldown = self.dash.cooldown.saturating_sub(1);
        self.iframes = self.iframes.saturating_sub(1);
        self.shot_cooldown = self.shot_cooldown.saturating_sub(1);
        self.dash.buffer = self.dash.buffer.saturating_sub(1);

        self.focused = input.is_down(InputFrame::FOCUS);
        let ramp = 1.0 / FOCUS_RAMP_TICKS;
        self.focus_t = if self.focused {
            (self.focus_t + ramp).min(1.0)
        } else {
            (self.focus_t - ramp).max(0.0)
        };

        let (ax, ay) = input.axis();
        // Normalizar es lo que impide que la diagonal sea un 41% mas rapida,
        // que es el bug de movimiento mas viejo del mundo.
        let dir = Vec2::new(ax, ay).normalize_or_zero();
        if dir != Vec2::ZERO {
            self.facing = dir;
        }

        if input.is_down(InputFrame::DASH) && !prev.is_down(InputFrame::DASH) {
            self.dash.buffer = INPUT_BUFFER_TICKS;
        }
        if self.dash.buffer > 0 && self.dash.ticks_left == 0 && self.dash.cooldown == 0 {
            self.dash.buffer = 0;
            self.dash.ticks_left = DASH_TICKS;
            self.dash.cooldown = DASH_COOLDOWN_TICKS;
            // `max`: dashear justo despues de reaparecer no debe recortar la
            // invulnerabilidad larga del respawn.
            self.iframes = self.iframes.max(DASH_IFRAME_TICKS);
            self.dash.dir = if dir != Vec2::ZERO { dir } else { self.facing };
        }

        let dashing = self.is_dashing();
        if dashing {
            // Durante el dash el input no manda: la direccion se fijo al
            // empezar. Poder redirigirlo a media ejecucion convertiria el dash
            // en "volar rapido" y se cargaria el compromiso que lo hace
            // interesante.
            self.vel = self.dash.dir * DASH_SPEED;
        } else {
            let speed = if self.focused {
                PLAYER_FOCUS_SPEED
            } else {
                PLAYER_SPEED
            };
            let rate = if dir == Vec2::ZERO { DECEL } else { ACCEL };
            self.vel = move_towards(self.vel, dir * speed, rate * DT);
        }

        self.pos += self.vel * DT;
        self.clamp_to_arena();

        self.trail.push(self.pos, dashing);
    }

    fn clamp_to_arena(&mut self) {
        let m = PLAYER_SPRITE_RADIUS;
        let clamped_x = self.pos.x.clamp(m, ARENA_W - m);
        let clamped_y = self.pos.y.clamp(m, ARENA_H - m);

        // Al chocar con el borde se mata la velocidad de ese eje. Si no, al
        // separarse de la pared el personaje sale disparado con la velocidad
        // que llevaba acumulada contra ella.
        if clamped_x != self.pos.x {
            self.vel.x = 0.0;
        }
        if clamped_y != self.pos.y {
            self.vel.y = 0.0;
        }
        self.pos = Vec2::new(clamped_x, clamped_y);
    }
}

/// Mueve `current` hacia `target` como mucho `max_delta`, sin pasarse.
fn move_towards(current: Vec2, target: Vec2, max_delta: f32) -> Vec2 {
    let delta = target - current;
    let len = delta.length();
    if len <= max_delta || len == 0.0 {
        target
    } else {
        current + delta / len * max_delta
    }
}
