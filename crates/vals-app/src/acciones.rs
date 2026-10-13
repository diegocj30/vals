//! Las acciones: la unica tabla entre las teclas y botones y lo que hacen.
//!
//! Antes cada escena miraba teclas y botones a su manera: el circulo era
//! "reintentar" en el KO y "salir" en la tienda, el triangulo era ESC fuera del
//! combate, y la ayuda de cada pantalla decidia por su cuenta si hablaba de
//! teclas o de botones. El resultado era una ayuda que decia "Z" con el mando
//! en la mano y una R sin equivalente en el mando.
//!
//! Aqui hay dos clases de acciones, y la diferencia importa:
//!
//! - **Las de menu** (`Accion`): aceptar, atras, pausa... Son fijas, como en
//!   cualquier juego: el boton de abajo acepta aunque se haya movido el salto,
//!   y asi nunca te quedas sin poder salir de un menu.
//! - **Las de juego** (`Juego`): disparar, saltar, dash, parry, super, focus.
//!   Son las que se pueden cambiar (`Controles`), y las que llegan a la
//!   simulacion como `InputFrame`. Moverse no se cambia: flechas y WASD, stick
//!   y cruceta.
//!
//! Y se recuerda **con que se jugo lo ultimo** (`Dispositivo`): toda ayuda en
//! pantalla sale de `Nombres`, que habla del teclado o del mando segun eso.

use macroquad::prelude::{KeyCode, get_last_key_pressed, is_key_down, is_key_pressed};
use vals_core::InputFrame;

use crate::mando::{self, Mando, boton};

/// Con que se esta jugando.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dispositivo {
    Teclado,
    Mando,
}

/// Con que se jugo lo ultimo, visto lo que ha pasado este frame.
///
/// Gana el ultimo que se toco. Enchufar un mando cuenta como tocarlo —es lo
/// que se espera al conectarlo con el juego abierto—, y desenchufarlo devuelve
/// la ayuda al teclado, que es lo unico que queda.
pub fn ultimo_usado(antes: Dispositivo, tecla: bool, boton: bool, conectado: bool) -> Dispositivo {
    if boton {
        Dispositivo::Mando
    } else if tecla || !conectado {
        Dispositivo::Teclado
    } else {
        antes
    }
}

/// Lo que se hace en los menus y entre partidas. Fijo, no se cambia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accion {
    Aceptar,
    Atras,
    /// La R de toda la vida, y Select en el mando.
    Reintentar,
    /// ESC y Start: en un baile o un paseo abre el entreacto; en el titulo,
    /// las opciones.
    Pausa,
    Arriba,
    Abajo,
    Izquierda,
    Derecha,
    /// Volver a andar un paseo ya andado, desde la pista.
    Pasear,
    /// Abrir y cerrar el libro de baile, desde el titulo.
    Libro,
    /// Volver a ver el prologo, desde el titulo. Es el mismo boton secundario
    /// que pasear en la pista.
    Historia,
}

impl Accion {
    /// Las teclas. La primera es la que se nombra en pantalla.
    pub fn teclas(self) -> &'static [KeyCode] {
        use KeyCode::*;
        match self {
            Accion::Aceptar => &[Z, Enter, Space],
            Accion::Atras => &[Escape, Backspace],
            Accion::Reintentar => &[R],
            Accion::Pausa => &[Escape, P],
            Accion::Arriba => &[Up, W],
            Accion::Abajo => &[Down, S],
            Accion::Izquierda => &[Left, A],
            Accion::Derecha => &[Right, D],
            Accion::Pasear | Accion::Historia => &[X],
            Accion::Libro => &[L],
        }
    }

    /// Los botones crudos del mando.
    pub fn botones(self) -> u32 {
        match self {
            Accion::Aceptar => boton::SUR,
            Accion::Atras => boton::ESTE,
            Accion::Reintentar => boton::SELECT,
            Accion::Pausa => boton::START,
            Accion::Arriba => boton::ARRIBA,
            Accion::Abajo => boton::ABAJO,
            Accion::Izquierda => boton::IZQUIERDA,
            Accion::Derecha => boton::DERECHA,
            Accion::Pasear | Accion::Historia => boton::OESTE,
            // El gatillo de focus, que en el titulo no hace nada.
            Accion::Libro => boton::L1,
        }
    }
}

/// Lo que se hace bailando. Esto si se cambia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Juego {
    Disparar,
    Saltar,
    Dash,
    Parry,
    Super,
    Focus,
}

impl Juego {
    pub const TODAS: [Juego; 6] = [
        Juego::Disparar,
        Juego::Saltar,
        Juego::Dash,
        Juego::Parry,
        Juego::Super,
        Juego::Focus,
    ];

    /// El bit que le llega a la simulacion.
    pub fn bit(self) -> u16 {
        match self {
            Juego::Disparar => InputFrame::SHOOT,
            Juego::Saltar => InputFrame::JUMP,
            Juego::Dash => InputFrame::DASH,
            Juego::Parry => InputFrame::PARRY,
            Juego::Super => InputFrame::SUPER,
            Juego::Focus => InputFrame::FOCUS,
        }
    }

    /// Como se llama en el guardado. No se traduce nunca: es formato, no
    /// texto.
    fn clave(self) -> &'static str {
        match self {
            Juego::Disparar => "disparar",
            Juego::Saltar => "saltar",
            Juego::Dash => "dash",
            Juego::Parry => "parry",
            Juego::Super => "super",
            Juego::Focus => "focus",
        }
    }

    /// Como se llama en pantalla.
    pub fn nombre(self) -> &'static str {
        TEXTOS.juego[self as usize]
    }
}

/// Los textos de este modulo, juntos para traducirlos de una vez.
struct Textos {
    juego: [&'static str; 6],
    mover_teclado: &'static str,
    mover_mando: &'static str,
    arriba_teclado: &'static str,
    arriba_mando: &'static str,
    /// "{tecla} o ARRIBA": el salto del teclado tambien sale con arriba.
    saltar_o_arriba: &'static str,
    nada: &'static str,
}

const TEXTOS: Textos = Textos {
    juego: ["Disparar", "Saltar", "Dash", "Parry", "Super", "Focus"],
    mover_teclado: "flechas",
    mover_mando: "stick o cruceta",
    arriba_teclado: "ARRIBA",
    arriba_mando: "stick arriba",
    saltar_o_arriba: "ARRIBA o {}",
    nada: "-",
};

/// Las teclas que se pueden dar a una accion de juego, con su nombre en
/// pantalla y en el guardado.
///
/// Las que faltan faltan a proposito: flechas y WASD mueven, ESC y P pausan,
/// R reintenta, M quita el sonido, T para el trazo, y las F son de depurar.
/// Darle una de esas a disparar haria dos cosas a la vez. Los nombres no
/// pueden llevar `;`, `=` ni `+`, que son los separadores del guardado.
///
/// El sitio en la lista es el bit de la mascara (`Controles::teclado`), asi
/// que caben 64; el guardado va por nombre y no por sitio.
const TECLAS: [(KeyCode, &str); 37] = [
    // El espacio el primero: entre las de una accion se ensena la primera de
    // la lista, y el super es "ESPACIO" aunque la V tambien valga.
    (KeyCode::Space, "ESPACIO"),
    (KeyCode::Z, "Z"),
    (KeyCode::X, "X"),
    (KeyCode::C, "C"),
    (KeyCode::V, "V"),
    (KeyCode::B, "B"),
    (KeyCode::N, "N"),
    (KeyCode::Q, "Q"),
    (KeyCode::E, "E"),
    (KeyCode::Y, "Y"),
    (KeyCode::U, "U"),
    (KeyCode::I, "I"),
    (KeyCode::O, "O"),
    (KeyCode::F, "F"),
    (KeyCode::G, "G"),
    (KeyCode::H, "H"),
    (KeyCode::J, "J"),
    (KeyCode::K, "K"),
    (KeyCode::L, "L"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::LeftShift, "SHIFT"),
    (KeyCode::RightShift, "SHIFT DCHO"),
    (KeyCode::LeftControl, "CTRL"),
    (KeyCode::RightControl, "CTRL DCHO"),
    (KeyCode::LeftAlt, "ALT"),
    (KeyCode::RightAlt, "ALT GR"),
    (KeyCode::Tab, "TAB"),
    (KeyCode::Enter, "INTRO"),
];

/// Los botones que se pueden dar a una accion de juego. Start y Select son
/// de la pausa y del reintento, y la cruceta mueve.
const BOTONES_LIBRES: u32 = boton::SUR
    | boton::ESTE
    | boton::OESTE
    | boton::NORTE
    | boton::L1
    | boton::R1
    | boton::L2
    | boton::R2
    | boton::L3
    | boton::R3;

/// El bit de una tecla en las mascaras del teclado, si se puede asignar.
pub fn bit_tecla(k: KeyCode) -> Option<u64> {
    TECLAS.iter().position(|(t, _)| *t == k).map(|i| 1 << i)
}

/// El bit de un boton crudo en las mascaras del mando, si se puede asignar.
/// Si se pulsan dos a la vez, cuenta el primero.
pub fn bit_boton(nuevos: u32) -> Option<u64> {
    let libres = nuevos & BOTONES_LIBRES;
    (libres != 0).then(|| 1 << libres.trailing_zeros())
}

/// El nombre en pantalla de una tecla cualquiera, de las asignables o de las
/// fijas de los menus.
fn nombre_tecla(k: KeyCode) -> &'static str {
    match k {
        KeyCode::Escape => "ESC",
        KeyCode::Backspace => "BORRAR",
        KeyCode::R => "R",
        KeyCode::P => "P",
        KeyCode::L => "L",
        KeyCode::Up => "ARRIBA",
        KeyCode::Down => "ABAJO",
        KeyCode::Left => "IZQUIERDA",
        KeyCode::Right => "DERECHA",
        _ => TECLAS.iter().find(|(t, _)| *t == k).map_or("?", |(_, n)| n),
    }
}

/// El nombre de un boton crudo en este mando. Si `bit` lleva varios, el
/// primero.
fn nombre_boton(bit: u32, playstation: bool) -> &'static str {
    let (ps, xbox) = mando::NOMBRES[bit.trailing_zeros() as usize % mando::NOMBRES.len()];
    if playstation { ps } else { xbox }
}

/// Las acciones de juego, cada una con sus teclas y sus botones.
///
/// Son **mascaras**: una accion puede tener varias (disparar va en R1 y R2, el
/// super en ESPACIO y V), y asi asignar, quitar y guardar es lo mismo en el
/// teclado que en el mando.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Controles {
    /// Por accion, los bits de `TECLAS`.
    pub teclado: [u64; 6],
    /// Por accion, los botones crudos de `mando::boton`.
    pub mando: [u64; 6],
}

impl Default for Controles {
    fn default() -> Self {
        let t = |ks: &[KeyCode]| {
            ks.iter()
                .filter_map(|k| bit_tecla(*k))
                .fold(0, |a, b| a | b)
        };
        Self {
            // El salto del teclado es la K, y ademas arriba (ver
            // `del_teclado`): en el suelo arriba no mueve.
            teclado: [
                t(&[KeyCode::Z]),
                t(&[KeyCode::K]),
                t(&[KeyCode::X]),
                t(&[KeyCode::C]),
                t(&[KeyCode::Space, KeyCode::V]),
                t(&[KeyCode::LeftShift, KeyCode::RightShift]),
            ],
            // Disparar en el gatillo: se mantiene con el indice y el pulgar
            // queda libre para saltar, que va en el boton de abajo.
            mando: [
                (boton::R1 | boton::R2) as u64,
                boton::SUR as u64,
                boton::OESTE as u64,
                boton::ESTE as u64,
                boton::NORTE as u64,
                (boton::L1 | boton::L2) as u64,
            ],
        }
    }
}

impl Controles {
    pub fn de(&mut self, d: Dispositivo) -> &mut [u64; 6] {
        match d {
            Dispositivo::Teclado => &mut self.teclado,
            Dispositivo::Mando => &mut self.mando,
        }
    }

    /// Le da `bit` a la accion `j`, y solo ese.
    ///
    /// Si otra accion lo tenia, lo pierde; y si se queda sin nada, se lleva
    /// lo que tenia `j`. Es decir: **se intercambian**. Es lo que menos
    /// sorprende, y sobre todo no deja nunca una accion sin tecla, que con el
    /// parry significa no poder jugar.
    pub fn asignar(&mut self, d: Dispositivo, j: Juego, bit: u64) {
        let m = self.de(d);
        let viejo = m[j as usize];
        for (i, otra) in m.iter_mut().enumerate() {
            if i != j as usize && *otra & bit != 0 {
                *otra &= !bit;
                if *otra == 0 {
                    *otra = viejo & !bit;
                }
            }
        }
        m[j as usize] = bit;
    }

    /// Los de serie, solo los de un aparato.
    pub fn restaurar(&mut self, d: Dispositivo) {
        *self.de(d) = *Self::default().de(d);
    }

    /// Lo que mandan las teclas apretadas. `abajo` dice si una tecla esta
    /// apretada: es `is_key_down`, y en los tests, lo que haga falta.
    pub fn del_teclado(&self, abajo: impl Fn(KeyCode) -> bool) -> InputFrame {
        let algo = |a: Accion| a.teclas().iter().any(|k| abajo(*k));
        let mut f = InputFrame::NONE;
        f.set(InputFrame::UP, algo(Accion::Arriba));
        f.set(InputFrame::DOWN, algo(Accion::Abajo));
        f.set(InputFrame::LEFT, algo(Accion::Izquierda));
        f.set(InputFrame::RIGHT, algo(Accion::Derecha));
        for j in Juego::TODAS {
            let m = self.teclado[j as usize];
            let apretada = TECLAS
                .iter()
                .enumerate()
                .any(|(i, (k, _))| m & (1 << i) != 0 && abajo(*k));
            f.set(j.bit(), apretada);
        }
        // El salto reutiliza arriba: en el modo con gravedad el eje vertical no
        // mueve, asi que la tecla queda libre para lo que toca en un
        // plataformas. Con el mando no, que alli arriba en diagonal saltaba
        // sin querer.
        if algo(Accion::Arriba) {
            f.set(InputFrame::JUMP, true);
        }
        f
    }

    /// Lo que mandan los botones crudos apretados.
    pub fn del_mando(&self, crudo: u32) -> InputFrame {
        let mut f = InputFrame::NONE;
        let algo = |a: Accion| crudo & a.botones() != 0;
        f.set(InputFrame::UP, algo(Accion::Arriba));
        f.set(InputFrame::DOWN, algo(Accion::Abajo));
        f.set(InputFrame::LEFT, algo(Accion::Izquierda));
        f.set(InputFrame::RIGHT, algo(Accion::Derecha));
        for j in Juego::TODAS {
            f.set(j.bit(), self.mando[j as usize] & crudo as u64 != 0);
        }
        f
    }

    /// El nombre de lo que tiene la accion `j` en el aparato `d`: la primera
    /// de sus teclas o botones, que es la que se ensena.
    pub fn nombre(&self, d: Dispositivo, j: Juego, playstation: bool) -> &'static str {
        match d {
            Dispositivo::Teclado => {
                let m = self.teclado[j as usize];
                TECLAS
                    .iter()
                    .enumerate()
                    .find(|(i, _)| m & (1 << i) != 0)
                    .map_or(TEXTOS.nada, |(_, (_, n))| n)
            }
            Dispositivo::Mando => {
                let m = self.mando[j as usize] as u32;
                if m == 0 {
                    TEXTOS.nada
                } else {
                    nombre_boton(m, playstation)
                }
            }
        }
    }

    /// Lo que se guarda: `teclado.disparar=Z`, `mando.super=3+5`... Solo lo
    /// que no es de serie, para que un guardado sin cambios se escriba igual
    /// que antes de que hubiera controles.
    pub fn a_texto(&self) -> Vec<String> {
        let serie = Self::default();
        let mut out = Vec::new();
        for j in Juego::TODAS {
            let i = j as usize;
            if self.teclado[i] != serie.teclado[i] {
                let nombres: Vec<&str> = TECLAS
                    .iter()
                    .enumerate()
                    .filter(|(b, _)| self.teclado[i] & (1 << b) != 0)
                    .map(|(_, (_, n))| *n)
                    .collect();
                out.push(format!("teclado.{}={}", j.clave(), nombres.join("+")));
            }
            if self.mando[i] != serie.mando[i] {
                let bits: Vec<String> = (0..17)
                    .filter(|b| self.mando[i] & (1 << b) != 0)
                    .map(|b| b.to_string())
                    .collect();
                out.push(format!("mando.{}={}", j.clave(), bits.join("+")));
            }
        }
        out
    }

    /// Lee una entrada de `a_texto`. Devuelve si era suya.
    ///
    /// Lo que no se entiende se ignora y deja lo de serie: un nombre de tecla
    /// de otra version, un boton que no se puede asignar, un guardado tocado a
    /// mano. Mejor jugar con lo de serie que quedarse sin parry.
    pub fn leer(&mut self, clave: &str, valor: &str) -> bool {
        let Some((aparato, accion)) = clave.split_once('.') else {
            return false;
        };
        let Some(j) = Juego::TODAS.into_iter().find(|j| j.clave() == accion) else {
            return false;
        };
        let mascara = match aparato {
            "teclado" => valor
                .split('+')
                .filter_map(|n| TECLAS.iter().position(|(_, t)| *t == n))
                .fold(0u64, |m, i| m | 1 << i),
            "mando" => valor
                .split('+')
                .filter_map(|n| n.parse::<u32>().ok())
                .filter(|b| *b < 17 && BOTONES_LIBRES & (1 << b) != 0)
                .fold(0u64, |m, b| m | 1 << b),
            _ => return false,
        };
        if mascara != 0 {
            let d = if aparato == "teclado" {
                Dispositivo::Teclado
            } else {
                Dispositivo::Mando
            };
            self.de(d)[j as usize] = mascara;
        }
        true
    }
}

/// Todos los nombres que salen en pantalla, en el aparato que se este usando.
///
/// Se monta una vez por frame y se pasa a quien dibuja ayuda: es **el unico
/// sitio** que decide si se dice "Z" o "X", y por eso ya no puede pasar que el
/// menu hable del mando y el KO del teclado.
#[derive(Clone, Debug)]
pub struct Nombres {
    pub aceptar: &'static str,
    pub atras: &'static str,
    pub reintentar: &'static str,
    pub pausa: &'static str,
    pub pasear: &'static str,
    pub libro: &'static str,
    pub historia: &'static str,
    pub mover: &'static str,
    pub arriba: &'static str,
    pub disparar: &'static str,
    pub saltar: String,
    pub dash: &'static str,
    pub parry: &'static str,
    pub super_: &'static str,
    pub focus: &'static str,
}

impl Nombres {
    pub fn de(d: Dispositivo, playstation: bool, c: &Controles) -> Self {
        let accion = |a: Accion| match d {
            Dispositivo::Teclado => nombre_tecla(a.teclas()[0]),
            Dispositivo::Mando => nombre_boton(a.botones(), playstation),
        };
        let juego = |j: Juego| c.nombre(d, j, playstation);
        let teclado = d == Dispositivo::Teclado;
        Self {
            aceptar: accion(Accion::Aceptar),
            atras: accion(Accion::Atras),
            reintentar: accion(Accion::Reintentar),
            pausa: accion(Accion::Pausa),
            pasear: accion(Accion::Pasear),
            libro: accion(Accion::Libro),
            historia: accion(Accion::Historia),
            mover: if teclado {
                TEXTOS.mover_teclado
            } else {
                TEXTOS.mover_mando
            },
            arriba: if teclado {
                TEXTOS.arriba_teclado
            } else {
                TEXTOS.arriba_mando
            },
            disparar: juego(Juego::Disparar),
            saltar: if teclado {
                TEXTOS.saltar_o_arriba.replace("{}", juego(Juego::Saltar))
            } else {
                juego(Juego::Saltar).to_owned()
            },
            dash: juego(Juego::Dash),
            parry: juego(Juego::Parry),
            super_: juego(Juego::Super),
            focus: juego(Juego::Focus),
        }
    }
}

/// El teclado y el mando juntos, y con cual se jugo lo ultimo.
pub struct Entrada {
    pub mando: Mando,
    pub ultimo: Dispositivo,
    /// La tecla que se acaba de pulsar, si alguna. Para cambiar controles.
    tecla: Option<KeyCode>,
}

impl Entrada {
    pub fn new() -> Self {
        Self {
            mando: Mando::new(),
            ultimo: Dispositivo::Teclado,
            tecla: None,
        }
    }

    /// Una vez por frame, antes de leer nada.
    pub fn actualizar(&mut self) {
        self.mando.actualizar();
        self.tecla = get_last_key_pressed();
        self.ultimo = ultimo_usado(
            self.ultimo,
            self.tecla.is_some(),
            self.mando.algo_pulsado() || self.mando.recien_conectado(),
            self.mando.conectado(),
        );
    }

    /// Si se acaba de pulsar una accion de menu, con lo que sea.
    pub fn pulsado(&self, a: Accion) -> bool {
        a.teclas().iter().any(|k| is_key_pressed(*k)) || self.mando.pulsado(a.botones())
    }

    /// Si se acaba de pulsar cualquier cosa.
    pub fn algo(&self) -> bool {
        self.tecla.is_some() || self.mando.algo_pulsado()
    }

    pub fn tecla(&self) -> Option<KeyCode> {
        self.tecla
    }

    /// Lo que le llega a la simulacion. Teclado y mando se suman en vez de
    /// elegir uno: soltar el mando a media partida no rompe nada.
    pub fn juego(&self, c: &Controles) -> InputFrame {
        InputFrame::from_bits(
            c.del_teclado(is_key_down).bits() | c.del_mando(self.mando.crudo()).bits(),
        )
    }

    pub fn nombres(&self, c: &Controles) -> Nombres {
        Nombres::de(self.ultimo, self.mando.playstation(), c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_acciones_de_menu_tienen_tecla_y_boton() {
        assert!(Accion::Aceptar.teclas().contains(&KeyCode::Z));
        assert_eq!(Accion::Pausa.botones(), boton::START);
        assert_eq!(Accion::Reintentar.botones(), boton::SELECT);
        assert_eq!(Accion::Reintentar.teclas(), &[KeyCode::R]);
        // Ninguna tecla de menu se puede dar a una accion de juego: haria dos
        // cosas a la vez.
        for a in [
            Accion::Atras,
            Accion::Reintentar,
            Accion::Pausa,
            Accion::Arriba,
        ] {
            for k in a.teclas() {
                assert_eq!(bit_tecla(*k), None, "{k:?}");
            }
        }
        assert_eq!(bit_boton(boton::START | boton::ARRIBA), None);
    }

    #[test]
    fn lo_de_serie_llega_a_la_simulacion() {
        let c = Controles::default();
        let f = c.del_teclado(|k| k == KeyCode::V || k == KeyCode::Left);
        assert!(f.is_down(InputFrame::SUPER) && f.is_down(InputFrame::LEFT));
        assert!(!f.is_down(InputFrame::SHOOT));
        // Arriba salta en el teclado.
        assert!(c.del_teclado(|k| k == KeyCode::W).is_down(InputFrame::JUMP));
        let f = c.del_mando(boton::R2 | boton::SUR | boton::DERECHA);
        assert!(f.is_down(InputFrame::SHOOT) && f.is_down(InputFrame::JUMP));
        assert!(f.is_down(InputFrame::RIGHT) && !f.is_down(InputFrame::PARRY));
        // Start y Select no son de la partida.
        assert_eq!(c.del_mando(boton::START | boton::SELECT).bits(), 0);
    }

    #[test]
    fn se_juega_con_lo_ultimo_que_se_toco() {
        use Dispositivo::*;
        assert_eq!(ultimo_usado(Teclado, false, true, true), Mando);
        assert_eq!(ultimo_usado(Mando, true, false, true), Teclado);
        assert_eq!(
            ultimo_usado(Mando, false, false, true),
            Mando,
            "sin nada, igual"
        );
        assert_eq!(
            ultimo_usado(Mando, false, false, false),
            Teclado,
            "desenchufado"
        );
        // Y los nombres siguen al aparato.
        let c = Controles::default();
        assert_eq!(Nombres::de(Teclado, false, &c).reintentar, "R");
        assert_eq!(Nombres::de(Mando, false, &c).reintentar, "Back");
        assert_eq!(Nombres::de(Mando, true, &c).pausa, "Options");
        assert_eq!(Nombres::de(Mando, true, &c).parry, "circulo");
        assert_eq!(Nombres::de(Teclado, true, &c).parry, "C");
    }

    #[test]
    fn asignar_una_tecla_ocupada_las_intercambia() {
        let mut c = Controles::default();
        let x = bit_tecla(KeyCode::X).unwrap();
        // El dash tenia la X: al darsela a disparar, el dash se queda la Z.
        c.asignar(Dispositivo::Teclado, Juego::Disparar, x);
        assert_eq!(c.nombre(Dispositivo::Teclado, Juego::Disparar, false), "X");
        assert_eq!(c.nombre(Dispositivo::Teclado, Juego::Dash, false), "Z");
        // Si la otra tenia dos, solo pierde esa y no se lleva nada.
        let v = bit_tecla(KeyCode::V).unwrap();
        c.asignar(Dispositivo::Teclado, Juego::Parry, v);
        assert_eq!(
            c.nombre(Dispositivo::Teclado, Juego::Super, false),
            "ESPACIO"
        );
        assert_eq!(c.teclado[Juego::Super as usize].count_ones(), 1);
        // Nunca hay una tecla en dos sitios ni una accion sin nada.
        let todas = c.teclado.iter().fold(0, |a, m| {
            assert!(*m != 0);
            assert_eq!(a & m, 0);
            a | m
        });
        assert!(todas != 0);
        // Con el mando igual.
        c.asignar(Dispositivo::Mando, Juego::Parry, boton::R2 as u64);
        assert_eq!(c.mando[Juego::Disparar as usize], boton::R1 as u64);
        c.restaurar(Dispositivo::Mando);
        assert_eq!(c.mando, Controles::default().mando);
        assert_ne!(
            c.teclado,
            Controles::default().teclado,
            "el teclado no se toca"
        );
    }

    #[test]
    fn los_controles_se_guardan_y_se_leen() {
        let mut c = Controles::default();
        assert!(c.a_texto().is_empty(), "lo de serie no se escribe");
        c.asignar(
            Dispositivo::Teclado,
            Juego::Focus,
            bit_tecla(KeyCode::J).unwrap(),
        );
        c.asignar(Dispositivo::Mando, Juego::Super, boton::L3 as u64);
        let mut leido = Controles::default();
        for e in c.a_texto() {
            let (k, v) = e.split_once('=').unwrap();
            assert!(leido.leer(k, v));
        }
        assert_eq!(leido, c);
        // Lo que no se entiende deja lo de serie.
        let mut raro = Controles::default();
        assert!(raro.leer("teclado.parry", "TECLA DEL FUTURO"));
        assert!(raro.leer("mando.parry", "9"), "Start no se puede asignar");
        assert!(!raro.leer("raton.parry", "1"));
        assert_eq!(raro, Controles::default());
    }
}
