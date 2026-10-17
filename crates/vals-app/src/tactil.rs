//! La pantalla tactil: para jugar en el navegador del movil.
//!
//! No es un aparato mas con sus teclas, porque no tiene teclas: tiene dedos
//! que caen donde caen. Asi que se reparte la pantalla segun lo que se este
//! haciendo (`Modo`), y de cada dedo sale lo mismo que saldria del teclado:
//! acciones de menu pulsadas este frame y bits de juego apretados. El resto
//! del juego no sabe que hay pantalla tactil; lo lee todo `Entrada`.
//!
//! - **Bailando o paseando**, un stick que aparece donde cae el pulgar
//!   izquierdo, en la mitad izquierda, y los botones a la derecha, en abanico
//!   alrededor del de disparar, que es el grande. Disparar **se queda puesto**
//!   al tocarlo y se quita tocandolo otra vez: con un solo pulgar a la derecha
//!   no se puede mantener disparar y saltar a la vez. Arriba a la derecha, la
//!   pausa.
//! - **En los menus**, tocar es aceptar, y arriba a la izquierda hay un boton
//!   de volver. En el titulo, tocar una de sus ayudas (el libro, la historia)
//!   hace lo que dice. En la pista el stick mueve a la bailarina por toda la
//!   pantalla, y tocar sin arrastrar entra.
//! - **En las listas** (las opciones, la tienda) tocar no acepta a ciegas:
//!   se le da el sitio tocado (`toque`) y cada una mira que fila es.
//!
//! Los botones solo se dibujan si lo ultimo que se toco fue la pantalla: con
//! el teclado o el mando en la mano, estorban.

use macroquad::prelude::*;
use vals_core::InputFrame;

use crate::acciones::{Accion, Juego};
use crate::glifos::{self, Boton};
use crate::idioma::{Texto, tx};
use crate::paleta::{PAPEL, TINTA};
use crate::protagonista::LAZO;

/// Lo que se esta haciendo, para saber que es cada dedo.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Modo {
    /// Bailando o paseando.
    Juego,
    /// La pista. `pasear` si delante hay un baile cuyo paseo ya se anduvo.
    Pista { pasear: bool },
    /// El titulo, con las ayudas que se pueden tocar y lo que hace cada una.
    Titulo(Vec<(Rect, Accion)>),
    /// Las opciones y la tienda: el sitio tocado lo mira cada una.
    Lista,
    /// Todo lo demas: tocar acepta.
    #[default]
    Menu,
}

/// Un boton de la pantalla.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tecla {
    Juego(Juego),
    Pausa,
    Atras,
    Pasear,
}

/// Un boton puesto: que es, el centro y el radio, en pixeles.
type Puesto = (Tecla, Vec2, f32);

/// La medida de todo, en pixeles: un pulgar es un pulgar, mida lo que mida la
/// lamina, asi que va con el lado corto de la pantalla y no con la escala de
/// la arena. Los numeros de abajo son para una pantalla de 540 de alto, que es
/// la del paseo.
pub fn unidad(sw: f32, sh: f32) -> f32 {
    (sw.min(sh) / 540.0).max(0.5)
}

/// El radio del stick: lo que hay que arrastrar para ir a tope.
const RADIO_STICK: f32 = 56.0;
/// Por debajo de esta fraccion del radio, el stick no mueve: el pulgar
/// apoyado tiembla.
const ZONA_MUERTA: f32 = 0.28;
/// Lo que se puede mover un dedo sin que deje de ser un toque, y lo que puede
/// durar.
const TOQUE_LEJOS: f32 = 14.0;
const TOQUE_LARGO: f64 = 0.4;

/// Los botones de cada modo.
///
/// Los de juego van en abanico alrededor de disparar, en la esquina de abajo
/// a la derecha: tres a un palmo (saltar, dash, parry) y dos algo mas lejos
/// (focus, super), que se tocan menos.
pub fn teclas(modo: &Modo, sw: f32, sh: f32) -> Vec<Puesto> {
    let u = unidad(sw, sh);
    let esquina = |dx: f32, dy: f32| vec2(sw + dx * u, sh + dy * u);
    let arriba_izq = vec2(40.0 * u, 40.0 * u);
    let arriba_dcha = vec2(sw - 40.0 * u, 40.0 * u);
    let atras = (Tecla::Atras, arriba_izq, 24.0 * u);
    let pausa = (Tecla::Pausa, arriba_dcha, 24.0 * u);
    match modo {
        Modo::Juego => {
            let disparar = esquina(-92.0, -92.0);
            let abanico = |grados: f32, lejos: f32| {
                disparar + Vec2::from_angle(grados.to_radians()) * lejos * u
            };
            vec![
                (Tecla::Juego(Juego::Disparar), disparar, 54.0 * u),
                (Tecla::Juego(Juego::Saltar), abanico(180.0, 125.0), 36.0 * u),
                (Tecla::Juego(Juego::Dash), abanico(225.0, 125.0), 36.0 * u),
                (Tecla::Juego(Juego::Parry), abanico(270.0, 125.0), 36.0 * u),
                (Tecla::Juego(Juego::Focus), abanico(200.0, 225.0), 32.0 * u),
                (Tecla::Juego(Juego::Super), abanico(250.0, 225.0), 32.0 * u),
                pausa,
            ]
        }
        Modo::Pista { pasear } => {
            let mut v = vec![atras];
            if *pasear {
                v.push((Tecla::Pasear, esquina(-84.0, -84.0), 40.0 * u));
            }
            v
        }
        Modo::Titulo(_) => vec![pausa],
        Modo::Lista | Modo::Menu => vec![atras],
    }
}

/// El boton que hay en `p`, si hay alguno. Se acepta un poco por fuera del
/// dibujo, que el dedo tapa lo que toca; si dos se solapan asi, el mas cerca.
pub fn tecla_en(puestas: &[Puesto], p: Vec2) -> Option<Tecla> {
    puestas
        .iter()
        .map(|(t, c, r)| (*t, c.distance(p) / r))
        .filter(|(_, d)| *d <= 1.2)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(t, _)| t)
}

/// Las direcciones de un stick con el centro en `centro`, el dedo en `dedo` y
/// radio `radio`: los bits de `InputFrame`.
///
/// En ocho sectores, como una cruceta: cada eje cuenta si pasa del seno de
/// 22,5 grados. Asi la diagonal sale en un cuarto de vuelta y no hay que
/// acertar con el angulo exacto.
pub fn palanca(centro: Vec2, dedo: Vec2, radio: f32) -> u16 {
    let d = dedo - centro;
    if d.length() < radio * ZONA_MUERTA {
        return 0;
    }
    let n = d.normalize();
    const SENO: f32 = 0.383;
    let mut bits = 0;
    if n.x < -SENO {
        bits |= InputFrame::LEFT;
    }
    if n.x > SENO {
        bits |= InputFrame::RIGHT;
    }
    if n.y < -SENO {
        bits |= InputFrame::UP;
    }
    if n.y > SENO {
        bits |= InputFrame::DOWN;
    }
    bits
}

/// El rectangulo ensanchado hasta `minimo` de alto: la ayuda del titulo es
/// letra pequena, y un dedo no acierta en una linea de diez pixeles.
fn ensanchar(r: Rect, minimo: f32) -> Rect {
    let mas = ((minimo - r.h) * 0.5).max(0.0);
    Rect::new(r.x - mas, r.y - mas, r.w + 2.0 * mas, r.h + 2.0 * mas)
}

/// Que es un dedo, decidido al apoyarlo.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Papel {
    /// El stick, con su centro. El centro sigue al dedo si se va mas alla del
    /// radio: asi no hay que volver al sitio donde se apoyo para cambiar de
    /// direccion.
    Stick(Vec2),
    Tecla(Tecla),
    /// Ni boton ni stick: puede acabar siendo un toque.
    Libre,
}

#[derive(Clone, Copy, Debug)]
struct Dedo {
    id: u64,
    inicio: Vec2,
    ahora: Vec2,
    desde: f64,
    /// Si se ha arrastrado: entonces ya no es un toque.
    lejos: bool,
    papel: Papel,
}

/// Un dedo en un frame, sin macroquad: lo que hace falta para los tests.
#[derive(Clone, Copy, Debug)]
pub struct Contacto {
    pub id: u64,
    pub pos: Vec2,
    /// Si se levanta este frame.
    pub fin: bool,
}

#[derive(Default)]
pub struct Tactil {
    /// Lo que se esta haciendo. Lo pone quien dibuja, al final de cada frame,
    /// y vale para el siguiente.
    pub modo: Modo,
    dedos: Vec<Dedo>,
    /// Disparar puesto.
    fijo: bool,
    // Lo de este frame.
    pulsadas: Vec<Accion>,
    bits: u16,
    toque: Option<Vec2>,
    nuevo: bool,
}

impl Tactil {
    /// Una vez por frame, con los dedos de `touches()`.
    ///
    /// Ojo: macroquad da los dedos en pixeles de la pantalla y `screen_width`
    /// en pixeles logicos. En el ordenador son lo mismo, pero en un movil hay
    /// dos o tres de los primeros por cada uno de los segundos: sin dividir,
    /// el stick (que sale donde caiga el dedo) iba, y ningun boton se acertaba.
    pub fn actualizar(&mut self) {
        let escala = macroquad::miniquad::window::dpi_scale();
        let contactos: Vec<Contacto> = touches()
            .iter()
            .map(|t| Contacto {
                id: t.id,
                pos: t.position / escala,
                fin: matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled),
            })
            .collect();
        self.paso(&contactos, get_time(), screen_width(), screen_height());
    }

    /// Lo de `actualizar`, sin ventana.
    ///
    /// Ojo con macroquad: guarda un solo estado por dedo y frame, asi que un
    /// toque rapido puede llegar ya levantado sin haberse visto apoyar, y uno
    /// que se arrastra, como movido. Por eso un dedo es nuevo por su `id`, no
    /// por su fase.
    pub fn paso(&mut self, contactos: &[Contacto], t: f64, sw: f32, sh: f32) {
        self.pulsadas.clear();
        self.toque = None;
        self.nuevo = false;
        let u = unidad(sw, sh);
        let puestas = teclas(&self.modo, sw, sh);
        let radio = RADIO_STICK * u;
        // Los dedos que ya no estan se olvidan, por si se perdio su final.
        self.dedos
            .retain(|d| contactos.iter().any(|c| c.id == d.id));
        for c in contactos {
            let i = match self.dedos.iter().position(|d| d.id == c.id) {
                Some(i) => i,
                None => {
                    self.nuevo = true;
                    let papel = self.apoyar(&puestas, c.pos, sw);
                    self.dedos.push(Dedo {
                        id: c.id,
                        inicio: c.pos,
                        ahora: c.pos,
                        desde: t,
                        lejos: false,
                        papel,
                    });
                    self.dedos.len() - 1
                }
            };
            let d = &mut self.dedos[i];
            d.ahora = c.pos;
            d.lejos |= d.inicio.distance(c.pos) > TOQUE_LEJOS * u;
            if let Papel::Stick(centro) = &mut d.papel {
                let v = c.pos - *centro;
                if v.length() > radio {
                    *centro = c.pos - v.normalize() * radio;
                }
            }
            if c.fin {
                let d = self.dedos.remove(i);
                // Un boton ya hizo lo suyo al apoyarlo.
                let boton = matches!(d.papel, Papel::Tecla(_));
                if !boton && !d.lejos && t - d.desde < TOQUE_LARGO {
                    self.tocar(c.pos, u);
                }
            }
        }
        // Lo apretado, de los dedos que siguen.
        let mut bits = 0;
        let juego = self.modo == Modo::Juego;
        for d in &self.dedos {
            match d.papel {
                Papel::Stick(centro) if juego || matches!(self.modo, Modo::Pista { .. }) => {
                    bits |= palanca(centro, d.ahora, radio);
                }
                // Se puede correr el pulgar de un boton a otro sin levantarlo.
                Papel::Tecla(Tecla::Juego(_)) if juego => {
                    if let Some(Tecla::Juego(j)) = tecla_en(&puestas, d.ahora) {
                        bits |= j.bit();
                    }
                }
                _ => {}
            }
        }
        if juego && self.fijo {
            bits |= InputFrame::SHOOT;
        }
        self.bits = bits;
    }

    /// Un dedo que se acaba de apoyar en `p`: que es.
    fn apoyar(&mut self, puestas: &[Puesto], p: Vec2, sw: f32) -> Papel {
        if let Some(k) = tecla_en(puestas, p) {
            match k {
                Tecla::Pausa => self.pulsadas.push(Accion::Pausa),
                Tecla::Atras => self.pulsadas.push(Accion::Atras),
                Tecla::Pasear => self.pulsadas.push(Accion::Pasear),
                Tecla::Juego(Juego::Disparar) => self.fijo = !self.fijo,
                Tecla::Juego(_) => {}
            }
            return Papel::Tecla(k);
        }
        match self.modo {
            Modo::Juego if p.x < sw * 0.5 => Papel::Stick(p),
            Modo::Pista { .. } => Papel::Stick(p),
            _ => Papel::Libre,
        }
    }

    /// Un toque en `p`, sin arrastrar: lo que sea segun el modo.
    fn tocar(&mut self, p: Vec2, u: f32) {
        match &self.modo {
            Modo::Juego => {}
            Modo::Lista => self.toque = Some(p),
            Modo::Titulo(zonas) => {
                let zona = zonas
                    .iter()
                    .filter(|(r, _)| ensanchar(*r, 40.0 * u).contains(p))
                    .min_by(|a, b| {
                        let d = |r: &Rect| r.center().distance(p);
                        d(&a.0).total_cmp(&d(&b.0))
                    });
                self.pulsadas
                    .push(zona.map_or(Accion::Aceptar, |(_, a)| *a));
            }
            Modo::Pista { .. } | Modo::Menu => self.pulsadas.push(Accion::Aceptar),
        }
    }

    /// Si se acaba de pulsar `a` con un dedo.
    pub fn pulsado(&self, a: Accion) -> bool {
        self.pulsadas.contains(&a)
    }

    /// Si se ha apoyado algun dedo este frame.
    pub fn algo(&self) -> bool {
        self.nuevo
    }

    /// Lo que le llega a la simulacion.
    pub fn bits(&self) -> u16 {
        self.bits
    }

    /// Donde se ha tocado este frame, en las listas.
    pub fn toque(&self) -> Option<Vec2> {
        self.toque
    }

    /// Los botones y el stick, encima de todo. Medio transparentes: el juego
    /// tiene que verse por debajo.
    pub fn dibujar(&self) {
        let (sw, sh) = (screen_width(), screen_height());
        let u = unidad(sw, sh);
        let puestas = teclas(&self.modo, sw, sh);
        let apretadas: Vec<Tecla> = self
            .dedos
            .iter()
            .filter(|d| matches!(d.papel, Papel::Tecla(_)))
            .filter_map(|d| tecla_en(&puestas, d.ahora))
            .collect();
        for (k, c, r) in &puestas {
            let b = match k {
                Tecla::Juego(j) => Boton::Toque(j.nombre()),
                Tecla::Pausa => Boton::Pausa,
                Tecla::Atras => Boton::Volver,
                Tecla::Pasear => Boton::Toque(tx(Texto::TocaPasear)),
            };
            let encendido =
                apretadas.contains(k) || *k == Tecla::Juego(Juego::Disparar) && self.fijo;
            let a = if encendido { 0.75 } else { 0.5 };
            glifos::tactil(b, *c, Vec2::splat(r * 2.0), a, encendido);
        }
        // El stick: donde esta el pulgar, o un fantasma donde suele ir para
        // que se sepa que existe.
        let stick = self.dedos.iter().find_map(|d| match d.papel {
            Papel::Stick(c) => Some((c, d.ahora)),
            _ => None,
        });
        let fantasma = matches!(self.modo, Modo::Juego | Modo::Pista { .. });
        let radio = RADIO_STICK * u;
        let (centro, seta, a) = match stick {
            Some((c, p)) => (c, c + (p - c).clamp_length_max(radio), 0.55),
            None if fantasma => {
                let c = vec2(110.0 * u, sh - 110.0 * u);
                (c, c, 0.22)
            }
            None => return,
        };
        draw_circle(
            centro.x,
            centro.y,
            radio,
            Color {
                a: a * 0.35,
                ..PAPEL
            },
        );
        draw_circle_lines(centro.x, centro.y, radio, 3.0 * u, Color { a, ..TINTA });
        draw_circle_lines(
            centro.x,
            centro.y,
            radio - 6.0 * u,
            1.5 * u,
            Color { a: a * 0.8, ..LAZO },
        );
        glifos::tactil(
            Boton::Dedo,
            seta,
            Vec2::splat(52.0 * u),
            a + 0.15,
            stick.is_some(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 960.0;
    const H: f32 = 540.0;

    fn dedo(id: u64, x: f32, y: f32) -> Contacto {
        Contacto {
            id,
            pos: vec2(x, y),
            fin: false,
        }
    }

    fn fuera(c: Contacto) -> Contacto {
        Contacto { fin: true, ..c }
    }

    #[test]
    fn el_stick_tiene_zona_muerta_y_ocho_direcciones() {
        let c = vec2(100.0, 100.0);
        let r = 50.0;
        assert_eq!(
            palanca(c, c + vec2(10.0, 5.0), r),
            0,
            "el pulgar que tiembla"
        );
        assert_eq!(palanca(c, c + vec2(40.0, 0.0), r), InputFrame::RIGHT);
        assert_eq!(palanca(c, c + vec2(0.0, -40.0), r), InputFrame::UP);
        assert_eq!(
            palanca(c, c + vec2(-30.0, 30.0), r),
            InputFrame::LEFT | InputFrame::DOWN
        );
        // Casi horizontal sigue siendo horizontal.
        assert_eq!(palanca(c, c + vec2(40.0, 10.0), r), InputFrame::RIGHT);
        // Y lejisimos sigue siendo la misma direccion.
        assert_eq!(palanca(c, c + vec2(0.0, 900.0), r), InputFrame::DOWN);
    }

    #[test]
    fn cada_boton_se_toca_en_su_sitio() {
        let puestas = teclas(&Modo::Juego, W, H);
        for (k, c, r) in &puestas {
            assert_eq!(tecla_en(&puestas, *c), Some(*k), "{k:?}");
            assert_eq!(tecla_en(&puestas, *c + vec2(r * 0.9, 0.0)), Some(*k));
            // Ninguno se pisa con otro: el centro de cada uno queda fuera de
            // los demas.
            for (o, oc, or) in &puestas {
                if o != k {
                    assert!(c.distance(*oc) > (r + or) * 1.05, "{k:?} pisa {o:?}");
                }
            }
            // Y todos caben en la pantalla, a la derecha: la izquierda es
            // del stick.
            assert!(c.x - r >= 0.0 && c.x + r <= W && c.y - r >= 0.0 && c.y + r <= H);
            if *k != Tecla::Pausa {
                assert!(c.x > W * 0.5, "{k:?}");
            }
        }
        assert_eq!(tecla_en(&puestas, vec2(W * 0.5, H * 0.5)), None);
        assert_eq!(
            tecla_en(&teclas(&Modo::Menu, W, H), vec2(40.0, 40.0)),
            Some(Tecla::Atras)
        );
    }

    #[test]
    fn el_pulgar_izquierdo_mueve_y_el_derecho_dispara() {
        let mut t = Tactil {
            modo: Modo::Juego,
            ..Default::default()
        };
        let disparar = teclas(&Modo::Juego, W, H)[0].1;
        t.paso(&[dedo(1, 150.0, 400.0)], 0.0, W, H);
        assert!(t.algo());
        assert_eq!(t.bits(), 0, "recien apoyado no mueve");
        t.paso(&[dedo(1, 220.0, 400.0)], 0.1, W, H);
        assert_eq!(t.bits(), InputFrame::RIGHT);
        // Disparar se queda puesto al soltarlo.
        let d = dedo(2, disparar.x, disparar.y);
        t.paso(&[dedo(1, 220.0, 400.0), d], 0.2, W, H);
        assert!(t.bits() & InputFrame::SHOOT != 0);
        t.paso(&[dedo(1, 220.0, 400.0), fuera(d)], 0.3, W, H);
        assert_eq!(t.bits(), InputFrame::RIGHT | InputFrame::SHOOT);
        // Y otro toque lo quita.
        t.paso(&[dedo(3, disparar.x, disparar.y)], 0.4, W, H);
        t.paso(&[fuera(dedo(3, disparar.x, disparar.y))], 0.5, W, H);
        assert_eq!(t.bits(), 0);
        // Bailando, tocar no acepta: se toca la pantalla todo el rato.
        t.paso(&[fuera(dedo(4, 600.0, 100.0))], 0.6, W, H);
        assert!(!t.pulsado(Accion::Aceptar));
    }

    #[test]
    fn en_los_menus_tocar_acepta_y_arrastrar_no() {
        let mut t = Tactil::default();
        // Un toque que llega ya levantado, sin haberse visto apoyar.
        t.paso(&[fuera(dedo(1, 300.0, 300.0))], 0.0, W, H);
        assert!(t.pulsado(Accion::Aceptar) && t.algo());
        // Un arrastre no.
        t.paso(&[dedo(2, 300.0, 300.0)], 1.0, W, H);
        t.paso(&[fuera(dedo(2, 400.0, 300.0))], 1.1, W, H);
        assert!(!t.pulsado(Accion::Aceptar));
        // Ni dejar el dedo puesto un rato.
        t.paso(&[dedo(3, 300.0, 300.0)], 2.0, W, H);
        t.paso(&[fuera(dedo(3, 300.0, 300.0))], 3.0, W, H);
        assert!(!t.pulsado(Accion::Aceptar));
        // Volver, arriba a la izquierda, va al apoyar y no acepta al soltar.
        t.paso(&[dedo(4, 40.0, 40.0)], 4.0, W, H);
        assert!(t.pulsado(Accion::Atras));
        t.paso(&[fuera(dedo(4, 40.0, 40.0))], 4.1, W, H);
        assert!(!t.pulsado(Accion::Aceptar));
        // En una lista se dice donde, y no se acepta nada.
        t.modo = Modo::Lista;
        t.paso(&[fuera(dedo(5, 300.0, 200.0))], 5.0, W, H);
        assert_eq!(t.toque(), Some(vec2(300.0, 200.0)));
        assert!(!t.pulsado(Accion::Aceptar));
    }

    #[test]
    fn en_el_titulo_se_toca_la_ayuda() {
        let libro = Rect::new(20.0, 30.0, 200.0, 12.0);
        let historia = Rect::new(20.0, 52.0, 200.0, 12.0);
        let mut t = Tactil {
            modo: Modo::Titulo(vec![(libro, Accion::Libro), (historia, Accion::Historia)]),
            ..Default::default()
        };
        // Un poco por encima de la linea tambien vale, y entre dos, la mas
        // cerca.
        t.paso(&[fuera(dedo(1, 100.0, 26.0))], 0.0, W, H);
        assert!(t.pulsado(Accion::Libro));
        t.paso(&[fuera(dedo(2, 100.0, 60.0))], 1.0, W, H);
        assert!(t.pulsado(Accion::Historia));
        t.paso(&[fuera(dedo(3, 500.0, 300.0))], 2.0, W, H);
        assert!(t.pulsado(Accion::Aceptar));
    }

    #[test]
    fn en_la_pista_el_stick_va_por_toda_la_pantalla() {
        let mut t = Tactil {
            modo: Modo::Pista { pasear: false },
            ..Default::default()
        };
        t.paso(&[dedo(1, 800.0, 300.0)], 0.0, W, H);
        t.paso(&[dedo(1, 800.0, 220.0)], 0.1, W, H);
        assert_eq!(t.bits(), InputFrame::UP);
        // El centro sigue al dedo: volver un poco basta para ir al reves.
        t.paso(&[dedo(1, 800.0, 100.0)], 0.2, W, H);
        t.paso(&[dedo(1, 800.0, 180.0)], 0.3, W, H);
        assert_eq!(t.bits(), InputFrame::DOWN);
    }
}
