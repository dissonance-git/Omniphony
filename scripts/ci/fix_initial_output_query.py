from pathlib import Path

p = Path("omniphony-renderer/renderer/src/spatial_renderer/mod.rs")
s = p.read_text(encoding="utf-8")

old = '''    pub fn output_is_binaural(&self) -> bool {
        matches!(
            self.active_output_mode,
            crate::live_params::OutputMode::Binaural
        )
    }
'''
new = '''    pub fn output_is_binaural(&self) -> bool {
        let mode = if self.has_rendered_frame {
            self.active_output_mode
        } else {
            // Hosts may seed embedded/live configuration after construction.
            // Before any PCM exists there is no active frame geometry to
            // preserve, so queries must reflect the requested initial mode.
            self.control.live.read().binaural.output_mode
        };
        matches!(mode, crate::live_params::OutputMode::Binaural)
    }
'''
if s.count(old) != 1:
    raise SystemExit(f"output_is_binaural anchor count={s.count(old)}")
s = s.replace(old, new, 1)

old = '''    pub fn output_channel_count(&self) -> usize {
        match self.active_output_mode {
            crate::live_params::OutputMode::Binaural => 2,
            crate::live_params::OutputMode::SpeakerArray => self.num_speakers,
        }
    }
'''
new = '''    pub fn output_channel_count(&self) -> usize {
        let mode = if self.has_rendered_frame {
            self.active_output_mode
        } else {
            // Configuration is still allowed to settle before the first frame.
            // Once rendering begins, the active/fading mode owns geometry.
            self.control.live.read().binaural.output_mode
        };
        match mode {
            crate::live_params::OutputMode::Binaural => 2,
            crate::live_params::OutputMode::SpeakerArray => self.num_speakers,
        }
    }
'''
if s.count(old) != 1:
    raise SystemExit(f"output_channel_count anchor count={s.count(old)}")
s = s.replace(old, new, 1)

p.write_text(s, encoding="utf-8")
