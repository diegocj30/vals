//! Simulacion de VALS.
//!
//! Regla de oro del proyecto: **este crate no conoce macroquad ni ninguna
//! libreria grafica**. Todo lo que hay aqui es logica pura y determinista.
//!
//! Eso compra tres cosas:
//!
//! 1. Se puede testear en headless (incluido el determinismo).
//! 2. Se puede medir con `criterion` sin ruido de GPU.
//! 3. Cambiar de renderer (miniquad puro, wgpu) es reescribir `vals-app`,
//!    no reescribir el juego.

pub mod input;
pub mod rng;
pub mod world;

pub use input::InputFrame;
pub use rng::Pcg32;
pub use world::{Player, World};

/// Frecuencia de la simulacion. El render va desacoplado e interpola.
pub const TICK_HZ: u32 = 60;

/// Duracion de un tick de simulacion, en segundos.
pub const DT: f32 = 1.0 / TICK_HZ as f32;

/// Ancho del campo de juego, en unidades logicas.
///
/// El campo es vertical, como manda el genero: da mas tiempo de reaccion a las
/// balas que bajan y hace legibles los patrones densos.
pub const ARENA_W: f32 = 640.0;

/// Alto del campo de juego, en unidades logicas.
pub const ARENA_H: f32 = 800.0;
