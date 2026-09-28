//! El progreso, guardado entre partidas.
//!
//! Dos caminos, como con el mando, porque tampoco aqui hay ninguno que sirva
//! para los dos:
//!
//! - **Nativo**: un fichero de texto en la carpeta de datos del usuario.
//! - **Web**: `localStorage`, por el **mismo mecanismo de plugins de miniquad**
//!   que ya usan el audio y el mando. Sin `wasm-bindgen`.
//!
//! Se guarda **por nombre y no por indice**. Un indice se rompe en cuanto se
//! reordenan los bailes o entra uno nuevo en medio: el guardado seguiria
//! cargando y diria que tienes hecho el que no es, que es la peor forma de
//! fallar. Con nombres, anadir un baile no toca lo ya guardado.
//!
//! Y con la disciplina del replay: **un guardado ilegible no revienta**, vuelve
//! al valor por defecto. Perder el progreso es malo; no arrancar es peor.
//!
//! Las fichas, las compras y el equipo van en la misma linea,
//! cada una con su prefijo (`ficha:`, `compra:`, `tiro:`, `amuleto:`). Asi la
//! version no sube: un guardado de antes se lee igual —con cero fichas y nada
//! comprado— y uno de ahora abierto por una version vieja solo le da nombres de
//! baile que no existen, que ignora.

use vals_core::Equipo;
use vals_core::equipo::{Amuleto, Tiro};

/// Cabecera del formato. Si cambia lo que se guarda **de forma que lo viejo no
/// se entienda**, sube el numero y los guardados viejos se ignoran solos.
const MAGIA: &str = "VALS1";

/// Tope de lo que se lee. Con ocho bailes y sus fichas sobra de largo (hoy no
/// pasa de 800), y pone un limite a lo que un `localStorage` manipulado a mano
/// puede meter en memoria.
const TOPE: usize = 4096;

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    /// Las pone `web/index.html`. Si faltaran, miniquad las sustituye por un
    /// aviso en consola y el juego arranca igual, sin progreso.
    fn vals_guardar(ptr: *const u8, len: u32);
    fn vals_cargar(ptr: *mut u8, cap: u32) -> i32;
}

/// Lo que sobrevive a cerrar el juego.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Guardado {
    /// Los bailes que ya se han sacado, por nombre.
    pub bailados: Vec<String>,
    /// Las fichas que ya son tuyas, como `paseo:indice`: el nombre del paseo y
    /// su sitio en la lista del RON. Por nombre, igual que los bailes.
    pub fichas: Vec<String>,
    /// Lo comprado a la modista, por nombre. Lo que se tiene en el bolsillo no
    /// se guarda: sale de restar lo comprado a lo cogido, y asi no puede
    /// desincronizarse.
    pub compras: Vec<String>,
    /// Lo que se lleva puesto.
    pub equipo: Equipo,
}

impl Guardado {
    pub fn cargar() -> Self {
        Self::desde_texto(&leer().unwrap_or_default())
    }

    pub fn guardar(&self) {
        escribir(&self.a_texto());
    }

    pub fn marcar(&mut self, nombre: &str) {
        if !self.tiene(nombre) {
            self.bailados.push(nombre.to_owned());
        }
    }

    pub fn tiene(&self, nombre: &str) -> bool {
        self.bailados.iter().any(|n| n == nombre)
    }

    pub fn tiene_ficha(&self, paseo: &str, i: usize) -> bool {
        let clave = clave_ficha(paseo, i);
        self.fichas.contains(&clave)
    }

    pub fn marcar_ficha(&mut self, paseo: &str, i: usize) {
        if !self.tiene_ficha(paseo, i) {
            self.fichas.push(clave_ficha(paseo, i));
        }
    }

    pub fn compro(&self, nombre: &str) -> bool {
        self.compras.iter().any(|c| c == nombre)
    }

    /// Una linea: la cabecera y los nombres separados por punto y coma.
    ///
    /// Es texto a proposito. Un guardado que se puede abrir con el bloc de
    /// notas se depura mirandolo, y aqui no hay nada que proteger.
    pub(crate) fn a_texto(&self) -> String {
        let mut out = String::from(MAGIA);
        let mut poner = |prefijo: &str, n: &str| {
            // El punto y coma separa, asi que un nombre que lo lleve rompería
            // el formato. Ningun baile se llama asi, pero mas vale quitarlo que
            // fiarse.
            out.push(';');
            out.push_str(prefijo);
            out.push_str(&n.replace(';', " "));
        };
        for n in &self.bailados {
            poner("", n);
        }
        for n in &self.fichas {
            poner(FICHA, n);
        }
        for n in &self.compras {
            poner(COMPRA, n);
        }
        // Lo de serie no se escribe: un guardado sin equipo sale igual que
        // antes de que lo hubiera.
        if self.equipo.tiro != Tiro::default() {
            poner(TIRO, self.equipo.tiro.nombre());
        }
        if self.equipo.amuleto != Amuleto::default() {
            poner(AMULETO, self.equipo.amuleto.nombre());
        }
        out
    }

    pub(crate) fn desde_texto(texto: &str) -> Self {
        let texto = texto.trim();
        let mut trozos = texto.split(';');
        if trozos.next() != Some(MAGIA) {
            // Ni cabecera, ni version que conozcamos, ni nada: se empieza de
            // cero en vez de adivinar.
            return Self::default();
        }
        let mut g = Self::default();
        for n in trozos.map(str::trim).filter(|n| !n.is_empty()) {
            if let Some(f) = n.strip_prefix(FICHA) {
                g.fichas.push(f.to_owned());
            } else if let Some(c) = n.strip_prefix(COMPRA) {
                g.compras.push(c.to_owned());
            } else if let Some(t) = n.strip_prefix(TIRO) {
                // Un nombre que no se conoce (de una version con mas tiros, o
                // tocado a mano) deja el de serie: mejor la aguja que nada.
                g.equipo.tiro = Tiro::por_nombre(t).unwrap_or_default();
            } else if let Some(a) = n.strip_prefix(AMULETO) {
                g.equipo.amuleto = Amuleto::por_nombre(a).unwrap_or_default();
            } else {
                g.bailados.push(n.to_owned());
            }
        }
        g
    }
}

const FICHA: &str = "ficha:";
const COMPRA: &str = "compra:";
const TIRO: &str = "tiro:";
const AMULETO: &str = "amuleto:";

fn clave_ficha(paseo: &str, i: usize) -> String {
    format!("{paseo}:{i}")
}

// ---------------------------------------------------------------------------
// Web
// ---------------------------------------------------------------------------

/// miniquad busca esta funcion para cuadrar la version del plugin con la del
/// crate. Sin ella el plugin va igual pero deja un aviso diciendo que nadie lo
/// usa, que es mentira.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vals_guardado_crate_version() -> u32 {
    1
}

#[cfg(target_arch = "wasm32")]
fn leer() -> Option<String> {
    let mut buf = [0u8; TOPE];
    // SAFETY: se le pasa un buffer propio y su capacidad; el lado de JS escribe
    // como mucho `cap` bytes y devuelve cuantos ha escrito, o negativo si no
    // habia nada guardado.
    let n = unsafe { vals_cargar(buf.as_mut_ptr(), TOPE as u32) };
    if n <= 0 {
        return None;
    }
    let n = (n as usize).min(TOPE);
    // Si lo guardado no fuese UTF-8 valido —solo puede pasar manipulandolo a
    // mano— se trata como si no hubiera nada.
    String::from_utf8(buf[..n].to_vec()).ok()
}

#[cfg(target_arch = "wasm32")]
fn escribir(texto: &str) {
    let bytes = texto.as_bytes();
    // SAFETY: puntero y longitud de un slice vivo durante la llamada.
    unsafe { vals_guardar(bytes.as_ptr(), bytes.len() as u32) };
}

// ---------------------------------------------------------------------------
// Nativo
// ---------------------------------------------------------------------------

/// Donde va el fichero.
///
/// Sin dependencias: se mira `APPDATA` en Windows y `XDG_DATA_HOME`/`HOME` en
/// lo demas. Si no hay ninguna, cae al directorio actual, que es feo pero
/// funciona y no pierde el progreso en silencio.
#[cfg(not(target_arch = "wasm32"))]
fn ruta() -> std::path::PathBuf {
    use std::path::PathBuf;

    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
        });
    match base {
        Some(b) => b.join("vals").join("progreso.txt"),
        None => PathBuf::from("vals-progreso.txt"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn leer() -> Option<String> {
    let texto = std::fs::read_to_string(ruta()).ok()?;
    Some(texto.chars().take(TOPE).collect())
}

#[cfg(not(target_arch = "wasm32"))]
fn escribir(texto: &str) {
    let ruta = ruta();
    if let Some(padre) = ruta.parent() {
        let _ = std::fs::create_dir_all(padre);
    }
    // Si no se puede escribir —disco lleno, permisos— se pierde el progreso,
    // pero el juego sigue. No hay nada util que hacer aqui.
    if let Err(e) = std::fs::write(&ruta, texto) {
        println!("[guardado] no se pudo escribir {}: {e}", ruta.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lo_guardado_se_vuelve_a_leer() {
        let mut g = Guardado::default();
        g.marcar("El Vals");
        g.marcar("El Tango");
        assert_eq!(Guardado::desde_texto(&g.a_texto()), g);
    }

    #[test]
    fn marcar_dos_veces_no_duplica() {
        let mut g = Guardado::default();
        g.marcar("El Vals");
        g.marcar("El Vals");
        assert_eq!(g.bailados.len(), 1);
        assert!(g.tiene("El Vals"));
        assert!(!g.tiene("El Tango"));
    }

    #[test]
    fn un_guardado_roto_no_revienta_y_empieza_de_cero() {
        // Perder el progreso es malo; no arrancar es peor.
        for basura in [
            "",
            "   ",
            "no soy un guardado",
            "VALS0;El Vals", // otra version
            "VALS1",         // cabecera sola: valido y vacio
            "VALS1;;;",      // separadores de mas
            ";El Vals",      // sin cabecera
        ] {
            let g = Guardado::desde_texto(basura);
            assert!(g.bailados.is_empty(), "{basura:?} deberia dar nada");
        }
    }

    #[test]
    fn el_nombre_no_puede_romper_el_formato() {
        // Ningun baile lleva punto y coma, pero si algun dia lo llevara no
        // puede partir el fichero en dos entradas.
        let mut g = Guardado::default();
        g.marcar("El Vals; y algo mas");
        let leido = Guardado::desde_texto(&g.a_texto());
        assert_eq!(leido.bailados.len(), 1);
    }

    #[test]
    fn las_fichas_las_compras_y_el_equipo_se_vuelven_a_leer() {
        let mut g = Guardado::default();
        g.marcar("El Vals");
        g.marcar_ficha("El paseo de Viena", 2);
        g.marcar_ficha("El paseo de Viena", 2);
        g.compras.push("Abanico".into());
        g.equipo.tiro = Tiro::Abanico;
        g.equipo.amuleto = Amuleto::Guante;
        let leido = Guardado::desde_texto(&g.a_texto());
        assert_eq!(leido, g);
        assert_eq!(leido.fichas.len(), 1, "una ficha es una sola vez");
        assert!(leido.tiene_ficha("El paseo de Viena", 2));
        assert!(!leido.tiene_ficha("El paseo de Viena", 0));
        assert!(leido.compro("Abanico"));
        assert!(!leido.tiene("ficha:El paseo de Viena:2"), "no es un baile");
    }

    #[test]
    fn un_guardado_de_antes_de_las_fichas_carga_sin_nada() {
        // Tal cual lo escribia la version anterior.
        let g = Guardado::desde_texto("VALS1;El Vals;El paseo de Viena");
        assert!(g.tiene("El Vals") && g.tiene("El paseo de Viena"));
        assert!(g.fichas.is_empty() && g.compras.is_empty());
        assert_eq!(g.equipo, Equipo::default());
        // Y sin fichas ni equipo se escribe igual que entonces.
        assert_eq!(g.a_texto(), "VALS1;El Vals;El paseo de Viena");
    }

    #[test]
    fn un_equipo_que_no_existe_cae_en_el_de_serie() {
        let g = Guardado::desde_texto("VALS1;tiro:Trabuco;amuleto:Herradura");
        assert_eq!(g.equipo, Equipo::default());
        assert!(g.bailados.is_empty());
    }

    #[test]
    fn se_guarda_por_nombre_y_no_por_sitio() {
        // Es lo que hace que anadir un baile nuevo no corrompa lo ya hecho: con
        // indices, meter uno en medio te cambiaria cual tienes sacado.
        let texto = {
            let mut g = Guardado::default();
            g.marcar("El Tango");
            g.a_texto()
        };
        assert!(texto.contains("El Tango"));
        assert!(Guardado::desde_texto(&texto).tiene("El Tango"));
    }
}
