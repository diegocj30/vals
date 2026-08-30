//! Las dos tipografias del juego.
//!
//! Hasta ahora todo el texto salia con la bitmap por defecto de macroquad, y
//! era **lo que mas delataba que esto lo habia hecho una persona en su casa**.
//! Un juego se juzga por el texto antes que por las balas: es lo primero que se
//! lee y lo unico que esta en pantalla todo el rato.
//!
//! - **Poiret One** para titulos y cartelas: art deco, fina y con aire de
//!   salon. Es la cara del juego.
//! - **Barlow** para todo lo demas: una grotesca que se lee a 14 pixeles, que
//!   es lo que la deco no hace.
//!
//! Las dos con licencia SIL OFL, embebidas con `include_bytes!` y montadas con
//! `load_ttf_font_from_bytes`. macroquad ya trae `fontdue`, asi que esto
//! funciona igual en nativo y en web sin dependencias nuevas.
//!
//! **Rompen el "cero assets" del proyecto**, y es a proposito: una tipografia
//! no es arte del juego, es una herramienta, como el compilador. Todo lo que se
//! dibuja —balas, personajes, escenografia— lo sigue generando el codigo.
//!
//! # Por que un global y no un parametro
//!
//! Las fuentes viven en un `thread_local`. La alternativa era pasar un
//! `&Fuentes` por las diez funciones de dibujo y por sus ayudantes, para que
//! todas hicieran exactamente lo mismo con el. macroquad es de un solo hilo
//! —tiene un contexto global de OpenGL—, asi que aqui no se gana nada
//! explicitando lo que no puede variar.

use std::cell::RefCell;

use macroquad::prelude::*;
use macroquad::text::{Font, TextDimensions, load_ttf_font_from_bytes};

const POIRET: &[u8] = include_bytes!("../../../assets/fonts/PoiretOne-Regular.ttf");
const BARLOW: &[u8] = include_bytes!("../../../assets/fonts/Barlow-Regular.ttf");

/// Cual de las dos.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cara {
    /// Poiret One. Para titulos, cartelas y nombres.
    Titulo,
    /// Barlow. Para el HUD, la ayuda y todo lo que hay que leer rapido.
    Cuerpo,
}

thread_local! {
    static FUENTES: RefCell<Option<(Font, Font)>> = const { RefCell::new(None) };
}

/// Monta las dos caras. Se llama una vez al arrancar.
///
/// Si alguna no cargase —un `.ttf` corrupto—, se sigue con la bitmap de
/// macroquad en vez de no arrancar. El juego se ve peor; el juego se ve.
pub fn cargar() {
    let titulo = load_ttf_font_from_bytes(POIRET);
    let cuerpo = load_ttf_font_from_bytes(BARLOW);
    match (titulo, cuerpo) {
        (Ok(t), Ok(c)) => {
            FUENTES.with(|f| *f.borrow_mut() = Some((t, c)));
            println!("[fuentes] Poiret One y Barlow montadas");
        }
        (t, c) => {
            println!(
                "[fuentes] no se pudieron montar (titulo ok: {}, cuerpo ok: {}); \
                 se sigue con la de macroquad",
                t.is_ok(),
                c.is_ok()
            );
        }
    }
}

/// Ejecuta `f` con la cara pedida, o con `None` si no hay fuentes montadas.
fn con<R>(cara: Cara, f: impl FnOnce(Option<&Font>) -> R) -> R {
    FUENTES.with(|celda| {
        let prestado = celda.borrow();
        let elegida = prestado.as_ref().map(|(t, c)| match cara {
            Cara::Titulo => t,
            Cara::Cuerpo => c,
        });
        f(elegida)
    })
}

/// Dibuja texto. Sustituye a `draw_text`.
pub fn texto(s: &str, x: f32, y: f32, tam: f32, cara: Cara, color: Color) {
    con(cara, |fuente| {
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font: fuente,
                font_size: tam as u16,
                color,
                ..Default::default()
            },
        );
    });
}

/// Mide texto. Sustituye a `measure_text`.
pub fn medir(s: &str, tam: f32, cara: Cara) -> TextDimensions {
    con(cara, |fuente| measure_text(s, fuente, tam as u16, 1.0))
}

/// Dibuja texto centrado en `cx`. Es lo que hace media interfaz del juego.
pub fn centrado(s: &str, cx: f32, y: f32, tam: f32, cara: Cara, color: Color) {
    let m = medir(s, tam, cara);
    texto(s, cx - m.width * 0.5, y, tam, cara, color);
}

/// Dibuja texto pegado a la derecha de `dx`.
pub fn derecha(s: &str, dx: f32, y: f32, tam: f32, cara: Cara, color: Color) {
    let m = medir(s, tam, cara);
    texto(s, dx - m.width, y, tam, cara, color);
}
