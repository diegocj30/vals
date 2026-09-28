//! El equipo: el tiro y el amuleto que se llevan puestos.
//!
//! Lo vende la modista del mapa a cambio de fichas, y cambia la
//! simulacion: cuanto pega un disparo y hacia donde sale, cuantas vidas hay,
//! cuanto dura el parry. Por eso vive aqui y no en la app, y por eso **es una
//! entrada de la simulacion**, como la semilla: se pasa al construir el
//! `World` y el `Paseo`, y el replay lo guarda en su cabecera. Un replay
//! grabado con el abanico y reproducido con la aguja divergiria en el primer
//! disparo.
//!
//! **El equipo de serie es el juego de antes, bit a bit.** Para la aguja y sin
//! amuleto, cada numero de abajo es exactamente la constante que se usaba
//! antes, y el replay dorado lo demuestra sin regrabarlo.
//!
//! Todas las tablas estan aqui juntas, las del combate y las de la calle, para
//! que comparar dos tiros sea leer una pantalla y no dos ficheros.

use glam::Vec2;

use crate::player::{PARRY_WINDOW_TICKS, PLAYER_HITBOX_RADIUS, SHOT_DAMAGE, SHOT_EVERY};

/// Lo que se dispara. Uno a la vez: el que se lleva puesto.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tiro {
    /// Dos chorros rectos. El de siempre, y el unico que no se compra.
    #[default]
    Aguja,
    /// Tres a la vez en abanico, y mas despacio. De cerca pega lo mismo que la
    /// aguja; de lejos solo entra el del centro. Cubre, no pega.
    Abanico,
    /// Un golpe gordo de muy poco alcance. Pega mas que nada, pero hay que
    /// meterse encima: es el tiro de quien se atreve.
    Castanuela,
    /// Busca sola al blanco, y pega la mitad. Es el tiro de quien prefiere
    /// esquivar a apuntar.
    Serpentina,
}

/// Lo que se lleva colgado. Uno a la vez, o ninguno.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Amuleto {
    #[default]
    Ninguno,
    /// Una vida mas, en el combate y en la calle.
    Relicario,
    /// El parry se queda abierto casi el doble.
    Guante,
    /// El cuerpo que recibe es mas estrecho.
    Corse,
}

/// El tiro y el amuleto. `Default` es el juego de antes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Equipo {
    pub tiro: Tiro,
    pub amuleto: Amuleto,
}

/// Lo que se abre el abanico a cada lado, en radianes (unos once grados). A
/// 250 unidades de un jefe entran las tres; a 450, solo la del centro.
pub const ABANICO: f32 = 0.2;

/// Lo que tuerce la serpentina hacia su blanco en cada tick: se suma a su
/// direccion esta fraccion de la direccion al blanco. Poco, para que se vea
/// la curva y no un disparo que teletransporta.
pub const GIRO_SERPENTINA: f32 = 0.16;

/// Lo que encoge el corse el cuerpo que recibe.
const TALLE_CORSE: f32 = 0.6;

impl Tiro {
    pub const TODOS: [Tiro; 4] = [
        Tiro::Aguja,
        Tiro::Abanico,
        Tiro::Castanuela,
        Tiro::Serpentina,
    ];

    /// El nombre, que es tambien como se guarda (por nombre).
    pub fn nombre(self) -> &'static str {
        match self {
            Tiro::Aguja => "Aguja",
            Tiro::Abanico => "Abanico",
            Tiro::Castanuela => "Castanuela",
            Tiro::Serpentina => "Serpentina",
        }
    }

    pub fn por_nombre(nombre: &str) -> Option<Self> {
        Self::TODOS.into_iter().find(|t| t.nombre() == nombre)
    }

    /// Ticks entre rafaga y rafaga en el combate.
    pub fn cada(self) -> u32 {
        match self {
            Tiro::Aguja | Tiro::Serpentina => SHOT_EVERY,
            Tiro::Abanico | Tiro::Castanuela => 6,
        }
    }

    /// Dano de cada bala contra un jefe. La aguja echa dos por rafaga: 4
    /// cada 4 ticks. El abanico, 6 cada 6 si entran las tres; la castanuela, 7
    /// cada 6; la serpentina, 2 cada 4, justo la mitad, pero entran siempre.
    pub fn dano(self) -> i32 {
        match self {
            Tiro::Aguja | Tiro::Abanico => SHOT_DAMAGE,
            Tiro::Castanuela => 7,
            Tiro::Serpentina => 1,
        }
    }

    /// Lo que vive una bala en el combate, en segundos. Todas salen a
    /// `SHOT_SPEED`, asi que esto es el alcance: 0,4 son 360 unidades, que en
    /// el suelo solo llegan al jefe saltando.
    pub fn ttl(self) -> f32 {
        match self {
            Tiro::Castanuela => 0.4,
            _ => 2.0,
        }
    }

    /// Ticks entre disparo y disparo en la calle. Alli las vidas son de un
    /// digito y un disparo quita uno, asi que "la mitad" no se puede hacer con
    /// el dano: la serpentina la hace disparando la mitad de veces.
    pub fn cada_calle(self) -> u32 {
        match self {
            Tiro::Serpentina => SHOT_EVERY * 2,
            _ => self.cada(),
        }
    }

    /// Dano de cada disparo contra un enemigo de la calle: uno, y tres la
    /// castanuela, que dispara cada 6 en vez de cada 4.
    pub fn dano_calle(self) -> i32 {
        match self {
            Tiro::Castanuela => 3,
            _ => 1,
        }
    }

    /// Lo que vive un disparo en la calle: 0,7 son dos tercios de la vista.
    /// La castanuela, 0,3: unas 270 unidades, un salto y poco.
    pub fn ttl_calle(self) -> f32 {
        match self {
            Tiro::Castanuela => 0.3,
            _ => 0.7,
        }
    }
}

impl Amuleto {
    pub const TODOS: [Amuleto; 4] = [
        Amuleto::Ninguno,
        Amuleto::Relicario,
        Amuleto::Guante,
        Amuleto::Corse,
    ];

    pub fn nombre(self) -> &'static str {
        match self {
            Amuleto::Ninguno => "Ninguno",
            Amuleto::Relicario => "Relicario",
            Amuleto::Guante => "Guante",
            Amuleto::Corse => "Corse",
        }
    }

    pub fn por_nombre(nombre: &str) -> Option<Self> {
        Self::TODOS.into_iter().find(|a| a.nombre() == nombre)
    }

    /// Vidas de mas, en el combate y en la calle.
    pub fn vidas_extra(self) -> u32 {
        u32::from(self == Amuleto::Relicario)
    }

    /// Ticks que queda abierto el parry al pulsarlo: 12 con el guante, casi el
    /// doble de los 7 de serie. El enfriamiento no cambia, asi que no se puede
    /// parriar mas a menudo: se puede parriar antes.
    pub fn ventana_parry(self) -> u32 {
        match self {
            Amuleto::Guante => PARRY_WINDOW_TICKS + 5,
            _ => PARRY_WINDOW_TICKS,
        }
    }

    /// El radio de la hitbox del combate. El de serie se devuelve tal cual, y
    /// no multiplicado por uno, para que no haya ni que pensar si cambia algo.
    pub fn hitbox(self) -> f32 {
        match self {
            Amuleto::Corse => PLAYER_HITBOX_RADIUS * TALLE_CORSE,
            _ => PLAYER_HITBOX_RADIUS,
        }
    }

    /// Lo mismo para la capsula de la calle: su radio y su media altura.
    pub fn cuerpo(self, radio: f32, medio: f32) -> (f32, f32) {
        match self {
            Amuleto::Corse => (radio * TALLE_CORSE, medio * TALLE_CORSE),
            _ => (radio, medio),
        }
    }
}

impl Equipo {
    /// Dos bytes, para la cabecera del replay. Un numero que no se conoce cae
    /// en el de serie: un replay de una version con mas tiros no revienta, se
    /// reproduce mal y lo dicen sus huellas.
    pub fn a_bytes(self) -> [u8; 2] {
        let t = Tiro::TODOS
            .iter()
            .position(|t| *t == self.tiro)
            .unwrap_or(0);
        let a = Amuleto::TODOS
            .iter()
            .position(|a| *a == self.amuleto)
            .unwrap_or(0);
        [t as u8, a as u8]
    }

    pub fn de_bytes([t, a]: [u8; 2]) -> Self {
        Self {
            tiro: Tiro::TODOS.get(t as usize).copied().unwrap_or_default(),
            amuleto: Amuleto::TODOS.get(a as usize).copied().unwrap_or_default(),
        }
    }
}

/// Tuerce una velocidad hacia `hacia` (una direccion, no hace falta que sea
/// unitaria) sin cambiar su modulo. Es la serpentina, en el combate y en la
/// calle.
pub(crate) fn guiar(vel: Vec2, hacia: Vec2) -> Vec2 {
    let nueva = vel.normalize_or_zero() + hacia.normalize_or_zero() * GIRO_SERPENTINA;
    nueva.normalize_or_zero() * vel.length()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_equipo_de_serie_son_las_constantes_de_antes() {
        // Si esto cambia, cambia el juego sin equipo, y el dorado lo dira.
        let e = Equipo::default();
        assert_eq!(e.tiro.cada(), SHOT_EVERY);
        assert_eq!(e.tiro.dano(), SHOT_DAMAGE);
        assert_eq!(e.amuleto.ventana_parry(), PARRY_WINDOW_TICKS);
        assert_eq!(e.amuleto.hitbox(), PLAYER_HITBOX_RADIUS);
        assert_eq!(e.amuleto.vidas_extra(), 0);
        assert_eq!(e.amuleto.cuerpo(10.0, 16.0), (10.0, 16.0));
    }

    #[test]
    fn el_equipo_va_y_vuelve_por_bytes_y_por_nombre() {
        for tiro in Tiro::TODOS {
            for amuleto in Amuleto::TODOS {
                let e = Equipo { tiro, amuleto };
                assert_eq!(Equipo::de_bytes(e.a_bytes()), e);
                assert_eq!(Tiro::por_nombre(tiro.nombre()), Some(tiro));
                assert_eq!(Amuleto::por_nombre(amuleto.nombre()), Some(amuleto));
            }
        }
        assert_eq!(Equipo::de_bytes([200, 200]), Equipo::default());
    }

    #[test]
    fn guiar_tuerce_sin_acelerar() {
        let v = Vec2::new(0.0, -900.0);
        let g = guiar(v, Vec2::new(1.0, 0.0));
        assert!(g.x > 0.0, "tiene que torcer hacia el blanco");
        assert!((g.length() - 900.0).abs() < 0.01);
    }
}
