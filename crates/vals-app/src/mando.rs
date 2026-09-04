//! El mando.
//!
//! macroquad 0.4 no lo trae: su propio modulo de input dice "cross-platform
//! mouse, keyboard (and gamepads soon)". Asi que hay que ponerlo a mano, y son
//! dos caminos distintos porque no hay ninguno que sirva para los dos:
//!
//! - **Nativo**: `gilrs`, que es la opcion estandar y hace de traductor entre
//!   XInput, DirectInput y evdev.
//! - **Web**: la Gamepad API del navegador, leida desde JS y pasada a Rust por
//!   el mismo mecanismo de plugins de miniquad que ya usa el audio. Sin
//!   `wasm-bindgen`, que es justo lo que no se quiso arrastrar al elegir
//!   macroquad.
//!
//! Los dos lados producen **los mismos bits**, que son los de `InputFrame` mas
//! uno que dice si hay algo conectado. Toda la traduccion —stick a cruceta,
//! zona muerta, que boton es que— ocurre en cada lado, y aqui solo se junta.
//!
//! Ojo: el lado de JS vive en `web/index.html` y **repite estos numeros a
//! mano**. Hay un test abajo que falla si los bits de `InputFrame` se mueven,
//! porque si no el fallo seria silencioso y solo en web.

use vals_core::InputFrame;

/// Bit que anaden los dos lados para decir que hay un mando conectado.
///
/// Va fuera del rango de `InputFrame` a proposito: asi el mismo `u16` lleva
/// los botones y la presencia sin inventar otro canal.
pub const CONECTADO: u16 = 1 << 15;

/// Y este dice que el mando es de PlayStation.
///
/// Solo sirve para poner los nombres correctos en pantalla. Los botones son
/// los mismos —el sur es el sur—, pero decirle "Y" a alguien que tiene un
/// mando con un triangulo no le ayuda a encontrarlo.
pub const PLAYSTATION: u16 = 1 << 14;

/// Como se llaman los cuatro botones de la derecha en este mando.
///
/// Se decide por el nombre que da el sistema, no por el protocolo: un mando de
/// PS4 en Windows entra por DirectInput y aun asi se llama a si mismo
/// "Wireless Controller".
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn es_de_playstation(nombre: &str) -> bool {
    let n = nombre.to_ascii_lowercase();
    [
        "playstation",
        "dualshock",
        "dualsense",
        "sony",
        "wireless controller",
    ]
    .iter()
    .any(|pista| n.contains(pista))
}

/// Lo lejos que hay que empujar el stick para que cuente.
///
/// Generosa: en un danmaku lo que se hace son diagonales limpias, no medidas
/// finas, y un stick gastado tiembla alrededor del centro.
#[cfg(not(target_arch = "wasm32"))]
const ZONA_MUERTA: f32 = 0.45;

/// miniquad comprueba que la version del plugin de JS y la del crate cuadren,
/// y para eso busca esta funcion. Sin ella el plugin funciona igual pero deja
/// un aviso en la consola diciendo que nadie lo usa, que es mentira y confunde.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vals_mando_crate_version() -> u32 {
    1
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    /// La pone `web/index.html`. Si faltara, el bundle de miniquad la sustituye
    /// por un aviso en consola en vez de reventar la carga.
    fn vals_mando() -> u32;
}

/// El estado del mando, con memoria de un frame para poder detectar flancos.
pub struct Mando {
    anterior: u16,
    actual: u16,
    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<gilrs::Gilrs>,
}

impl Mando {
    pub fn new() -> Self {
        Self {
            anterior: 0,
            actual: 0,
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: gilrs::Gilrs::new().ok(),
        }
    }

    /// Una vez por frame, antes de leer nada.
    pub fn actualizar(&mut self) {
        self.anterior = self.actual;
        self.actual = self.leer();
    }

    /// Si hay un mando enchufado. Solo sirve para decirlo en pantalla: el juego
    /// suma mando y teclado sin preguntar cual se esta usando.
    pub fn conectado(&self) -> bool {
        self.actual & CONECTADO != 0
    }

    /// Los botones que estan pulsados ahora.
    pub fn frame(&self) -> InputFrame {
        InputFrame::from_bits(self.actual & !(CONECTADO | PLAYSTATION))
    }

    /// Los cuatro botones de la derecha, con el nombre que llevan escrito
    /// encima en **este** mando.
    ///
    /// Devuelve (disparar, dash, parry, super).
    pub fn botones(&self) -> [&'static str; 4] {
        if self.actual & PLAYSTATION != 0 {
            ["X", "cuadrado", "circulo", "triangulo"]
        } else {
            ["A", "X", "B", "Y"]
        }
    }

    /// Si **algun** boton se acaba de pulsar en este frame.
    ///
    /// Distinto de mirar si hay algo apretado: al entrar a un baile con el
    /// mando, el boton con el que entras sigue pulsado despues, y confundir las
    /// dos cosas hacia que la cartela de presentacion se saltara sola.
    pub fn algo_pulsado(&self) -> bool {
        let botones = !(CONECTADO | PLAYSTATION);
        (self.actual & botones) & !(self.anterior & botones) != 0
    }

    /// Si un boton se acaba de pulsar en este frame.
    ///
    /// Lo necesitan el menu y la pista, que reaccionan a la pulsacion y no a
    /// tenerlo apretado: sin esto, entrar a un baile con el mando volveria a
    /// entrar sesenta veces por segundo.
    pub fn pulsado(&self, boton: u16) -> bool {
        self.actual & boton != 0 && self.anterior & boton == 0
    }

    #[cfg(target_arch = "wasm32")]
    fn leer(&mut self) -> u16 {
        // SAFETY: la funcion la aporta el plugin de `web/index.html`, y si no
        // estuviera, miniquad la sustituye por un stub que devuelve undefined
        // -> 0. En los dos casos leerla es seguro.
        (unsafe { vals_mando() }) as u16
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn leer(&mut self) -> u16 {
        use gilrs::{Axis, Button};

        let Some(g) = self.gilrs.as_mut() else {
            return 0;
        };
        // Vaciar la cola es lo que pone al dia el estado de los mandos. Sin
        // esto `gamepads()` devuelve lo de hace un rato.
        while g.next_event().is_some() {}

        let Some((_, pad)) = g.gamepads().next() else {
            return 0;
        };

        let (ax, ay) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
        let mut bits = CONECTADO;
        if es_de_playstation(pad.name()) {
            bits |= PLAYSTATION;
        }
        let mut set = |b: u16, v: bool| {
            if v {
                bits |= b;
            }
        };

        // El stick y la cruceta hacen lo mismo: nadie deberia tener que
        // acordarse de cual de los dos mueve.
        let arriba = ay > ZONA_MUERTA || pad.is_pressed(Button::DPadUp);
        set(InputFrame::UP, arriba);
        set(
            InputFrame::DOWN,
            ay < -ZONA_MUERTA || pad.is_pressed(Button::DPadDown),
        );
        set(
            InputFrame::LEFT,
            ax < -ZONA_MUERTA || pad.is_pressed(Button::DPadLeft),
        );
        set(
            InputFrame::RIGHT,
            ax > ZONA_MUERTA || pad.is_pressed(Button::DPadRight),
        );
        // Igual que en el teclado, saltar es "arriba": en el modo con gravedad
        // el eje vertical no mueve y la direccion queda libre.
        set(InputFrame::JUMP, arriba);

        set(InputFrame::SHOOT, pad.is_pressed(Button::South));
        set(InputFrame::DASH, pad.is_pressed(Button::West));
        set(InputFrame::PARRY, pad.is_pressed(Button::East));
        set(InputFrame::SUPER, pad.is_pressed(Button::North));
        set(
            InputFrame::FOCUS,
            pad.is_pressed(Button::LeftTrigger) || pad.is_pressed(Button::LeftTrigger2),
        );
        bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Los bits que `web/index.html` repite a mano.
    ///
    /// Si alguien reordena `InputFrame`, el nativo se entera solo —usa las
    /// constantes— pero la web no, porque alli los numeros estan escritos en
    /// JavaScript. El fallo seria que en el navegador el boton de disparar
    /// hiciera un dash, y solo en el navegador. Esto lo convierte en un test
    /// rojo antes de salir del repositorio.
    #[test]
    fn los_bits_son_los_que_repite_el_javascript() {
        assert_eq!(InputFrame::UP, 1 << 0);
        assert_eq!(InputFrame::DOWN, 1 << 1);
        assert_eq!(InputFrame::LEFT, 1 << 2);
        assert_eq!(InputFrame::RIGHT, 1 << 3);
        assert_eq!(InputFrame::FOCUS, 1 << 4);
        assert_eq!(InputFrame::SHOOT, 1 << 5);
        assert_eq!(InputFrame::DASH, 1 << 6);
        assert_eq!(InputFrame::PARRY, 1 << 7);
        assert_eq!(InputFrame::SUPER, 1 << 8);
        assert_eq!(InputFrame::JUMP, 1 << 9);
        assert_eq!(CONECTADO, 1 << 15);
        assert_eq!(PLAYSTATION, 1 << 14);
    }

    #[test]
    fn se_reconoce_un_mando_de_playstation_por_el_nombre() {
        // El de PS4 en Windows entra por DirectInput y se llama a si mismo
        // "Wireless Controller", que es el caso que hay que acertar.
        assert!(es_de_playstation("Wireless Controller"));
        assert!(es_de_playstation("Sony DualShock 4"));
        assert!(es_de_playstation("DualSense Wireless Controller"));
        assert!(!es_de_playstation("Xbox 360 Controller"));
        assert!(!es_de_playstation("Xbox Series X Controller"));
    }

    #[test]
    fn el_nombre_del_mando_no_se_cuela_entre_los_botones() {
        let m = Mando {
            anterior: 0,
            actual: CONECTADO | PLAYSTATION,
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: None,
        };
        assert_eq!(m.frame().bits(), 0);
        assert_eq!(m.botones()[3], "triangulo");
    }

    #[test]
    fn algo_pulsado_es_un_flanco_y_no_un_estado() {
        let mut m = Mando {
            anterior: 0,
            actual: 0,
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: None,
        };
        m.actual = CONECTADO | InputFrame::SHOOT;
        assert!(m.algo_pulsado(), "el frame en que se pulsa, si");
        m.anterior = m.actual;
        assert!(!m.algo_pulsado(), "mantenerlo apretado, no");
        // Y estar conectado no cuenta como pulsar.
        m.anterior = 0;
        m.actual = CONECTADO | PLAYSTATION;
        assert!(!m.algo_pulsado());
    }

    #[test]
    fn sin_mando_no_se_pulsa_nada() {
        let m = Mando {
            anterior: 0,
            actual: 0,
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: None,
        };
        assert!(!m.conectado());
        assert_eq!(m.frame().bits(), 0);
        assert!(!m.pulsado(InputFrame::SHOOT));
    }

    #[test]
    fn el_flanco_solo_salta_una_vez() {
        let mut m = Mando {
            anterior: 0,
            actual: 0,
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: None,
        };
        m.anterior = 0;
        m.actual = CONECTADO | InputFrame::SHOOT;
        assert!(m.pulsado(InputFrame::SHOOT), "el primer frame si");
        m.anterior = m.actual;
        assert!(!m.pulsado(InputFrame::SHOOT), "mantenerlo ya no cuenta");
    }

    #[test]
    fn el_bit_de_conectado_no_se_cuela_entre_los_botones() {
        let m = Mando {
            anterior: 0,
            actual: CONECTADO,
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: None,
        };
        assert!(m.conectado());
        assert_eq!(m.botones()[0], "A", "sin marca de Sony, nombres de Xbox");
        assert_eq!(m.frame().bits(), 0, "estar conectado no es pulsar nada");
    }
}
