//! El libro de baile: lo que queda apuntado de cada intento.
//!
//! Son tres cosas y van juntas porque son la misma informacion vista desde
//! tres sitios:
//!
//! - **Las marcas**: el mejor tiempo y la mejor nota de cada baile y de cada
//!   paseo, cuantas veces se ha intentado, en que figura se cae y cuanto se
//!   lleva jugado. Van en el guardado.
//! - **La tarjeta de resultado**, al ganar un baile o llegar a la puerta de un
//!   paseo. Es la de Cuphead, impresa como una tarjeta de baile, con la nota
//!   estampada encima.
//! - **La pagina del libro**, que se abre desde el menu.
//!
//! La nota se calcula en `vals_core::nota`, sin ventana y con sus tests; aqui
//! solo se apunta y se pinta. Nada de esto toca la simulacion: se lee de los
//! `Events`, que no entran en la huella.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use macroquad::prelude::*;
use vals_core::Events;
use vals_core::boss::BossDef;
use vals_core::nota::{self, Marcador, Nota};
use vals_core::paseo::{PaseoDef, VISTA_W};
use vals_core::{ARENA_H, ARENA_W, TICK_HZ};

use crate::acciones::Nombres;
use crate::draw::{Layout, fade, paspartu};
use crate::fuentes::{self, Cara};
use crate::glifos;
use crate::guardado::Guardado;
use crate::idioma::{Texto, nombre, tx};
use crate::jefes;
use crate::paleta::{self, PAPEL, TINTA, TINTA_TENUE};

/// Todo lo que se lee en pantalla, junto: los textos estan en `idioma`, y
/// aqui los que llevan algo de logica (plurales, huecos).
mod textos {
    use vals_core::nota::Nota;

    use crate::glifos::{Trozo, txg};
    use crate::idioma::{Texto, tx, txf};

    pub fn volver(ok: &[Trozo], otra: &[Trozo]) -> Vec<Trozo> {
        txg(Texto::TarjetaVolver, &[ok, otra])
    }
    pub fn seguir(ok: &[Trozo]) -> Vec<Trozo> {
        txg(Texto::Seguir, &[ok])
    }
    pub fn intentos(n: u32) -> String {
        txf(
            if n == 1 {
                Texto::Intento
            } else {
                Texto::Intentos
            },
            &[&n],
        )
    }
    pub fn mejor(t: &str) -> String {
        txf(Texto::Mejor, &[&t])
    }
    pub fn donde_caes(figura: &str, n: u32) -> String {
        txf(Texto::DondeCaes, &[&figura, &n])
    }
    pub fn jugado(t: &str) -> String {
        txf(Texto::Jugado, &[&t])
    }
    pub fn cerrar(atras: &[Trozo]) -> Vec<Trozo> {
        txg(Texto::CerrarLibro, &[atras])
    }
    pub fn abrir(tecla: &[Trozo]) -> Vec<Trozo> {
        txg(Texto::AbrirLibro, &[tecla])
    }
    /// Lo que hace el publico con cada nota.
    pub fn publico(n: Nota) -> &'static str {
        tx(match n {
            Nota::APlus => Texto::Ovacion,
            Nota::A => Texto::Repitan,
            Nota::B => Texto::Aplausos,
            Nota::C => Texto::Palmas,
            Nota::D => Texto::Silbidos,
        })
    }
}

/// La tinta de los sellos: el rojo de un tampon de goma. Es la misma en todos
/// los bailes a proposito: la mostaza del charleston sobre el papel no se lee.
const ROJO: Color = color_u8!(158, 40, 44, 255);

/// Ticks minimos que se ve la tarjeta del paseo antes de poder saltarla, y los
/// que tarda en seguir sola. Seguir sola es lo de siempre: el paseo lleva al
/// baile sin pedir permiso, y la tarjeta no puede convertirse en un peaje.
pub const LEER: u64 = 90;
pub const TARJETA_PASEO: u64 = 330;

/// Lo apuntado de un baile o de un paseo.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Marca {
    pub intentos: u32,
    /// El mejor tiempo ganado, en ticks.
    pub mejor: Option<u64>,
    pub nota: Option<Nota>,
    /// Golpes recibidos en cada figura. Un paseo tiene una sola.
    pub caidas: Vec<u32>,
}

impl Marca {
    /// Apunta un intento ganado. **Solo mejora**: un tiempo peor o una nota
    /// peor no borran lo que habia. Devuelve si se ha batido una marca que ya
    /// existia (la primera vez no es un record, es la primera vez).
    pub fn mejorar(&mut self, ticks: u64, nota: Option<Nota>) -> (bool, bool) {
        let tiempo = self.mejor.is_none_or(|t| ticks < t);
        let habia_tiempo = self.mejor.is_some();
        if tiempo {
            self.mejor = Some(ticks);
        }
        let mejor_nota = nota.is_some_and(|n| self.nota.is_none_or(|v| n > v));
        let habia_nota = self.nota.is_some();
        if mejor_nota {
            self.nota = nota;
        }
        (tiempo && habia_tiempo, mejor_nota && habia_nota)
    }

    /// La figura donde mas se cae y cuantas veces, si se ha caido alguna.
    fn peor_figura(&self) -> Option<(usize, u32)> {
        let (i, n) = self
            .caidas
            .iter()
            .enumerate()
            .max_by_key(|(i, n)| (**n, usize::MAX - i))?;
        (*n > 0).then_some((i, *n))
    }

    /// `nombre|i3|t4321|nB|c0.2.1`. Clave y valor y no posiciones: lo que se
    /// anada mas adelante entra sin romper lo guardado, y lo que no se conozca
    /// se ignora.
    fn a_texto(&self, nombre: &str) -> String {
        let mut s = nombre.replace('|', " ");
        let _ = write!(s, "|i{}", self.intentos);
        if let Some(t) = self.mejor {
            let _ = write!(s, "|t{t}");
        }
        if let Some(n) = self.nota {
            let _ = write!(s, "|n{}", n.letra());
        }
        if !self.caidas.is_empty() {
            let c: Vec<String> = self.caidas.iter().map(u32::to_string).collect();
            let _ = write!(s, "|c{}", c.join("."));
        }
        s
    }

    fn desde_texto(s: &str) -> Option<(String, Self)> {
        let mut trozos = s.split('|');
        let nombre = trozos.next().filter(|n| !n.is_empty())?.to_owned();
        let mut m = Self::default();
        for t in trozos {
            let Some((k, v)) = t.split_at_checked(1) else {
                continue;
            };
            // Un numero ilegible se queda en lo de serie: mejor perder una
            // marca que el guardado entero.
            match k {
                "i" => m.intentos = v.parse().unwrap_or(0),
                "t" => m.mejor = v.parse().ok(),
                "n" => m.nota = Nota::por_letra(v),
                "c" => m.caidas = v.split('.').map(|x| x.parse().unwrap_or(0)).collect(),
                _ => {}
            }
        }
        Some((nombre, m))
    }
}

/// Todas las marcas, por nombre como el resto del guardado.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Marcas {
    pub por_nombre: BTreeMap<String, Marca>,
    /// Ticks bailados o paseados, en total. Los menus no cuentan.
    pub jugado: u64,
}

impl Marcas {
    /// Empieza un intento de `nombre`, y lo cuenta.
    pub fn empezar(&mut self, nombre: &str) -> Intento {
        self.marca(nombre).intentos += 1;
        Intento {
            nombre: nombre.to_owned(),
            ..Default::default()
        }
    }

    pub fn nota(&self, nombre: &str) -> Option<Nota> {
        self.por_nombre.get(nombre)?.nota
    }

    fn marca(&mut self, nombre: &str) -> &mut Marca {
        self.por_nombre.entry(nombre.to_owned()).or_default()
    }

    /// Una entrada del guardado por marca, sin el prefijo.
    pub(crate) fn lineas(&self) -> impl Iterator<Item = String> + '_ {
        self.por_nombre.iter().map(|(n, m)| m.a_texto(n))
    }

    pub(crate) fn leer(&mut self, linea: &str) {
        if let Some((n, m)) = Marca::desde_texto(linea) {
            self.por_nombre.insert(n, m);
        }
    }
}

/// El intento en curso: su marcador y, al acabar, su tarjeta.
#[derive(Default)]
pub struct Intento {
    nombre: String,
    pub marcador: Marcador,
    /// Saltado con F3. Se ve la tarjeta, pero no cuenta para las marcas: si
    /// no, el mejor tiempo de todos seria el de la trampa.
    pub trampa: bool,
    pub tarjeta: Option<Tarjeta>,
}

impl Intento {
    /// Un tick bailado: al marcador, al tiempo jugado y, si ha habido golpe, a
    /// la figura donde ha sido.
    pub fn apuntar(&mut self, marcas: &mut Marcas, e: &Events, figura: usize) {
        self.marcador.sumar(e);
        marcas.jugado += 1;
        if e.player_died && !self.nombre.is_empty() {
            let c = &mut marcas.marca(&self.nombre).caidas;
            if c.len() <= figura {
                c.resize(figura + 1, 0);
            }
            c[figura] += 1;
        }
    }

    /// Baile ganado: su nota, sus marcas y su tarjeta.
    pub fn cerrar_baile(&mut self, marcas: &mut Marcas) {
        let n = nota::nota(&self.marcador, nota::par(&self.nombre));
        self.cerrar(marcas, Some(n), None);
    }

    /// Paseo andado hasta la puerta, con las fichas que son tuyas de las que
    /// tiene.
    pub fn cerrar_paseo(&mut self, marcas: &mut Marcas, fichas: (usize, usize)) {
        self.cerrar(marcas, None, Some(fichas));
    }

    fn cerrar(&mut self, marcas: &mut Marcas, nota: Option<Nota>, fichas: Option<(usize, usize)>) {
        let (record, _) = if self.trampa {
            (false, false)
        } else {
            marcas
                .marca(&self.nombre)
                .mejorar(self.marcador.ticks, nota)
        };
        self.tarjeta = Some(Tarjeta {
            marcador: self.marcador,
            nota,
            record,
            fichas,
            desde: ahora(),
        });
    }
}

/// Lo que se ensena al acabar.
pub struct Tarjeta {
    marcador: Marcador,
    nota: Option<Nota>,
    /// Si se ha batido el mejor tiempo que habia.
    record: bool,
    fichas: Option<(usize, usize)>,
    /// Reloj de pared al ganar: la tarjeta se presenta sola, sin estado.
    desde: f64,
}

impl Tarjeta {
    fn edad(&self) -> f32 {
        (ahora() - self.desde) as f32
    }

    /// Si la del baile ya se ha leido lo bastante para dejar seguir. Sin esto,
    /// la Z que se lleva machacando para disparar se la saltaba sin verla.
    pub fn lista(&self) -> bool {
        self.edad() > jefes::CAIDA + 1.6
    }
}

/// El reloj de pared. El de miniquad y no `get_time`, que pide la ventana y
/// los tests no la tienen.
fn ahora() -> f64 {
    macroquad::miniquad::date::now()
}

/// Minutos y segundos, con la decima: "1:42.3".
fn reloj(ticks: u64) -> String {
    let hz = u64::from(TICK_HZ);
    let s = ticks / hz;
    format!("{}:{:02}.{}", s / 60, s % 60, ticks % hz * 10 / hz)
}

/// El tiempo jugado, en horas y minutos.
fn horas(ticks: u64) -> String {
    let min = ticks / u64::from(TICK_HZ) / 60;
    match min / 60 {
        0 => format!("{min} min"),
        h => format!("{h} h {:02} min", min % 60),
    }
}

/// Un sello de tampon con la nota: dos aros y la letra, en rojo.
///
/// `fondo` lo hace macizo, rojo con la letra en papel, para cuando no se
/// estampa sobre papel (en el mapa va sobre la tarima). `golpe` es de 0 a 1 lo que lleva de
/// caer: llega grande y transparente y se aplasta en su sitio, que es lo que
/// hace que se lea como un golpe de tampon y no como un texto que aparece.
pub fn sello(c: Vec2, r: f32, n: Nota, fondo: bool, golpe: f32) {
    if golpe <= 0.0 {
        return;
    }
    let k = 1.0 - golpe.min(1.0);
    let r = r * (1.0 + 1.4 * k * k);
    let a = golpe.min(1.0);
    let letra = n.letra();
    if fondo {
        // Sobre la tarima, macizo: un aro fino no se leeria a este tamano.
        draw_circle(c.x, c.y, r + 1.5, fade(TINTA, a));
        draw_circle(c.x, c.y, r, fade(ROJO, a));
        draw_circle_lines(c.x, c.y, r * 0.8, 1.0, fade(PAPEL, a * 0.8));
        let tam = r * if letra.len() > 1 { 1.0 } else { 1.25 };
        fuentes::centrado(
            letra,
            c.x,
            c.y + tam * 0.36,
            tam,
            Cara::Cuerpo,
            fade(PAPEL, a),
        );
        return;
    }
    draw_circle_lines(c.x, c.y, r * 0.92, r * 0.11, fade(ROJO, a * 0.9));
    draw_circle_lines(c.x, c.y, r * 0.72, r * 0.04, fade(ROJO, a * 0.8));
    let tam = r * if letra.len() > 1 { 0.95 } else { 1.15 };
    // Dos pasadas separadas: la tinta de un tampon carga, y la Barlow sola a
    // este tamano quedaba fina.
    for dx in [0.0, r * 0.03] {
        fuentes::centrado(
            letra,
            c.x + dx,
            c.y + tam * 0.36,
            tam,
            Cara::Cuerpo,
            fade(ROJO, a),
        );
    }
}

/// Una tarjeta de papel con su titulo y filas de "nombre ..... valor".
/// Devuelve la `y` de debajo de la ultima fila. Todo en unidades de lamina.
fn tarjeta(l: &Layout, r: Rect, titulo: &str, sub: &str, filas: &[(&str, String)], a: f32) -> f32 {
    let p = |x: f32, y: f32| l.to_screen(x, y);
    let (o, w, h) = (p(r.x, r.y), l.len(r.w), l.len(r.h));
    draw_rectangle(
        o.x + l.len(6.0),
        o.y + l.len(6.0),
        w,
        h,
        fade(TINTA, a * 0.45),
    );
    draw_rectangle(o.x, o.y, w, h, fade(PAPEL, a));
    draw_rectangle_lines(o.x, o.y, w, h, l.len(3.0), fade(TINTA, a));
    let m = l.len(7.0);
    draw_rectangle_lines(
        o.x + m,
        o.y + m,
        w - 2.0 * m,
        h - 2.0 * m,
        l.len(1.0),
        fade(TINTA, a * 0.6),
    );

    let cx = r.x + r.w * 0.5;
    let s = p(cx, r.y + 42.0);
    fuentes::centrado(titulo, s.x, s.y, l.len(24.0), Cara::Titulo, fade(TINTA, a));
    let mut y = r.y + 52.0;
    if !sub.is_empty() {
        let s = p(cx, y + 14.0);
        fuentes::centrado(
            sub,
            s.x,
            s.y,
            l.len(14.0),
            Cara::Cuerpo,
            fade(TINTA_TENUE, a),
        );
        y += 18.0;
    }
    let (i, d) = (p(r.x + 30.0, y + 6.0), p(r.x + r.w - 30.0, y + 6.0));
    draw_line(i.x, i.y, d.x, d.y, l.len(1.5), fade(TINTA, a));
    y += 44.0;
    // Las filas ocupan la izquierda: la derecha es del sello.
    let fin = r.x + if filas.len() > 3 { 250.0 } else { r.w - 40.0 };
    for (nombre, valor) in filas {
        let s = p(r.x + 34.0, y);
        fuentes::texto(nombre, s.x, s.y, l.len(19.0), Cara::Cuerpo, fade(TINTA, a));
        let ancho_n = fuentes::medir(nombre, l.len(19.0), Cara::Cuerpo).width;
        let ancho_v = fuentes::medir(valor, l.len(19.0), Cara::Cuerpo).width;
        // Los puntos de un programa de mano, del nombre al valor.
        let (x0, x1) = (
            s.x + ancho_n + l.len(8.0),
            p(fin, y).x - ancho_v - l.len(8.0),
        );
        let mut x = x0;
        while x < x1 {
            draw_circle(x, s.y - l.len(3.0), l.len(1.0), fade(TINTA_TENUE, a));
            x += l.len(7.0);
        }
        let s = p(fin, y);
        fuentes::derecha(valor, s.x, s.y, l.len(19.0), Cara::Cuerpo, fade(TINTA, a));
        y += 36.0;
    }
    y
}

/// La tarjeta de un baile ganado, encima del sello de victoria. Espera a que
/// el sello este puesto, y la nota se estampa un momento despues: primero se
/// leen las cuentas y luego cae el veredicto, como en Cuphead.
pub fn tarjeta_de_baile(t: &Tarjeta, l: &Layout, teclas: &Nombres) {
    let edad = t.edad() - jefes::CAIDA;
    let a = jefes::suave(jefes::tramo(edad, 0.3, 0.7));
    if a <= 0.0 {
        return;
    }
    let r = Rect::new(110.0, 300.0 + 20.0 * (1.0 - a), 420.0, 300.0);
    let m = &t.marcador;
    let mut tiempo = reloj(m.ticks);
    if t.record {
        tiempo = format!("{}  {tiempo}", tx(Texto::Record));
    }
    let filas = [
        (tx(Texto::Tiempo), tiempo),
        (tx(Texto::Golpes), m.golpes.to_string()),
        (tx(Texto::Parries), m.parries.to_string()),
        (tx(Texto::Supers), m.supers.to_string()),
    ];
    tarjeta(l, r, tx(Texto::TarjetaDeBaile), "", &filas, a);
    if let Some(n) = t.nota {
        let c = l.to_screen(r.x + 345.0, r.y + 165.0);
        sello(c, l.len(52.0), n, false, jefes::tramo(edad, 1.1, 1.3));
        let k = jefes::tramo(edad, 1.4, 1.8);
        let s = l.to_screen(r.x + 345.0, r.y + 240.0);
        let tam = l.len(15.0);
        fuentes::centrado(
            textos::publico(n),
            s.x,
            s.y,
            tam,
            Cara::Cuerpo,
            fade(ROJO, k),
        );
    }
    if t.lista() {
        let s = l.to_screen(r.x + r.w * 0.5, r.y + r.h - 22.0);
        let ayuda = textos::volver(&teclas.aceptar, &teclas.reintentar);
        glifos::encajado(
            &ayuda,
            s.x,
            s.y,
            l.len(15.0),
            l.len(r.w - 24.0),
            TINTA_TENUE,
        );
    }
}

/// La tarjeta de la puerta de un paseo: menos cosas, y sin nota.
pub fn tarjeta_de_paseo(t: &Tarjeta, nombre: &str, l: &Layout, teclas: &Nombres) {
    let edad = t.edad();
    let a = jefes::suave(jefes::tramo(edad, 0.2, 0.6));
    if a <= 0.0 {
        return;
    }
    let (w, h) = (380.0, 260.0);
    let r = Rect::new((VISTA_W - w) * 0.5, 120.0 + 16.0 * (1.0 - a), w, h);
    let m = &t.marcador;
    let mut tiempo = reloj(m.ticks);
    if t.record {
        tiempo = format!("{}  {tiempo}", tx(Texto::Record));
    }
    let (tuyas, total) = t.fichas.unwrap_or_default();
    let filas = [
        (tx(Texto::Tiempo), tiempo),
        (tx(Texto::FichasFila), format!("{tuyas} / {total}")),
        (tx(Texto::Golpes), m.golpes.to_string()),
    ];
    let y = tarjeta(l, r, nombre, tx(Texto::HastaLaPuerta), &filas, a);
    if edad > LEER as f32 / TICK_HZ as f32 {
        let s = l.to_screen(r.x + w * 0.5, y + 2.0);
        let ayuda = textos::seguir(&teclas.aceptar);
        glifos::centrado(&ayuda, s.x, s.y, l.len(15.0), TINTA_TENUE);
    }
}

/// Lo que dice el menu para abrir el libro: arriba a la izquierda, que es lo
/// unico que el modo atractor deja libre (el centro es del titulo y de ella, y
/// arriba a la derecha va el nombre de la figura).
pub fn aviso_en_el_menu(l: &Layout, teclas: &Nombres) {
    // Se abre con la L, o con el gatillo de focus: "L1" en PlayStation, "LB"
    // en Xbox.
    let s = l.to_screen(24.0, 40.0);
    let texto = textos::abrir(&teclas.libro);
    glifos::texto(&texto, s.x, s.y, l.len(16.0), fade(PAPEL, 0.8));
}

/// La pagina del libro: cada baile con su sello, su mejor tiempo, sus
/// intentos y donde se cae; cada paseo con lo suyo; y el tiempo jugado.
pub fn pagina(l: &Layout, g: &Guardado, bailes: &[BossDef], paseos: &[PaseoDef], teclas: &Nombres) {
    clear_background(PAPEL);
    let p = |x: f32, y: f32| l.to_screen(x, y);
    let o = p(0.0, 0.0);
    draw_rectangle(o.x, o.y, l.len(ARENA_W), l.len(ARENA_H), PAPEL);
    let a = p(20.0, 20.0);
    draw_rectangle_lines(
        a.x,
        a.y,
        l.len(600.0),
        l.len(760.0),
        l.len(1.2),
        TINTA_TENUE,
    );

    let s = p(320.0, 78.0);
    fuentes::centrado(
        tx(Texto::ElLibro),
        s.x,
        s.y,
        l.len(46.0),
        Cara::Titulo,
        TINTA,
    );
    let s = p(320.0, 104.0);
    fuentes::centrado(
        tx(Texto::Temporada),
        s.x,
        s.y,
        l.len(15.0),
        Cara::Cuerpo,
        TINTA_TENUE,
    );

    let seccion = |y: f32, titulo: &str| {
        let s = p(320.0, y);
        fuentes::centrado(titulo, s.x, s.y, l.len(19.0), Cara::Titulo, TINTA);
        // Los filetes, hasta el titulo y no mas: "THE PROMENADES" no mide lo
        // que "LOS BAILES".
        let media = fuentes::medir(titulo, l.len(19.0), Cara::Titulo).width / l.len(1.0) * 0.5;
        let hueco = (media + 14.0).max(70.0);
        for (x0, x1) in [(50.0, 320.0 - hueco), (320.0 + hueco, 590.0)] {
            let (i, d) = (p(x0, y - 6.0), p(x1, y - 6.0));
            draw_line(i.x, i.y, d.x, d.y, l.len(1.0), TINTA);
        }
    };

    seccion(150.0, tx(Texto::LosBailes));
    let vacia = Marca::default();
    for (i, def) in bailes.iter().enumerate() {
        let y = 176.0 + i as f32 * 92.0;
        let m = g.marcas.por_nombre.get(&def.name).unwrap_or(&vacia);
        let c = p(78.0, y + 34.0);
        match m.nota {
            Some(n) => sello(c, l.len(28.0), n, false, 1.0),
            None => draw_circle_lines(c.x, c.y, l.len(25.0), l.len(1.0), TINTA_TENUE),
        }
        let (tinta, _) = paleta::del_baile(i);
        let s = p(124.0, y + 26.0);
        // El nombre, con un filete en la tinta de su baile debajo.
        fuentes::texto(
            nombre(&def.name),
            s.x,
            s.y,
            l.len(27.0),
            Cara::Titulo,
            TINTA,
        );
        let ancho = fuentes::medir(nombre(&def.name), l.len(27.0), Cara::Titulo).width;
        draw_line(
            s.x,
            s.y + l.len(5.0),
            s.x + ancho,
            s.y + l.len(5.0),
            l.len(2.5),
            tinta,
        );

        let resumen = match m.mejor {
            Some(t) => format!(
                "{}   ·   {}",
                textos::mejor(&reloj(t)),
                textos::intentos(m.intentos)
            ),
            None if m.intentos > 0 => textos::intentos(m.intentos),
            None => tx(Texto::SinBailar).to_owned(),
        };
        let s = p(124.0, y + 50.0);
        fuentes::texto(&resumen, s.x, s.y, l.len(16.0), Cara::Cuerpo, TINTA);
        let caes = match m.peor_figura() {
            Some((f, n)) => {
                let figura = def.phases.get(f).map_or("", |ph| nombre(&ph.name));
                textos::donde_caes(figura, n)
            }
            None if m.intentos > 0 => tx(Texto::SinCaidas).to_owned(),
            None => String::new(),
        };
        let s = p(124.0, y + 70.0);
        fuentes::texto(&caes, s.x, s.y, l.len(14.0), Cara::Cuerpo, TINTA_TENUE);
        caidas(l, m, def.phases.len(), vec2(470.0, y + 14.0));
    }

    let y0 = 176.0 + bailes.len() as f32 * 92.0 + 22.0;
    seccion(y0, tx(Texto::LosPaseos));
    for (i, def) in paseos.iter().enumerate() {
        let y = y0 + 34.0 + i as f32 * 40.0;
        let m = g.marcas.por_nombre.get(&def.nombre).unwrap_or(&vacia);
        let s = p(60.0, y);
        fuentes::texto(
            nombre(&def.nombre),
            s.x,
            s.y,
            l.len(18.0),
            Cara::Cuerpo,
            TINTA,
        );
        let tuyas = (0..def.fichas.len())
            .filter(|k| g.tiene_ficha(&def.nombre, *k))
            .count();
        let detalle = match m.mejor {
            Some(t) => format!(
                "{}   ·   {} {tuyas}/{}   ·   {}",
                textos::mejor(&reloj(t)),
                tx(Texto::FichasFila).to_lowercase(),
                def.fichas.len(),
                textos::intentos(m.intentos)
            ),
            None if m.intentos > 0 => textos::intentos(m.intentos),
            None => tx(Texto::SinAndar).to_owned(),
        };
        let s = p(580.0, y);
        fuentes::derecha(&detalle, s.x, s.y, l.len(14.0), Cara::Cuerpo, TINTA_TENUE);
    }

    let s = p(48.0, 756.0);
    let jugado = textos::jugado(&horas(g.marcas.jugado));
    fuentes::texto(&jugado, s.x, s.y, l.len(15.0), Cara::Cuerpo, TINTA);
    let s = p(592.0, 756.0);
    let ayuda = textos::cerrar(&teclas.atras);
    glifos::derecha(&ayuda, s.x, s.y, l.len(15.0), TINTA_TENUE);

    paspartu(l);
}

/// Los golpes por figura, en columnas de tinta: donde se cae se ve de lejos,
/// sin leer numeros. La columna mas alta es la de la figura que mas cuesta.
fn caidas(l: &Layout, m: &Marca, figuras: usize, esquina: Vec2) {
    let max = m.caidas.iter().copied().max().unwrap_or(0).max(1) as f32;
    let (ancho, alto) = (22.0, 46.0);
    for f in 0..figuras {
        let n = m.caidas.get(f).copied().unwrap_or(0);
        let x = esquina.x + f as f32 * (ancho + 8.0);
        let h = alto * n as f32 / max;
        let base = l.to_screen(x, esquina.y + alto);
        let arriba = l.to_screen(x, esquina.y + alto - h);
        draw_rectangle(arriba.x, arriba.y, l.len(ancho), base.y - arriba.y, TINTA);
        draw_line(
            base.x - l.len(2.0),
            base.y,
            base.x + l.len(ancho + 2.0),
            base.y,
            l.len(1.0),
            TINTA,
        );
        let s = l.to_screen(x + ancho * 0.5, esquina.y + alto + 16.0);
        fuentes::centrado(
            &n.to_string(),
            s.x,
            s.y,
            l.len(13.0),
            Cara::Cuerpo,
            TINTA_TENUE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_marcas_solo_mejoran() {
        let mut m = Marca::default();
        // La primera vez se apunta, pero no es un record: no habia nada.
        assert_eq!(m.mejorar(5000, Some(Nota::C)), (false, false));
        assert_eq!((m.mejor, m.nota), (Some(5000), Some(Nota::C)));
        // Peor en las dos: no se toca nada.
        assert_eq!(m.mejorar(6000, Some(Nota::D)), (false, false));
        assert_eq!((m.mejor, m.nota), (Some(5000), Some(Nota::C)));
        // Mejor tiempo y peor nota: cada una por su lado.
        assert_eq!(m.mejorar(4000, Some(Nota::D)), (true, false));
        assert_eq!((m.mejor, m.nota), (Some(4000), Some(Nota::C)));
        assert_eq!(m.mejorar(4500, Some(Nota::A)), (false, true));
        assert_eq!((m.mejor, m.nota), (Some(4000), Some(Nota::A)));
        // Un paseo no trae nota y no borra la que hubiera.
        m.mejorar(9000, None);
        assert_eq!(m.nota, Some(Nota::A));
    }

    #[test]
    fn se_apuntan_intentos_golpes_por_figura_y_tiempo() {
        let mut marcas = Marcas::default();
        let mut i = marcas.empezar("El Vals");
        let golpe = Events {
            player_died: true,
            ..Default::default()
        };
        i.apuntar(&mut marcas, &golpe, 2);
        i.apuntar(&mut marcas, &Events::default(), 2);
        i.apuntar(&mut marcas, &golpe, 0);
        marcas.empezar("El Vals");
        let m = &marcas.por_nombre["El Vals"];
        assert_eq!(m.intentos, 2);
        assert_eq!(m.caidas, [1, 0, 1]);
        assert_eq!(m.peor_figura(), Some((0, 1)), "empate: la primera");
        assert_eq!(marcas.jugado, 3);
        assert_eq!(i.marcador.golpes, 2);
    }

    #[test]
    fn la_trampa_no_cuenta_para_las_marcas() {
        let mut marcas = Marcas::default();
        let mut i = marcas.empezar("El Vals");
        i.trampa = true;
        i.apuntar(&mut marcas, &Events::default(), 0);
        i.cerrar_baile(&mut marcas);
        assert!(i.tarjeta.is_some(), "la tarjeta se ve igual");
        assert_eq!(marcas.nota("El Vals"), None);
        assert_eq!(marcas.por_nombre["El Vals"].mejor, None);
    }

    #[test]
    fn una_marca_se_vuelve_a_leer_y_lo_roto_no_revienta() {
        let m = Marca {
            intentos: 7,
            mejor: Some(6123),
            nota: Some(Nota::APlus),
            caidas: vec![0, 3, 1],
        };
        let texto = m.a_texto("El Vals");
        assert_eq!(texto, "El Vals|i7|t6123|nA+|c0.3.1");
        assert_eq!(Marca::desde_texto(&texto), Some(("El Vals".into(), m)));
        // Claves de una version futura, numeros rotos y nombres vacios.
        let (_, rara) = Marca::desde_texto("El Tango|i2|zzz|tabc|nQ|cx.4|ñ").unwrap();
        assert_eq!(rara.intentos, 2);
        assert_eq!((rara.mejor, rara.nota), (None, None));
        assert_eq!(rara.caidas, [0, 4]);
        assert_eq!(Marca::desde_texto("|i3"), None);
    }

    #[test]
    fn el_reloj_y_las_horas() {
        assert_eq!(reloj(0), "0:00.0");
        assert_eq!(reloj(60 * 102 + 18), "1:42.3");
        assert_eq!(horas(60 * 60 * 59), "59 min");
        assert_eq!(horas(60 * 60 * 72), "1 h 12 min");
    }
}
