//! El entreacto y las opciones.
//!
//! La pausa de un baile o de un paseo se llama **entreacto**, que es lo que
//! hay en un teatro entre un numero y el siguiente: el telon a medias, la sala
//! en penumbra y el programa de mano abierto. Se dibuja asi, como una tarjeta
//! de papel sobre la escena oscurecida.
//!
//! Lo importante no se ve: **mientras esta abierta, el mundo no avanza ni un
//! tick** (ver `pasos`). No es que vaya lento ni que se recupere luego: el
//! acumulador del paso fijo no suma, asi que al seguir no hay ticks que
//! ponerse al dia, y el replay no se entera de que hubo pausa.
//!
//! Desde el entreacto y desde el titulo se llega a las opciones, que se
//! guardan con el progreso (`Opciones`, en `guardado.rs`): la dificultad, el
//! volumen, el trazo que hierve, la sacudida, la pantalla completa y los
//! controles.

use macroquad::prelude::*;
use vals_core::Dificultad;

use crate::acciones::{
    Accion, Controles, Dispositivo, Entrada, Juego, Nombres, bit_boton, bit_tecla,
};
use crate::draw::fade;
use crate::fuentes::{self, Cara};
use crate::glifos::{self, Trozo, txg, txt};
use crate::idioma::{Idioma, Texto, tx, txf};
use crate::mando::boton;
use crate::paleta::{ORO, PAPEL, TINTA, TINTA_TENUE};

/// Lo que se guarda de las opciones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opciones {
    /// De 0 a `VOLUMEN_MAX`.
    pub musica: u8,
    pub efectos: u8,
    /// El trazo que hierve (`trazo.rs`). A quien le marea el movimiento no le
    /// sirve de nada que sea bonito.
    pub hervor: bool,
    /// La sacudida de pantalla (`zumo.rs`). El hitstop se queda: no mueve
    /// nada, solo para.
    pub sacudida: bool,
    /// Solo en nativo: en web la pantalla completa la da el navegador.
    pub pantalla_completa: bool,
    pub controles: Controles,
    /// El de la primera vez lo elige `guardado` (ver `idioma::de_primeras`).
    pub idioma: Idioma,
    /// Se aplica al empezar un baile, no a mitad: el mundo la lleva dentro.
    pub dificultad: Dificultad,
}

pub const VOLUMEN_MAX: u8 = 10;

impl Default for Opciones {
    fn default() -> Self {
        Self {
            musica: VOLUMEN_MAX,
            efectos: VOLUMEN_MAX,
            hervor: true,
            sacudida: true,
            pantalla_completa: false,
            controles: Controles::default(),
            idioma: Idioma::Es,
            dificultad: Dificultad::Normal,
        }
    }
}

impl Opciones {
    /// Lo que se guarda, como `clave=valor`. Solo lo que no es de serie: un
    /// guardado sin opciones tocadas se escribe igual que antes de que las
    /// hubiera.
    pub fn a_texto(&self) -> Vec<String> {
        let serie = Self::default();
        let mut out = Vec::new();
        let si_no = |b: bool| if b { "1" } else { "0" };
        if self.musica != serie.musica {
            out.push(format!("musica={}", self.musica));
        }
        if self.efectos != serie.efectos {
            out.push(format!("efectos={}", self.efectos));
        }
        if self.hervor != serie.hervor {
            out.push(format!("hervor={}", si_no(self.hervor)));
        }
        if self.sacudida != serie.sacudida {
            out.push(format!("sacudida={}", si_no(self.sacudida)));
        }
        if self.pantalla_completa != serie.pantalla_completa {
            out.push(format!("pantalla={}", si_no(self.pantalla_completa)));
        }
        if self.idioma != serie.idioma {
            out.push(format!("idioma={}", self.idioma.clave()));
        }
        if self.dificultad != serie.dificultad {
            out.push(format!("dificultad={}", self.dificultad.clave()));
        }
        out.extend(self.controles.a_texto());
        out
    }

    /// Lee una entrada de `a_texto`. Lo que no se entiende se ignora: una
    /// opcion de una version mas nueva, o un valor tocado a mano.
    pub fn leer(&mut self, entrada: &str) {
        let Some((clave, valor)) = entrada.split_once('=') else {
            return;
        };
        let volumen = valor.parse::<u8>().ok().filter(|v| *v <= VOLUMEN_MAX);
        let si_no = match valor {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        };
        match clave {
            "musica" => self.musica = volumen.unwrap_or(self.musica),
            "efectos" => self.efectos = volumen.unwrap_or(self.efectos),
            "hervor" => self.hervor = si_no.unwrap_or(self.hervor),
            "sacudida" => self.sacudida = si_no.unwrap_or(self.sacudida),
            "pantalla" => self.pantalla_completa = si_no.unwrap_or(self.pantalla_completa),
            "idioma" => self.idioma = Idioma::por_clave(valor).unwrap_or(self.idioma),
            "dificultad" => {
                self.dificultad = Dificultad::por_clave(valor).unwrap_or(self.dificultad)
            }
            _ => {
                self.controles.leer(clave, valor);
            }
        }
    }
}

/// Cuantos ticks de simulacion tocan este frame.
///
/// Es el paso fijo de siempre, sacado aqui para poder probar lo que importa de
/// la pausa: **parado, el acumulador no suma**, asi que no sale ningun tick y
/// al seguir no hay nada que recuperar. El mismo freno vale para el hitstop y
/// para la cartela de entrada. El tope evita la espiral de la muerte: si un
/// frame tarda muchisimo, el juego se ralentiza antes que encadenar
/// simulaciones cada vez mas largas.
pub fn pasos(acumulador: &mut f32, frame_dt: f32, parado: bool, tope: u32) -> u32 {
    if !parado {
        *acumulador += frame_dt;
    }
    let mut n = 0;
    while *acumulador >= vals_core::DT {
        *acumulador -= vals_core::DT;
        n += 1;
        if n >= tope {
            *acumulador = 0.0;
            break;
        }
    }
    n
}

/// Lo que la tarjeta le pide a la partida.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orden {
    /// Cerrar la tarjeta y seguir donde se estaba.
    Seguir,
    Reintentar,
    /// Volver a la pista.
    Mapa,
    /// Se ha movido algo: suena el tic.
    Tic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pantalla {
    Entreacto,
    Opciones,
    Controles(Dispositivo),
}

/// Una fila de la tarjeta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fila {
    Seguir,
    Reintentar,
    Mapa,
    Opciones,
    Idioma,
    Dificultad,
    Musica,
    Efectos,
    Hervor,
    Sacudida,
    PantallaCompleta,
    Controles(Dispositivo),
    Accion(Juego),
    Restaurar,
    Volver,
}

/// La tarjeta: el entreacto o, desde el titulo, las opciones a secas.
pub struct Carta {
    pantalla: Pantalla,
    cursor: usize,
    /// Abierta desde el titulo: no hay entreacto al que volver, ni partida
    /// que reintentar.
    desde_titulo: bool,
    /// Esperando la tecla o el boton nuevo para una accion.
    esperando: Option<Juego>,
    aviso: Option<(Texto, f64)>,
}

impl Carta {
    /// El entreacto de un baile o un paseo.
    pub fn entreacto() -> Self {
        Self {
            pantalla: Pantalla::Entreacto,
            cursor: 0,
            desde_titulo: false,
            esperando: None,
            aviso: None,
        }
    }

    /// Las opciones, desde el titulo.
    pub fn opciones() -> Self {
        Self {
            pantalla: Pantalla::Opciones,
            desde_titulo: true,
            ..Self::entreacto()
        }
    }

    /// Si esta esperando una tecla nueva: mientras, ninguna otra tecla del
    /// juego deberia hacer nada (la M no deberia quitar el sonido al intentar
    /// asignarla).
    pub fn capturando(&self) -> bool {
        self.esperando.is_some()
    }

    fn filas(&self) -> Vec<Fila> {
        match self.pantalla {
            Pantalla::Entreacto => vec![Fila::Seguir, Fila::Reintentar, Fila::Mapa, Fila::Opciones],
            Pantalla::Opciones => {
                // El idioma, lo primero: quien no entiende el que hay lo
                // busca sin poder leer nada mas. Y luego la dificultad, que
                // es lo que busca quien no ha jugado nunca.
                let mut v = vec![
                    Fila::Idioma,
                    Fila::Dificultad,
                    Fila::Musica,
                    Fila::Efectos,
                    Fila::Hervor,
                    Fila::Sacudida,
                ];
                if cfg!(not(target_arch = "wasm32")) {
                    v.push(Fila::PantallaCompleta);
                }
                v.extend([
                    Fila::Controles(Dispositivo::Teclado),
                    Fila::Controles(Dispositivo::Mando),
                    Fila::Volver,
                ]);
                v
            }
            Pantalla::Controles(_) => Juego::TODAS
                .into_iter()
                .map(Fila::Accion)
                .chain([Fila::Restaurar, Fila::Volver])
                .collect(),
        }
    }

    fn ir(&mut self, p: Pantalla, cursor: usize) {
        self.pantalla = p;
        self.cursor = cursor;
        self.aviso = None;
    }

    /// Un paso atras. Devuelve `Seguir` si se cierra la tarjeta.
    fn atras(&mut self) -> Orden {
        match self.pantalla {
            Pantalla::Entreacto => Orden::Seguir,
            Pantalla::Opciones if self.desde_titulo => Orden::Seguir,
            Pantalla::Opciones => {
                self.ir(Pantalla::Entreacto, 3);
                Orden::Tic
            }
            Pantalla::Controles(d) => {
                let vuelta = self
                    .filas_de(Pantalla::Opciones)
                    .iter()
                    .position(|f| *f == Fila::Controles(d))
                    .unwrap_or(0);
                self.ir(Pantalla::Opciones, vuelta);
                Orden::Tic
            }
        }
    }

    fn filas_de(&self, p: Pantalla) -> Vec<Fila> {
        Self {
            pantalla: p,
            cursor: 0,
            desde_titulo: self.desde_titulo,
            esperando: None,
            aviso: None,
        }
        .filas()
    }

    /// Un frame de entrada. Cambia `op` en el sitio; quien llama lo guarda si
    /// ha cambiado.
    pub fn entrada(&mut self, e: &Entrada, op: &mut Opciones) -> Option<Orden> {
        if let Some(j) = self.esperando {
            return self.capturar(e, op, j);
        }
        // La R y el Select reintentan tambien desde aqui: son el atajo, y el
        // entreacto es donde mas se busca.
        if e.pulsado(Accion::Reintentar) && !self.desde_titulo {
            return Some(Orden::Reintentar);
        }
        // Start con el entreacto abierto lo cierra, como en cualquier juego.
        if e.pulsado(Accion::Atras) || e.pulsado(Accion::Pausa) {
            return Some(self.atras());
        }
        let filas = self.filas();
        let n = filas.len();
        if e.pulsado(Accion::Arriba) {
            self.cursor = (self.cursor + n - 1) % n;
            return Some(Orden::Tic);
        }
        if e.pulsado(Accion::Abajo) {
            self.cursor = (self.cursor + 1) % n;
            return Some(Orden::Tic);
        }
        let fila = filas[self.cursor.min(n - 1)];
        let menos = e.pulsado(Accion::Izquierda);
        let mas = e.pulsado(Accion::Derecha);
        let aceptar = e.pulsado(Accion::Aceptar);
        let volumen = |v: &mut u8| {
            *v = if menos {
                v.saturating_sub(1)
            } else if mas {
                (*v + 1).min(VOLUMEN_MAX)
            } else {
                // Aceptar sube y da la vuelta: con un solo boton se llega a
                // todos.
                (*v + 1) % (VOLUMEN_MAX + 1)
            };
        };
        if !(menos || mas || aceptar) {
            return None;
        }
        match fila {
            Fila::Musica => volumen(&mut op.musica),
            Fila::Efectos => volumen(&mut op.efectos),
            Fila::Idioma => op.idioma = op.idioma.otro(),
            Fila::Dificultad => {
                op.dificultad = match op.dificultad {
                    Dificultad::Normal => Dificultad::Sencillo,
                    Dificultad::Sencillo => Dificultad::Normal,
                }
            }
            Fila::Hervor => op.hervor = !op.hervor,
            Fila::Sacudida => op.sacudida = !op.sacudida,
            Fila::PantallaCompleta => op.pantalla_completa = !op.pantalla_completa,
            _ if !aceptar => return None,
            Fila::Seguir => return Some(Orden::Seguir),
            Fila::Reintentar => return Some(Orden::Reintentar),
            Fila::Mapa => return Some(Orden::Mapa),
            Fila::Opciones => self.ir(Pantalla::Opciones, 0),
            Fila::Controles(d) => self.ir(Pantalla::Controles(d), 0),
            Fila::Accion(j) => self.esperando = Some(j),
            Fila::Restaurar => {
                if let Pantalla::Controles(d) = self.pantalla {
                    op.controles.restaurar(d);
                    self.aviso = Some((Texto::Restaurado, get_time()));
                }
            }
            Fila::Volver => return Some(self.atras()),
        }
        Some(Orden::Tic)
    }

    /// Esperando la tecla o el boton nuevo.
    ///
    /// Se mira el **flanco**: el boton con el que se eligio la fila sigue
    /// apretado el frame siguiente, y no puede asignarse a si mismo. ESC o
    /// Start dejan la que estaba, desde cualquiera de los dos aparatos.
    fn capturar(&mut self, e: &Entrada, op: &mut Opciones, j: Juego) -> Option<Orden> {
        let Pantalla::Controles(d) = self.pantalla else {
            self.esperando = None;
            return None;
        };
        if e.pulsado(Accion::Pausa) {
            self.esperando = None;
            return Some(Orden::Tic);
        }
        let (bit, otra) = match d {
            Dispositivo::Teclado => (e.tecla().and_then(bit_tecla), e.tecla().is_some()),
            Dispositivo::Mando => {
                let nuevos = e.mando.nuevos();
                (bit_boton(nuevos), nuevos & !boton::START != 0)
            }
        };
        match bit {
            Some(b) => {
                op.controles.asignar(d, j, b);
                self.esperando = None;
                self.aviso = None;
                Some(Orden::Tic)
            }
            None if otra => {
                self.aviso = Some((Texto::NoSePuede, get_time()));
                None
            }
            None => None,
        }
    }

    /// La tarjeta, sobre la escena oscurecida. Va en pixeles de pantalla y no
    /// en la lamina: es lo mismo sobre la arena que sobre el paseo apaisado.
    pub fn dibujar(&self, op: &Opciones, n: &Nombres, e: &Entrada) {
        let (sw, sh) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, sw, sh, fade(TINTA, 0.78));

        let u = (sh / 860.0).clamp(0.6, 1.4);
        let filas = self.filas();
        let alto_fila = 34.0 * u;
        // Arriba el titulo, en medio las filas y abajo el pie: la ayuda va
        // dentro del papel, que sobre la sala oscura competia con lo de detras.
        let (w, h) = (470.0 * u, 184.0 * u + alto_fila * filas.len() as f32);
        let (x0, y0) = ((sw - w) * 0.5, (sh - h) * 0.5 - 20.0 * u);
        let cx = sw * 0.5;

        // El papel, con su sombra de tinta corrida y la doble orla de un
        // programa de mano: gruesa fuera, fina dentro.
        draw_rectangle(x0 + 7.0 * u, y0 + 7.0 * u, w, h, fade(TINTA, 0.8));
        draw_rectangle(x0, y0, w, h, PAPEL);
        draw_rectangle_lines(x0, y0, w, h, 5.0 * u, TINTA);
        let d = 10.0 * u;
        draw_rectangle_lines(x0 + d, y0 + d, w - 2.0 * d, h - 2.0 * d, 1.5 * u, TINTA);
        for (x, y) in [
            (x0 + d, y0 + d),
            (x0 + w - d, y0 + d),
            (x0 + d, y0 + h - d),
            (x0 + w - d, y0 + h - d),
        ] {
            draw_poly(x, y, 4, 5.0 * u, 45.0, TINTA);
        }

        let titulo = match self.pantalla {
            Pantalla::Entreacto => Texto::Entreacto,
            Pantalla::Opciones => Texto::Opciones,
            Pantalla::Controles(Dispositivo::Teclado) => Texto::ElTeclado,
            Pantalla::Controles(Dispositivo::Mando) => Texto::ElMando,
        };
        let titulo = tx(titulo);
        let ty = y0 + 62.0 * u;
        fuentes::centrado(titulo, cx, ty, 40.0 * u, Cara::Titulo, TINTA);
        let ry = ty + 16.0 * u;
        let media = w * 0.3;
        draw_line(cx - media, ry, cx - 10.0 * u, ry, 1.5 * u, TINTA);
        draw_line(cx + 10.0 * u, ry, cx + media, ry, 1.5 * u, TINTA);
        draw_poly(cx, ry, 4, 5.0 * u, 45.0, TINTA);

        let playstation = e.mando.playstation();
        let mut y = ry + 40.0 * u;
        for (i, fila) in filas.iter().enumerate() {
            let elegida = i == self.cursor;
            let (texto, valor) = self.fila(*fila, op, n, playstation);
            let tinta = if elegida { PAPEL } else { TINTA };
            if elegida {
                // La elegida, en negativo: tinta llena y letra de papel, como
                // la linea que el impresor quiere que se lea primero.
                let (bx, bw) = (x0 + 26.0 * u, w - 52.0 * u);
                draw_rectangle(bx, y - 23.0 * u, bw, 31.0 * u, TINTA);
                draw_poly(bx + 14.0 * u, y - 7.5 * u, 4, 5.0 * u, 45.0, ORO);
            }
            let tam = 22.0 * u;
            fuentes::texto(texto, x0 + 48.0 * u, y, tam, Cara::Cuerpo, tinta);
            let volumen = match fila {
                Fila::Musica => Some(op.musica),
                Fila::Efectos => Some(op.efectos),
                _ => None,
            };
            if let Some(v) = volumen {
                // Una cuna de diez rayas que crecen, como la de un aparato de
                // radio: se lee de un vistazo, mejor que un numero.
                for k in 0..VOLUMEN_MAX {
                    let x = x0 + w - 44.0 * u - f32::from(VOLUMEN_MAX - k) * 11.0 * u;
                    let alto = (6.0 + f32::from(k) * 1.5) * u;
                    let (rx, ry, rw) = (x + 4.0 * u, y - alto, 7.0 * u);
                    if k < v {
                        draw_rectangle(rx, ry, rw, alto, tinta);
                    } else {
                        draw_rectangle_lines(rx, ry, rw, alto, 1.2 * u, fade(tinta, 0.6));
                    }
                }
            }
            if let Some(v) = valor {
                let esperando = elegida && self.esperando.is_some();
                let color = if esperando { ORO } else { tinta };
                let v = if esperando { txt("...") } else { v };
                glifos::derecha(&v, x0 + w - 44.0 * u, y, tam, color);
            }
            y += alto_fila;
        }

        // El pie, bajo un filete.
        let pie = match (self.esperando, self.pantalla) {
            (Some(j), Pantalla::Controles(d)) => {
                let pide = match d {
                    Dispositivo::Teclado => Texto::PulsaTecla,
                    Dispositivo::Mando => Texto::PulsaBoton,
                };
                glifos::rellenar(
                    "{}    {}",
                    &[
                        &txt(txf(pide, &[&j.nombre().to_lowercase()])),
                        &txg(Texto::Cancelar, &[&n.pausa]),
                    ],
                )
            }
            _ => {
                let mut p = txg(Texto::Pie, &[&n.aceptar, &n.atras]);
                if self.pantalla == Pantalla::Opciones
                    && matches!(
                        filas[self.cursor.min(filas.len() - 1)],
                        Fila::Musica | Fila::Efectos | Fila::Idioma | Fila::Dificultad
                    )
                {
                    p.extend(txg(Texto::PieCambiar, &[&n.mover]));
                }
                p
            }
        };
        let fy = y0 + h - 78.0 * u;
        draw_line(
            x0 + 40.0 * u,
            fy,
            x0 + w - 40.0 * u,
            fy,
            1.0 * u,
            fade(TINTA, 0.5),
        );
        let py = y0 + h - 26.0 * u;
        let ancho = w - 60.0 * u;
        glifos::encajado(&pie, cx, py, 17.0 * u, ancho, TINTA_TENUE);
        if let Some((aviso, cuando)) = self.aviso {
            let k = 1.0 - (((get_time() - cuando) as f32 - 1.6) / 0.4).clamp(0.0, 1.0);
            if k > 0.0 {
                fuentes::centrado(
                    tx(aviso),
                    cx,
                    py - 24.0 * u,
                    16.0 * u,
                    Cara::Cuerpo,
                    fade(TINTA, k),
                );
            }
        }
    }

    /// El texto de una fila y, si lo tiene, su valor a la derecha: un si o un
    /// no, el idioma, o el boton que hace eso, dibujado.
    fn fila(
        &self,
        f: Fila,
        op: &Opciones,
        n: &Nombres,
        playstation: bool,
    ) -> (&'static str, Option<Vec<Trozo>>) {
        let si_no = |b: bool| Some(txt(tx(if b { Texto::Si } else { Texto::No })));
        let (texto, valor) = match f {
            Fila::Seguir => (Texto::FilaSeguir, Some(n.pausa.clone())),
            Fila::Reintentar => (Texto::FilaReintentar, Some(n.reintentar.clone())),
            Fila::Mapa => (Texto::FilaMapa, None),
            Fila::Opciones => (Texto::FilaOpciones, None),
            Fila::Idioma => (Texto::FilaIdioma, Some(txt(op.idioma.nombre()))),
            Fila::Dificultad => {
                let d = match op.dificultad {
                    Dificultad::Normal => Texto::Normal,
                    Dificultad::Sencillo => Texto::Sencillo,
                };
                (Texto::FilaDificultad, Some(txt(tx(d))))
            }
            // El volumen se dibuja aparte, en rayas.
            Fila::Musica => (Texto::Musica, None),
            Fila::Efectos => (Texto::Efectos, None),
            Fila::Hervor => (Texto::Hervor, si_no(op.hervor)),
            Fila::Sacudida => (Texto::Sacudida, si_no(op.sacudida)),
            Fila::PantallaCompleta => (Texto::PantallaCompleta, si_no(op.pantalla_completa)),
            Fila::Controles(Dispositivo::Teclado) => (Texto::ControlesTeclado, None),
            Fila::Controles(Dispositivo::Mando) => (Texto::ControlesMando, None),
            // Las acciones ya traen su nombre traducido.
            Fila::Accion(j) => {
                let Pantalla::Controles(d) = self.pantalla else {
                    return (j.nombre(), None);
                };
                return (j.nombre(), Some(op.controles.glifo(d, j, playstation)));
            }
            Fila::Restaurar => (Texto::Restaurar, None),
            Fila::Volver => (Texto::Volver, None),
        };
        (tx(texto), valor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vals_core::DT;

    #[test]
    fn parado_no_avanza_ni_se_pone_al_dia_despues() {
        let mut acc = 0.0;
        // Diez segundos de entreacto: ni un tick.
        for _ in 0..600 {
            assert_eq!(pasos(&mut acc, 1.0 / 60.0, true, 5), 0);
        }
        // Al seguir, un frame normal da un tick como mucho, no los 600 que
        // "se debian".
        let n = pasos(&mut acc, DT, false, 5);
        assert!(n <= 1, "{n} ticks de golpe al salir de la pausa");
        // Y sin parar va al ritmo de siempre.
        let total: u32 = (0..60).map(|_| pasos(&mut acc, DT, false, 5)).sum();
        assert!((59..=61).contains(&total), "{total}");
    }

    #[test]
    fn las_opciones_se_guardan_y_se_leen() {
        let mut op = Opciones::default();
        assert!(op.a_texto().is_empty(), "lo de serie no se escribe");
        op.musica = 3;
        op.efectos = 0;
        op.hervor = false;
        op.sacudida = false;
        op.pantalla_completa = true;
        op.dificultad = Dificultad::Sencillo;
        op.controles
            .asignar(Dispositivo::Mando, Juego::Parry, boton::L3 as u64);
        let mut leido = Opciones::default();
        for e in op.a_texto() {
            leido.leer(&e);
        }
        assert_eq!(leido, op);
        // Lo que no se entiende se ignora.
        let mut raro = Opciones::default();
        for e in [
            "musica=99",
            "efectos=-1",
            "hervor=quiza",
            "dificultad=imposible",
            "brillo=5",
            "sin igual",
        ] {
            raro.leer(e);
        }
        assert_eq!(raro, Opciones::default());
    }

    #[test]
    fn desde_el_titulo_no_hay_entreacto_al_que_volver() {
        let mut c = Carta::opciones();
        assert_eq!(c.atras(), Orden::Seguir);
        let mut c = Carta::entreacto();
        c.ir(Pantalla::Controles(Dispositivo::Mando), 2);
        assert_eq!(c.atras(), Orden::Tic);
        assert_eq!(c.pantalla, Pantalla::Opciones);
        assert_eq!(c.filas()[c.cursor], Fila::Controles(Dispositivo::Mando));
        assert_eq!(c.atras(), Orden::Tic);
        assert_eq!(c.pantalla, Pantalla::Entreacto);
        assert_eq!(c.atras(), Orden::Seguir);
    }
}
