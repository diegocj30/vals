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
//! Los dos lados producen **los mismos bits crudos**: un bit por boton fisico,
//! numerados como el "standard gamepad" de la Gamepad API (0 el de abajo, 1 el
//! de la derecha, ... 9 Start), mas dos que dicen si hay algo conectado y si es
//! de PlayStation. Que hace cada boton no se decide aqui sino en `acciones.rs`,
//! que es donde se pueden cambiar: asi el rebindeo del mando funciona igual en
//! nativo y en web sin repetir la tabla en JavaScript.
//!
//! Ojo: el lado de JS vive en `web/index.html` y **repite estos numeros a
//! mano**. Hay un test abajo que falla si se mueven, porque si no el fallo
//! seria silencioso y solo en web.

/// Los botones crudos, por su sitio en el mando y no por lo que hacen.
///
/// Es el orden del "standard gamepad" del W3C. En nativo se traduce desde los
/// nombres de gilrs; en web llega tal cual del navegador.
pub mod boton {
    pub const SUR: u32 = 1 << 0;
    pub const ESTE: u32 = 1 << 1;
    pub const OESTE: u32 = 1 << 2;
    pub const NORTE: u32 = 1 << 3;
    pub const L1: u32 = 1 << 4;
    pub const R1: u32 = 1 << 5;
    pub const L2: u32 = 1 << 6;
    pub const R2: u32 = 1 << 7;
    pub const SELECT: u32 = 1 << 8;
    pub const START: u32 = 1 << 9;
    pub const L3: u32 = 1 << 10;
    pub const R3: u32 = 1 << 11;
    /// La cruceta. Cada lado le suma el stick, con su zona muerta: nadie
    /// deberia tener que acordarse de cual de los dos mueve.
    pub const ARRIBA: u32 = 1 << 12;
    pub const ABAJO: u32 = 1 << 13;
    pub const IZQUIERDA: u32 = 1 << 14;
    pub const DERECHA: u32 = 1 << 15;
    /// Todos los botones juntos, sin los bits de estado. El 16 es el del
    /// centro (PS, Xbox): se lee, pero el sistema suele quedarselo y no se
    /// usa para nada.
    pub const TODOS: u32 = (1 << 17) - 1;
}

/// Bit que anaden los dos lados para decir que hay un mando conectado.
///
/// Va fuera del rango de los botones a proposito: asi el mismo numero lleva
/// los botones y la presencia sin inventar otro canal.
pub const CONECTADO: u32 = 1 << 20;

/// Y este dice que el mando es de PlayStation.
///
/// Solo sirve para poner los nombres correctos en pantalla. Los botones son
/// los mismos —el sur es el sur—, pero decirle "Y" a alguien que tiene un
/// mando con un triangulo no le ayuda a encontrarlo.
pub const PLAYSTATION: u32 = 1 << 21;

/// Como se llama cada boton crudo, por su numero de bit: (PlayStation, Xbox).
pub const NOMBRES: [(&str, &str); 17] = [
    ("X", "A"),
    ("circulo", "B"),
    ("cuadrado", "X"),
    ("triangulo", "Y"),
    ("L1", "LB"),
    ("R1", "RB"),
    ("L2", "LT"),
    ("R2", "RT"),
    ("Share", "Back"),
    ("Options", "Start"),
    ("L3", "LS"),
    ("R3", "RS"),
    ("arriba", "arriba"),
    ("abajo", "abajo"),
    ("izquierda", "izquierda"),
    ("derecha", "derecha"),
    ("PS", "Xbox"),
];

/// Si el mando es de PlayStation.
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
    anterior: u32,
    actual: u32,
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

    /// Si hay un mando enchufado.
    pub fn conectado(&self) -> bool {
        self.actual & CONECTADO != 0
    }

    /// Si se acaba de enchufar en este frame: es cuando las ayudas en pantalla
    /// pasan a hablar de botones aunque todavia no se haya pulsado ninguno.
    pub fn recien_conectado(&self) -> bool {
        self.conectado() && self.anterior & CONECTADO == 0
    }

    /// Si es de PlayStation, para los nombres.
    pub fn playstation(&self) -> bool {
        self.actual & PLAYSTATION != 0
    }

    /// Los botones crudos que estan apretados ahora.
    pub fn crudo(&self) -> u32 {
        self.actual & boton::TODOS
    }

    /// Los botones crudos que se acaban de pulsar en este frame.
    ///
    /// Distinto de mirar si hay algo apretado: al entrar a un baile con el
    /// mando, el boton con el que entras sigue pulsado despues, y confundir las
    /// dos cosas hacia que la cartela de presentacion se saltara sola.
    pub fn nuevos(&self) -> u32 {
        self.crudo() & !(self.anterior & boton::TODOS)
    }

    /// Si **algun** boton se acaba de pulsar en este frame.
    pub fn algo_pulsado(&self) -> bool {
        self.nuevos() != 0
    }

    /// Si alguno de los botones de `mascara` se acaba de pulsar.
    ///
    /// Lo necesitan los menus, que reaccionan a la pulsacion y no a tenerla
    /// apretada: sin esto, entrar a un baile con el mando volveria a entrar
    /// sesenta veces por segundo.
    pub fn pulsado(&self, mascara: u32) -> bool {
        self.nuevos() & mascara != 0
    }

    #[cfg(target_arch = "wasm32")]
    fn leer(&mut self) -> u32 {
        // SAFETY: la funcion la aporta el plugin de `web/index.html`, y si no
        // estuviera, miniquad la sustituye por un stub que devuelve undefined
        // -> 0. En los dos casos leerla es seguro.
        unsafe { vals_mando() }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn leer(&mut self) -> u32 {
        use gilrs::{Axis, Button};

        // En el orden del "standard gamepad", que es el que da el navegador:
        // asi los dos lados entregan los mismos bits. En gilrs el gatillo de
        // arriba es `LeftTrigger` y el de abajo `LeftTrigger2`.
        const ORDEN: [Button; 17] = [
            Button::South,
            Button::East,
            Button::West,
            Button::North,
            Button::LeftTrigger,
            Button::RightTrigger,
            Button::LeftTrigger2,
            Button::RightTrigger2,
            Button::Select,
            Button::Start,
            Button::LeftThumb,
            Button::RightThumb,
            Button::DPadUp,
            Button::DPadDown,
            Button::DPadLeft,
            Button::DPadRight,
            Button::Mode,
        ];

        let Some(g) = self.gilrs.as_mut() else {
            return 0;
        };
        // Vaciar la cola es lo que pone al dia el estado de los mandos. Sin
        // esto `gamepads()` devuelve lo de hace un rato.
        while g.next_event().is_some() {}

        let Some((_, pad)) = g.gamepads().next() else {
            return 0;
        };

        let mut bits = CONECTADO;
        if es_de_playstation(pad.name()) {
            bits |= PLAYSTATION;
        }
        for (i, b) in ORDEN.iter().enumerate() {
            if pad.is_pressed(*b) {
                bits |= 1 << i;
            }
        }
        // El stick se suma a la cruceta.
        let (ax, ay) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
        for (b, v) in [
            (boton::ARRIBA, ay > ZONA_MUERTA),
            (boton::ABAJO, ay < -ZONA_MUERTA),
            (boton::IZQUIERDA, ax < -ZONA_MUERTA),
            (boton::DERECHA, ax > ZONA_MUERTA),
        ] {
            if v {
                bits |= b;
            }
        }
        bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mando(anterior: u32, actual: u32) -> Mando {
        Mando {
            anterior,
            actual,
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: None,
        }
    }

    /// Los bits que `web/index.html` repite a mano.
    ///
    /// El nativo se entera solo si se mueven —usa las constantes— pero la web
    /// no, porque alli los numeros estan escritos en JavaScript. El fallo seria
    /// que en el navegador el Start hiciera de parry, y solo en el navegador.
    /// Esto lo convierte en un test rojo antes de salir del repositorio.
    #[test]
    fn los_bits_son_los_que_repite_el_javascript() {
        // El JS pone el bit i para `buttons[i]` y suma el stick a 12..15.
        assert_eq!(boton::SUR, 1 << 0);
        assert_eq!(boton::SELECT, 1 << 8);
        assert_eq!(boton::START, 1 << 9);
        assert_eq!(boton::ARRIBA, 1 << 12);
        assert_eq!(boton::DERECHA, 1 << 15);
        assert_eq!(boton::TODOS, (1 << 17) - 1);
        assert_eq!(CONECTADO, 1 << 20);
        assert_eq!(PLAYSTATION, 1 << 21);
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
    fn el_estado_no_se_cuela_entre_los_botones() {
        let m = mando(0, CONECTADO | PLAYSTATION);
        assert!(m.conectado() && m.playstation() && m.recien_conectado());
        assert_eq!(m.crudo(), 0, "estar conectado no es pulsar nada");
        assert!(!m.algo_pulsado());
    }

    #[test]
    fn algo_pulsado_es_un_flanco_y_no_un_estado() {
        let mut m = mando(0, CONECTADO | boton::R2);
        assert!(m.algo_pulsado(), "el frame en que se pulsa, si");
        assert!(m.pulsado(boton::R1 | boton::R2));
        assert!(!m.pulsado(boton::SUR));
        m.anterior = m.actual;
        assert!(!m.algo_pulsado(), "mantenerlo apretado, no");
        assert!(!m.recien_conectado());
    }

    #[test]
    fn sin_mando_no_se_pulsa_nada() {
        let m = mando(0, 0);
        assert!(!m.conectado());
        assert_eq!(m.crudo(), 0);
        assert!(!m.pulsado(boton::TODOS));
    }
}
