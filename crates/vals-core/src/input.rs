//! Input de un tick, empaquetado en bits.
//!
//! Un `InputFrame` ocupa un byte. Eso no es microoptimizacion gratuita: un
//! replay es literalmente un `Vec<InputFrame>` + una semilla, asi que dos
//! minutos de partida caben en 7 KB y se pueden guardar, versionar y comparar
//! sin pensarlo (ver hito H5).

/// Estado de los controles durante un tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct InputFrame(u8);

impl InputFrame {
    pub const UP: u8 = 1 << 0;
    pub const DOWN: u8 = 1 << 1;
    pub const LEFT: u8 = 1 << 2;
    pub const RIGHT: u8 = 1 << 3;
    /// Movimiento lento que ademas revela la hitbox. Canonico del genero.
    pub const FOCUS: u8 = 1 << 4;
    pub const SHOOT: u8 = 1 << 5;
    pub const DASH: u8 = 1 << 6;
    /// El guino a Cuphead. Se implementa en H4.
    pub const PARRY: u8 = 1 << 7;

    /// Input vacio.
    pub const NONE: Self = Self(0);

    /// Construye a partir de la mascara cruda.
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// Mascara cruda, para serializar replays.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// `true` si el boton esta pulsado.
    pub const fn is_down(self, button: u8) -> bool {
        self.0 & button != 0
    }

    /// Activa o desactiva un boton.
    pub fn set(&mut self, button: u8, down: bool) {
        if down {
            self.0 |= button;
        } else {
            self.0 &= !button;
        }
    }

    /// Direccion de movimiento en `[-1, 1]` por eje, **sin normalizar**.
    ///
    /// La normalizacion la hace el mundo, porque depende de si el jugador esta
    /// en modo focus o no.
    pub fn axis(self) -> (f32, f32) {
        let x = f32::from(self.is_down(Self::RIGHT)) - f32::from(self.is_down(Self::LEFT));
        let y = f32::from(self.is_down(Self::DOWN)) - f32::from(self.is_down(Self::UP));
        (x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_botones_opuestos_se_cancelan() {
        let f = InputFrame::from_bits(InputFrame::LEFT | InputFrame::RIGHT);
        assert_eq!(f.axis(), (0.0, 0.0));
    }

    #[test]
    fn set_y_is_down_son_coherentes() {
        let mut f = InputFrame::NONE;
        assert!(!f.is_down(InputFrame::DASH));
        f.set(InputFrame::DASH, true);
        assert!(f.is_down(InputFrame::DASH));
        f.set(InputFrame::DASH, false);
        assert!(!f.is_down(InputFrame::DASH));
    }

    #[test]
    fn un_inputframe_ocupa_un_byte() {
        assert_eq!(size_of::<InputFrame>(), 1);
    }
}
