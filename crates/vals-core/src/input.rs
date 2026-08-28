//! Input de un tick, empaquetado en bits.
//!
//! Un `InputFrame` ocupa dos bytes. Eso no es microoptimizacion gratuita: un
//! replay es literalmente un `Vec<InputFrame>` mas una semilla, asi que dos
//! minutos de partida caben en 14 KB y se pueden guardar, versionar y comparar
//! sin pensarlo (ver hito H5).
//!
//! Empezo siendo un byte con ocho botones. Al llegar el super en H4 se acabaron
//! los bits, y ampliar a `u16` fue mas honesto que buscarle un hueco raro o
//! meter combinaciones de teclas: sigue siendo trivial de serializar y deja
//! sitio para siete botones mas.

/// Estado de los controles durante un tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct InputFrame(u16);

impl InputFrame {
    pub const UP: u16 = 1 << 0;
    pub const DOWN: u16 = 1 << 1;
    pub const LEFT: u16 = 1 << 2;
    pub const RIGHT: u16 = 1 << 3;
    /// Movimiento lento que ademas revela la hitbox. Canonico del genero.
    pub const FOCUS: u16 = 1 << 4;
    pub const SHOOT: u16 = 1 << 5;
    pub const DASH: u16 = 1 << 6;
    /// El guino a Cuphead: neutraliza las balas rosas.
    pub const PARRY: u16 = 1 << 7;
    /// Descarga el medidor que llenan el parry y el graze.
    pub const SUPER: u16 = 1 << 8;
    /// Salto. Solo hace algo en el modo con gravedad.
    pub const JUMP: u16 = 1 << 9;

    /// Input vacio.
    pub const NONE: Self = Self(0);

    /// Construye a partir de la mascara cruda.
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    /// Mascara cruda, para serializar replays.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// `true` si el boton esta pulsado.
    pub const fn is_down(self, button: u16) -> bool {
        self.0 & button != 0
    }

    /// Activa o desactiva un boton.
    pub fn set(&mut self, button: u16, down: bool) {
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
    fn un_inputframe_ocupa_dos_bytes() {
        assert_eq!(size_of::<InputFrame>(), 2);
    }

    #[test]
    fn los_botones_son_bits_distintos() {
        let todos = [
            InputFrame::UP,
            InputFrame::DOWN,
            InputFrame::LEFT,
            InputFrame::RIGHT,
            InputFrame::FOCUS,
            InputFrame::SHOOT,
            InputFrame::DASH,
            InputFrame::PARRY,
            InputFrame::SUPER,
            InputFrame::JUMP,
        ];
        let mut vistos = 0u16;
        for b in todos {
            assert_eq!(vistos & b, 0, "bit repetido: {b:#b}");
            vistos |= b;
        }
        assert_eq!(vistos.count_ones(), 10);
    }
}
