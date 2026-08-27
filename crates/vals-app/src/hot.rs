//! Recarga en caliente del patron del jefe.
//!
//! Guardas el RON y el cambio entra sin recompilar. Es la funcion que convierte
//! disenar un patron en algo que se itera en segundos en vez de en minutos, y
//! por eso estaba en el plan desde el principio: un proyecto que se toca a
//! rachas no puede permitirse un ciclo de prueba lento.
//!
//! Se implementa comprobando la fecha del fichero cada pocos frames, no con un
//! observador del sistema de ficheros. Es una dependencia menos y un hilo
//! menos, y para un fichero que edita una persona a mano sobra de largo.
//!
//! En web no hay sistema de ficheros: alli el patron es el que quedo embebido
//! en el binario y esto se convierte en un no-op.

use vals_core::boss::BossDef;

/// Ruta del patron, resuelta al compilar.
///
/// Relativa al manifiesto y no al directorio de trabajo, para que funcione
/// tanto con `cargo run` desde la raiz como lanzando el ejecutable a pelo.
#[cfg(not(target_arch = "wasm32"))]
const RUTA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/patterns/boss1.ron"
);

/// Cuantos frames se muestra el aviso de recarga.
const AVISO_FRAMES: u32 = 180;

pub struct HotReload {
    #[cfg(not(target_arch = "wasm32"))]
    last: Option<std::time::SystemTime>,
    aviso: Option<(String, bool)>,
    aviso_frames: u32,
}

impl HotReload {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            last: None,
            aviso: None,
            aviso_frames: 0,
        }
    }

    /// Mensaje a mostrar y si es un error.
    pub fn aviso(&self) -> Option<(&str, bool)> {
        self.aviso.as_ref().map(|(m, e)| (m.as_str(), *e))
    }

    /// Comprueba si el fichero cambio. Devuelve el jefe nuevo si parsea bien.
    ///
    /// Si el RON tiene un error de sintaxis **no se recarga nada**: se avisa y
    /// se sigue jugando con la ultima version buena. Que un parentesis mal
    /// puesto te eche del juego a media sesion de afinado seria lo contrario
    /// de lo que busca esta funcion.
    pub fn poll(&mut self) -> Option<BossDef> {
        self.aviso_frames = self.aviso_frames.saturating_sub(1);
        if self.aviso_frames == 0 {
            self.aviso = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            None
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let modificado = std::fs::metadata(RUTA).and_then(|m| m.modified()).ok()?;
            if self.last == Some(modificado) {
                return None;
            }
            let primera_vez = self.last.is_none();
            self.last = Some(modificado);
            if primera_vez {
                // Solo se toma nota de la fecha inicial; no es una recarga.
                return None;
            }

            match std::fs::read_to_string(RUTA) {
                Ok(src) => match BossDef::from_ron(&src) {
                    Ok(def) => {
                        self.set_aviso(format!("patron recargado: {}", def.name), false);
                        Some(def)
                    }
                    Err(e) => {
                        self.set_aviso(format!("RON invalido: {e}"), true);
                        None
                    }
                },
                Err(e) => {
                    self.set_aviso(format!("no se pudo leer el patron: {e}"), true);
                    None
                }
            }
        }
    }

    fn set_aviso(&mut self, msg: String, error: bool) {
        // Tambien por consola: el banner se ve mientras juegas, pero al afinar
        // un patron sueles tener la terminal al lado, y ahi queda el historial
        // de recargas y de errores de sintaxis.
        println!("[hot-reload] {msg}");
        self.aviso = Some((msg, error));
        self.aviso_frames = AVISO_FRAMES;
    }
}
