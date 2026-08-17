//! Compatibility shim for the retired Windows Height+ preference.
//!
//! Height is now part of the Current spatial model itself, implemented by the
//! frequency-aware coherent elevation transfer in `renderer::music_field`.
//! Keeping this zero-cost shim for one release avoids a larger realtime-ABI
//! source rewrite while ensuring the old `height-plus.txt` preference has no
//! acoustic effect. The tray no longer exposes a height switch.

pub(crate) struct HeightPreference;

impl HeightPreference {
    #[inline]
    pub(crate) fn new() -> Self {
        Self
    }

    #[inline]
    pub(crate) fn apply(&mut self, _field: &mut [f32]) {
        // Intentionally empty. Current owns its native height geometry now.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_preference_is_bit_exact_noop() {
        let mut control = HeightPreference::new();
        let mut field = [0.25f32, -0.5, 0.75, -1.0];
        let before = field;
        control.apply(&mut field);
        assert_eq!(field, before);
    }
}
