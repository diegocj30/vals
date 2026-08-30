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

const VELO: Color = color_u8!(6, 6, 12, 235);
const TITULO: Color = color_u8!(212, 242, 255, 255);
const SUBRAYA: Color = color_u8!(255, 145, 210, 255);
const TEXTO: Color = color_u8!(150, 210, 235, 255);
const TENUE: Color = color_u8!(95, 125, 150, 255);

/// Una cartela en pantalla.
pub struct Cartela {
    baile: String,
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
            figura: figura.to_owned(),
            pieza: pieza.to_owned(),
            restante: ENTRADA,
            total: ENTRADA,
            para: true,
        }
    }

    /// La de cambio de figura. No para nada.
    pub fn figura(figura: &str) -> Self {
        Self {
            baile: String::new(),
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

    /// Se puede saltar con cualquier tecla, porque la segunda vez ya te la
    /// sabes. Devuelve al principio de la salida para que no corte en seco.
    pub fn saltar(&mut self) {
        self.restante = self.restante.min(SALE);
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

        // La raya se abre desde el centro segun entra: es lo que hace que la
        // cartela se *presente* en vez de aparecer.
        let apertura = ((self.total - self.restante) / ENTRA).min(1.0);
        let mut y = o.y + h * (if self.para { 0.34 } else { 0.24 });

        if !self.baile.is_empty() {
            fuentes::centrado(&self.baile, cx, y, 58.0, Cara::Titulo, fade(TITULO, a));
            y += 26.0;
        }

        let media = w * 0.30 * apertura;
        draw_line(cx - media, y, cx + media, y, 1.5, fade(SUBRAYA, a * 0.9));
        y += 30.0;

        if !self.figura.is_empty() {
            let tam = if self.para { 26.0 } else { 34.0 };
            let cara = if self.para {
                Cara::Cuerpo
            } else {
                Cara::Titulo
            };
            fuentes::centrado(&self.figura, cx, y, tam, cara, fade(TEXTO, a));
            y += 30.0;
        }

        if !self.pieza.is_empty() {
            fuentes::centrado(&self.pieza, cx, y, 16.0, Cara::Cuerpo, fade(TENUE, a));
        }
    }
}

/// Un fundido a negro entre escenas.
///
/// Es la diferencia entre un corte y un cambio. Cuesta veinte lineas y es de
/// las cosas que solo se notan cuando faltan.
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

    /// Arranca un fundido que dura `segundos`.
    pub fn empezar(&mut self, segundos: f32) {
        self.restante = segundos;
        self.total = segundos.max(0.001);
    }

    pub fn step(&mut self, dt: f32) {
        self.restante = (self.restante - dt).max(0.0);
    }

    pub fn dibujar(&self) {
        if self.restante <= 0.0 {
            return;
        }
        // Negro del todo al empezar y transparente al acabar.
        let a = self.restante / self.total;
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            fade(BLACK, a * a),
        );
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
        assert!(!Cartela::figura("El espejo").para_el_mundo());
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
    fn saltarla_no_la_corta_en_seco() {
        // La segunda vez ya te la sabes, pero quitarla de golpe se ve como un
        // parpadeo. Saltar deja justo la salida.
        let mut c = Cartela::entrada("El Vals", "El paso base", "x");
        for _ in 0..40 {
            c.step(1.0 / 60.0);
        }
        c.saltar();
        assert!(c.restante <= SALE);
        assert!(c.restante > 0.0, "todavia tiene que desvanecerse");
    }

    #[test]
    fn el_fundido_empieza_negro_y_acaba_limpio() {
        let mut f = Fundido::nuevo();
        assert_eq!(f.restante, 0.0, "de serie no tapa nada");
        f.empezar(0.3);
        assert!(f.restante > 0.0);
        for _ in 0..30 {
            f.step(1.0 / 60.0);
        }
        assert_eq!(f.restante, 0.0);
        // Y pasarse de pasos no lo deja en negativo.
        f.step(1.0 / 60.0);
        assert_eq!(f.restante, 0.0);
    }
}
