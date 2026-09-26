//! Las cartelas: lo que presenta un baile antes de bailarlo.
//!
//! Es lo que hace que un jefe parezca un numero de un programa y no el
//! siguiente enemigo. Cuphead lo hace con su "READY? WALLOP!"; aqui lo que toca
//! anunciar es **el baile, la figura y la pieza que suena**, porque ese es el
//! juego.
//!
//! Dos clases, y la diferencia importa:
//!
//! - **La de entrada para el mundo.** Se lee antes de empezar, no mientras te
//!   disparan: una cartela que hay que leer esquivando no se lee.
//! - **La de figura no lo para.** Salta al cambiar de fase, en mitad del
//!   combate, y es corta. Ahi parar seria quitarle el ritmo a lo que va bien.
//!
//! Y un fundido entre escenas, que es lo que separa un corte de un cambio.

use macroquad::prelude::*;

use crate::draw::Layout;
use crate::fuentes::{self, Cara};

// Todo esto va en **segundos de reloj**, no en ticks. Es presentacion: tiene
// que durar lo mismo a 60 que a 144 Hz, y sobre todo tiene que seguir corriendo
// mientras la cartela de entrada tiene el mundo parado, que es justo cuando no
// hay ticks.

/// Lo que dura la cartela de entrada: lo justo para leer tres lineas sin que
/// apetezca saltarsela.
const ENTRADA: f32 = 2.5;
/// Y la de figura, que solo tiene una linea.
const FIGURA: f32 = 1.35;
/// Aparicion y desaparicion.
const ENTRA: f32 = 0.36;
const SALE: f32 = 0.5;
/// Lo que tarda en poder saltarse.
///
/// Sin esto la cartela se salta sola: se entra a un baile pulsando un boton, y
/// ese mismo boton sigue pulsado el frame siguiente. Ademas de mirar flancos y
/// no si algo esta apretado, hace falta un margen: hasta el flanco mas limpio
/// llega demasiado pronto si vienes de aporrear el mando.
const GRACIA: f32 = 0.7;

const VELO: Color = color_u8!(6, 6, 12, 235);
const TITULO: Color = color_u8!(212, 242, 255, 255);
const SUBRAYA: Color = color_u8!(255, 145, 210, 255);
const TEXTO: Color = color_u8!(150, 210, 235, 255);
const TENUE: Color = color_u8!(95, 125, 150, 255);

/// El terciopelo del telon, su pliegue y el galon dorado del borde.
const TELON: Color = color_u8!(74, 24, 30, 255);
const TELON_PLIEGUE: Color = color_u8!(48, 14, 20, 190);
const TELON_ORO: Color = color_u8!(198, 158, 84, 220);
/// Rayas por mitad. Cuatro bastan para que la tela no sea un rectangulo.
const PLIEGUES: usize = 5;

/// Como se llama cada figura en el programa.
const ORDINALES: [&str; 4] = [
    "FIGURA PRIMERA",
    "FIGURA SEGUNDA",
    "FIGURA TERCERA",
    "FIGURA CUARTA",
];

/// El ordinal de una figura. Si algun dia un jefe tiene mas de cuatro, se queda
/// sin ordinal en vez de reventar: una cartela es presentacion, y la
/// presentacion nunca debe tumbar el juego.
pub(crate) fn ordinal(numero: usize) -> String {
    ORDINALES.get(numero).unwrap_or(&"").to_string()
}

/// Una cartela en pantalla.
pub struct Cartela {
    baile: String,
    /// "FIGURA PRIMERA" y demas. Un programa de mano numera los numeros, y ese
    /// ordinal es la mitad de lo que hace que esto parezca un espectaculo.
    orden: String,
    figura: String,
    pieza: String,
    restante: f32,
    total: f32,
    /// Si para el mundo mientras se lee.
    para: bool,
}

impl Cartela {
    /// La de entrada a un baile. Para el mundo.
    pub fn entrada(baile: &str, figura: &str, pieza: &str) -> Self {
        Self {
            baile: baile.to_owned(),
            orden: ordinal(0),
            figura: figura.to_owned(),
            pieza: pieza.to_owned(),
            restante: ENTRADA,
            total: ENTRADA,
            para: true,
        }
    }

    /// La de cambio de figura. No para nada.
    ///
    /// `numero` es la figura en la que se entra, contando desde cero. En un
    /// jefe de tres o cuatro figuras, saber por cual vas es informacion de
    /// verdad y no adorno: dice cuanto queda.
    pub fn figura(figura: &str, numero: usize) -> Self {
        Self {
            baile: String::new(),
            orden: ordinal(numero),
            figura: figura.to_owned(),
            pieza: String::new(),
            restante: FIGURA,
            total: FIGURA,
            para: false,
        }
    }

    /// Un frame. Devuelve `false` cuando se ha acabado.
    pub fn step(&mut self, dt: f32) -> bool {
        self.restante = (self.restante - dt).max(0.0);
        self.restante > 0.0
    }

    /// Si el mundo tiene que esperar a que termine.
    pub fn para_el_mundo(&self) -> bool {
        self.para && self.restante > 0.0
    }

    /// Si ya lleva puesta lo bastante como para poder saltarla.
    pub fn se_puede_saltar(&self) -> bool {
        self.total - self.restante >= GRACIA
    }

    /// Se salta con cualquier tecla, porque la segunda vez ya te la sabes.
    /// Devuelve al principio de la salida para que no corte en seco.
    pub fn saltar(&mut self) {
        if self.se_puede_saltar() {
            self.restante = self.restante.min(SALE);
        }
    }

    /// Cuanto se ve, de 0 a 1.
    fn opacidad(&self) -> f32 {
        let entrando = ((self.total - self.restante) / ENTRA).min(1.0);
        let saliendo = (self.restante / SALE).min(1.0);
        entrando.min(saliendo).clamp(0.0, 1.0)
    }

    pub fn dibujar(&self, l: &Layout) {
        let a = self.opacidad();
        if a <= 0.0 {
            return;
        }
        let o = l.to_screen(0.0, 0.0);
        let (w, h) = (l.len(vals_core::ARENA_W), l.len(vals_core::ARENA_H));
        let cx = o.x + w * 0.5;

        if self.para {
            draw_rectangle(o.x, o.y, w, h, fade(VELO, a));
        }

        // Lo que se abre desde el centro segun entra: es lo que hace que la
        // cartela se *presente* en vez de aparecer.
        let apertura = ((self.total - self.restante) / ENTRA).min(1.0);
        let mut y = o.y + h * (if self.para { 0.32 } else { 0.24 });

        // La orla. Solo la de entrada la lleva: la de figura salta en mitad del
        // combate y tiene que estorbar lo menos posible.
        if self.para {
            let (mw, mh) = (w * 0.36 * apertura, h * 0.115);
            let cy = y + mh * 0.45;
            for (d, grosor, alfa) in [(0.0, 2.5, 0.95), (7.0, 1.0, 0.5)] {
                draw_rectangle_lines(
                    cx - mw - d,
                    cy - mh - d,
                    (mw + d) * 2.0,
                    (mh + d) * 2.0,
                    grosor,
                    fade(SUBRAYA, a * alfa),
                );
            }
        }

        if !self.baile.is_empty() {
            fuentes::centrado(&self.baile, cx, y, 58.0, Cara::Titulo, fade(TITULO, a));
            y += 26.0;
        }

        let media = w * 0.30 * apertura;
        draw_line(cx - media, y, cx + media, y, 1.5, fade(SUBRAYA, a * 0.9));
        y += 34.0;

        if !self.figura.is_empty() {
            let tam = if self.para { 26.0 } else { 34.0 };
            let cara = if self.para {
                Cara::Cuerpo
            } else {
                Cara::Titulo
            };
            // "FIGURA PRIMERA" antes del nombre: un programa de mano numera los
            // numeros, y eso es lo que dice que esto es un espectaculo y no una
            // pantalla de carga.
            if !self.orden.is_empty() {
                fuentes::centrado(&self.orden, cx, y, 15.0, Cara::Cuerpo, fade(TENUE, a));
                y += 24.0;
            }
            fuentes::centrado(&self.figura, cx, y, tam, cara, fade(TEXTO, a));
            y += 32.0;
        }

        if !self.pieza.is_empty() {
            fuentes::centrado(&self.pieza, cx, y, 16.0, Cara::Cuerpo, fade(TENUE, a));
        }
    }
}

/// El telon entre escenas.
///
/// Es la diferencia entre un corte y un cambio. Antes era un rectangulo negro
/// que se desvanecia; ahora son **dos mitades de terciopelo que entran y
/// salen**, que cuesta lo mismo y convierte una transicion en un numero.
///
/// Corre con el reloj de pared y no con los ticks, y no es un detalle: tiene que
/// avanzar **mientras la cartela tiene el mundo parado**, que es exactamente
/// cuando no hay ticks. La primera version metia esto en el bucle de simulacion
/// y la pantalla se quedaba negra para siempre al entrar a un baile.
pub struct Fundido {
    restante: f32,
    total: f32,
}

impl Fundido {
    pub fn nuevo() -> Self {
        Self {
            restante: 0.0,
            total: 1.0,
        }
    }

    /// Arranca un telon que dura `segundos`.
    pub fn empezar(&mut self, segundos: f32) {
        self.restante = segundos;
        self.total = segundos.max(0.001);
    }

    pub fn step(&mut self, dt: f32) {
        self.restante = (self.restante - dt).max(0.0);
    }

    /// Lo cerrado que esta, de 0 a 1. Cerrado del todo al empezar y abierto al
    /// acabar.
    fn cerrado(&self) -> f32 {
        if self.restante <= 0.0 {
            return 0.0;
        }
        // Al cuadrado: sale disparado y se remansa, que es como pesa una tela.
        let t = self.restante / self.total;
        t * t
    }

    pub fn dibujar(&self) {
        let k = self.cerrado();
        if k <= 0.0 {
            return;
        }
        let (w, h) = (screen_width(), screen_height());
        let media = w * 0.5 * k;

        for (x0, sentido) in [(0.0, 1.0), (w - media, -1.0)] {
            draw_rectangle(x0, 0.0, media, h, TELON);
            // Los pliegues: unas rayas verticales bastan para que la tela deje
            // de ser un rectangulo. Se separan hacia el centro del escenario,
            // que es donde cae la luz.
            for i in 1..PLIEGUES {
                let f = i as f32 / PLIEGUES as f32;
                let x = if sentido > 0.0 {
                    media * f
                } else {
                    w - media * f
                };
                let g = 1.0 + f * 2.5;
                draw_line(x, 0.0, x, h, g, TELON_PLIEGUE);
            }
            // Y el borde interior, dorado, que es lo que dice "teatro".
            let borde = if sentido > 0.0 { media } else { w - media };
            draw_line(borde, 0.0, borde, h, 3.0, TELON_ORO);
        }
    }
}

fn fade(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_de_entrada_para_el_mundo_y_la_de_figura_no() {
        // Una cartela que hay que leer esquivando no se lee; y parar el mundo
        // en mitad de un combate que va bien le quita el ritmo. Son dos cosas
        // distintas a proposito.
        assert!(Cartela::entrada("El Vals", "El paso base", "x").para_el_mundo());
        assert!(!Cartela::figura("El espejo", 1).para_el_mundo());
    }

    #[test]
    fn se_desvanece_por_los_dos_lados_y_se_acaba() {
        let mut c = Cartela::entrada("El Vals", "El paso base", "x");
        assert!(c.opacidad() < 0.1, "empieza invisible");
        for _ in 0..24 {
            c.step(1.0 / 60.0);
        }
        assert!(c.opacidad() > 0.99, "y llega a verse entera");
        while c.step(1.0 / 60.0) {}
        assert!(c.opacidad() < 0.1, "y se va sin cortar en seco");
        assert!(!c.para_el_mundo(), "y deja de parar el mundo");
    }

    #[test]
    fn no_se_puede_saltar_nada_mas_salir() {
        // El bug que esto vigila salio jugando: "casi ni se ve, sale un instante
        // aunque no pulses nada". Se entra a un baile pulsando un boton, y ese
        // boton sigue pulsado cuando aparece la cartela.
        let mut c = Cartela::entrada("El Vals", "El paso base", "x");
        assert!(!c.se_puede_saltar(), "recien salida no");
        c.saltar();
        assert!(c.restante > SALE, "y saltarla ahi no hace nada");

        for _ in 0..(GRACIA * 60.0) as u32 + 2 {
            c.step(1.0 / 60.0);
        }
        assert!(c.se_puede_saltar(), "pasada la gracia si");
    }

    #[test]
    fn saltarla_no_la_corta_en_seco() {
        // La segunda vez ya te la sabes, pero quitarla de golpe se ve como un
        // parpadeo. Saltar deja justo la salida.
        let mut c = Cartela::entrada("El Vals", "El paso base", "x");
        // Pasada la gracia, que si no no deja.
        for _ in 0..70 {
            c.step(1.0 / 60.0);
        }
        c.saltar();
        assert!(c.restante <= SALE);
        assert!(c.restante > 0.0, "todavia tiene que desvanecerse");
    }

    #[test]
    fn el_telon_empieza_cerrado_y_acaba_abierto() {
        let mut f = Fundido::nuevo();
        assert_eq!(f.restante, 0.0, "de serie no tapa nada");
        assert_eq!(f.cerrado(), 0.0);
        f.empezar(0.3);
        assert!(f.cerrado() > 0.99, "tiene que arrancar cerrado del todo");
        for _ in 0..30 {
            f.step(1.0 / 60.0);
        }
        assert_eq!(f.restante, 0.0);
        assert_eq!(f.cerrado(), 0.0, "y acabar abierto del todo");
        // Y pasarse de pasos no lo deja en negativo.
        f.step(1.0 / 60.0);
        assert_eq!(f.restante, 0.0);
    }

    #[test]
    fn el_telon_se_abre_sin_volver_atras() {
        // Una tela que titubea se ve como un fallo. Abre y no vuelve.
        let mut f = Fundido::nuevo();
        f.empezar(0.5);
        let mut antes = f.cerrado();
        for _ in 0..40 {
            f.step(1.0 / 60.0);
            let ahora = f.cerrado();
            assert!(ahora <= antes, "el telon se ha vuelto a cerrar");
            antes = ahora;
        }
    }
}
