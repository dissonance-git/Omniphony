from pathlib import Path

MOD = Path("omniphony-renderer/renderer/src/spatial_renderer/mod.rs")
CONSTRUCTION = Path("omniphony-renderer/renderer/src/spatial_renderer/construction.rs")
BINAURAL = Path("omniphony-renderer/renderer/src/binaural/mod.rs")
FIXTURE = Path("omniphony-renderer/dsp_fixtures/src/binaural_block_size.rs")

mod = MOD.read_text(encoding="utf-8")
construction = CONSTRUCTION.read_text(encoding="utf-8")
binaural = BINAURAL.read_text(encoding="utf-8")
fixture = FIXTURE.read_text(encoding="utf-8")

# 1) A fixed audio-timeline motion clock. At 48 kHz this is 40 samples, matching
# the finest existing callback fixture and therefore preserving that path as the
# canonical behaviour instead of inventing a new trajectory resolution.
const_anchor = "pub const GAIN_SLEW_SECS: f32 = 0.02;\n"
const_add = '''pub const GAIN_SLEW_SECS: f32 = 0.02;

/// Direction/HRTF updates for a moving direct-binaural object are scheduled on
/// the audio timeline, not at host callback boundaries. 1200 Hz is 40 samples
/// at 48 kHz, preserving the finest pre-fix behaviour as the canonical motion
/// resolution while larger/awkward callbacks are internally partitioned onto
/// the same clock. Static scenes never pay this partitioning cost.
const BINAURAL_MOTION_UPDATE_HZ: u32 = 1_200;
'''
if const_anchor not in mod:
    raise SystemExit("GAIN_SLEW_SECS anchor not found")
mod = mod.replace(const_anchor, const_add, 1)

field_anchor = '''    /// Sample rate for ramp time calculations
    sample_rate: u32,

    /// Distance attenuation model
'''
field_add = '''    /// Sample rate for ramp time calculations
    sample_rate: u32,

    /// Absolute sample cursor for transport-invariant DSP scheduling. Unlike
    /// `frame_counter`, this advances by PCM frames, so changing host callback
    /// size cannot re-phase the direct-binaural motion clock.
    stream_sample_cursor: u64,

    /// Distance attenuation model
'''
if field_anchor not in mod:
    raise SystemExit("sample_rate field anchor not found")
mod = mod.replace(field_anchor, field_add, 1)

reset_anchor = '''            self.binaural.reset_runtime_state();
            if let Some(cascade) = self.cascade.as_mut() {
'''
reset_add = '''            self.binaural.reset_runtime_state();
            self.stream_sample_cursor = 0;
            if let Some(cascade) = self.cascade.as_mut() {
'''
if reset_anchor not in mod:
    raise SystemExit("reset anchor not found")
mod = mod.replace(reset_anchor, reset_add, 1)

sample_anchor = '''        let sample_length = if input_channel_count > 0 {
            input_pcm.len() / input_channel_count
        } else {
            0
        };

        // Snapshot the routing once for this frame via ArcSwap: no mutex and no
'''
sample_add = '''        let sample_length = if input_channel_count > 0 {
            input_pcm.len() / input_channel_count
        } else {
            0
        };
        let frame_timeline_start = self.stream_sample_cursor;

        // Snapshot the routing once for this frame via ArcSwap: no mutex and no
'''
if sample_anchor not in mod:
    raise SystemExit("sample_length anchor not found")
mod = mod.replace(sample_anchor, sample_add, 1)

old_direct = '''            } else {
                self.binaural_pos_buf.clear();
                self.binaural_pos_buf
                    .resize(input_channel_count, [0.0, 1.0, 0.0]);
                self.binaural_gain_buf.clear();
                self.binaural_gain_buf.resize(input_channel_count, 0.0);
                self.binaural_direct_buf.clear();
                self.binaural_direct_buf.resize(input_channel_count, false);
                let num_routed = channel_routing.len();
                {
                    let states = &mut self.channel_states;
                    for c in 0..input_channel_count {
                        // Object-level mute as a 0/1 factor (per-object output gain was
                        // removed; only mute remains live-tunable).
                        let obj_gain = match self.object_params_buf.get(c) {
                            Some(o) if o.muted => 0.0,
                            _ => 1.0,
                        };
                        // Stream metadata gain, same semantics as the VBAP path:
                        // silent (-128 = -inf dB) until the first metadata arrives.
                        let gain_db = states
                            .get(c)
                            .filter(|s| s.initialized)
                            .map(|s| s.gain_db)
                            .unwrap_or(-128);
                        let gain_linear = if gain_db == -128 {
                            0.0
                        } else {
                            10.0_f32.powf(gain_db as f32 / 20.0)
                        };
                        // Slewed like the VBAP path (block-end value: the binaural
                        // stage updates per block anyway).
                        let ramp_samples = self.sample_rate as f32 * GAIN_SLEW_SECS;
                        if let Some(state) = states.get_mut(c) {
                            let (start, step) = state.slew_gain(
                                obj_gain * gain_linear,
                                sample_length,
                                ramp_samples,
                            );
                            self.binaural_gain_buf[c] = start + step * sample_length as f32;
                        } else {
                            self.binaural_gain_buf[c] = 0.0;
                        }
                        // Same direct/virtual split as the VBAP path.
                        let direct_label = match channel_routing.get(c) {
                            Some(ChannelRoute::Direct(label)) if c < num_routed => Some(*label),
                            _ => None,
                        };
                        if let Some(label) = direct_label {
                            // Direct channel: place it at its resolved speaker's
                            // direction. A channel routed to a non-spatialized
                            // speaker (the LFE) keeps the direct-routing intent in
                            // headphone mode too: fed to both ears equally, no
                            // HRTF (issue #156).
                            if let Some(&spk) = active_label_to_speaker.get(&label) {
                                if let Some(s) = active_layout.speakers.get(spk) {
                                    self.binaural_pos_buf[c] = [s.x as f64, s.y as f64, s.z as f64];
                                    self.binaural_direct_buf[c] = !s.spatialize;
                                }
                            }
                        } else if let Some(st) = states.get_mut(c) {
                            // Advance the position ramp for this block (Frame-mode
                            // granularity: the binaural stage updates HRIR/ITD once
                            // per block anyway). Nothing else advances ramps in
                            // binaural mode — the VBAP mix loop that normally does
                            // is bypassed — so without this every object stays at
                            // the ramp default [0,0,0]: dead centre, and rotation-
                            // invariant (the zero vector ignores the head pose).
                            let progress = st.ramp.current_progress().unwrap_or(RampProgress {
                                completed_units: 0,
                                total_units: 0,
                            });
                            ramp_strategy.evaluate(&mut st.ramp, progress, &ramp_context);
                            self.binaural_pos_buf[c] = st.ramp.output_position;
                            st.ramp.commit_output_position();
                            st.ramp.advance_ramp(sample_length as u64);
                        }
                    }
                }
                self.binaural.render_frame(
                    input_pcm,
                    input_channel_count,
                    sample_length,
                    &binaural_params,
                    &self.binaural_pos_buf,
                    &self.binaural_gain_buf,
                    &self.binaural_direct_buf,
                    &mut output,
                );
            }
'''
new_direct = '''            } else {
                self.binaural_pos_buf.clear();
                self.binaural_pos_buf
                    .resize(input_channel_count, [0.0, 1.0, 0.0]);
                self.binaural_gain_buf.clear();
                self.binaural_gain_buf.resize(input_channel_count, 0.0);
                self.binaural_direct_buf.clear();
                self.binaural_direct_buf.resize(input_channel_count, false);
                let num_routed = channel_routing.len();
                let motion_quantum = ((self.sample_rate + BINAURAL_MOTION_UPDATE_HZ / 2)
                    / BINAURAL_MOTION_UPDATE_HZ)
                    .max(1) as u64;
                let ramp_samples = self.sample_rate as f32 * GAIN_SLEW_SECS;
                let mut sample_offset = 0usize;

                while sample_offset < sample_length {
                    let absolute_sample = frame_timeline_start + sample_offset as u64;
                    let motion_active = self
                        .channel_states
                        .iter()
                        .take(input_channel_count)
                        .enumerate()
                        .any(|(c, state)| {
                            !matches!(channel_routing.get(c), Some(ChannelRoute::Direct(_)))
                                && state.ramp.remaining_ramp_units.is_some()
                        });

                    let mut chunk_length = sample_length - sample_offset;
                    if motion_active {
                        // The next spatial evaluation boundary is a function of
                        // absolute PCM sample time, never of the caller's block.
                        let phase = absolute_sample % motion_quantum;
                        let to_quantum = if phase == 0 {
                            motion_quantum
                        } else {
                            motion_quantum - phase
                        };
                        chunk_length = chunk_length.min(to_quantum as usize);

                        // A ramp endpoint is also an intrinsic timeline boundary.
                        // Split there so the target lands on its authored sample
                        // even when it falls between two motion quanta.
                        for (c, state) in self
                            .channel_states
                            .iter()
                            .take(input_channel_count)
                            .enumerate()
                        {
                            if matches!(channel_routing.get(c), Some(ChannelRoute::Direct(_))) {
                                continue;
                            }
                            if let Some(remaining) = state.ramp.remaining_ramp_units {
                                if remaining > 0 {
                                    chunk_length = chunk_length.min(remaining as usize);
                                }
                            }
                        }
                    }
                    chunk_length = chunk_length.max(1);
                    let at_motion_boundary = absolute_sample % motion_quantum == 0;

                    {
                        let states = &mut self.channel_states;
                        for c in 0..input_channel_count {
                            // Object-level mute as a 0/1 factor (per-object output gain was
                            // removed; only mute remains live-tunable).
                            let obj_gain = match live.object_params.get(c) {
                                Some(o) if o.muted => 0.0,
                                _ => 1.0,
                            };
                            // Stream metadata gain, same semantics as the VBAP path:
                            // silent (-128 = -inf dB) until the first metadata arrives.
                            let gain_db = states
                                .get(c)
                                .filter(|s| s.initialized)
                                .map(|s| s.gain_db)
                                .unwrap_or(-128);
                            let gain_linear = if gain_db == -128 {
                                0.0
                            } else {
                                10.0_f32.powf(gain_db as f32 / 20.0)
                            };
                            if let Some(state) = states.get_mut(c) {
                                let (start, step) = state.slew_gain(
                                    obj_gain * gain_linear,
                                    chunk_length,
                                    ramp_samples,
                                );
                                self.binaural_gain_buf[c] =
                                    start + step * chunk_length as f32;
                            } else {
                                self.binaural_gain_buf[c] = 0.0;
                            }

                            let direct_label = match channel_routing.get(c) {
                                Some(ChannelRoute::Direct(label)) if c < num_routed => Some(*label),
                                _ => None,
                            };
                            self.binaural_direct_buf[c] = false;
                            if let Some(label) = direct_label {
                                // Direct channel: fixed resolved speaker direction.
                                if let Some(&spk) = active_label_to_speaker.get(&label) {
                                    if let Some(s) = active_layout.speakers.get(spk) {
                                        self.binaural_pos_buf[c] =
                                            [s.x as f64, s.y as f64, s.z as f64];
                                        self.binaural_direct_buf[c] = !s.spatialize;
                                    }
                                }
                            } else if let Some(st) = states.get_mut(c) {
                                // Only evaluate a moving object's spatial transfer
                                // on the fixed audio-timeline clock. A zero-length
                                // ramp is an authored instantaneous target and is
                                // allowed to land immediately.
                                let progress = st.ramp.current_progress();
                                let should_evaluate = at_motion_boundary
                                    || progress.is_some_and(RampProgress::is_finished);
                                if should_evaluate {
                                    if let Some(progress) = progress {
                                        ramp_strategy.evaluate(
                                            &mut st.ramp,
                                            progress,
                                            &ramp_context,
                                        );
                                        st.ramp.commit_output_position();
                                    }
                                }
                                self.binaural_pos_buf[c] = st.ramp.current_position;
                            }
                        }
                    }

                    let input_start = sample_offset * input_channel_count;
                    let input_end = (sample_offset + chunk_length) * input_channel_count;
                    let output_start = sample_offset * 2;
                    let output_end = (sample_offset + chunk_length) * 2;
                    self.binaural.render_frame(
                        &input_pcm[input_start..input_end],
                        input_channel_count,
                        chunk_length,
                        &binaural_params,
                        &self.binaural_pos_buf,
                        &self.binaural_gain_buf,
                        &self.binaural_direct_buf,
                        &mut output[output_start..output_end],
                    );

                    // Advance authored position time by exactly the samples that
                    // were rendered. The next evaluation happens only on an
                    // intrinsic quantum/endpoint, so a host callback split cannot
                    // create an extra HRTF update.
                    for (c, state) in self
                        .channel_states
                        .iter_mut()
                        .take(input_channel_count)
                        .enumerate()
                    {
                        if !matches!(channel_routing.get(c), Some(ChannelRoute::Direct(_))) {
                            state.ramp.advance_ramp(chunk_length as u64);
                        }
                    }
                    sample_offset += chunk_length;
                }
            }
'''
if old_direct not in mod:
    raise SystemExit("direct binaural branch anchor not found")
mod = mod.replace(old_direct, new_direct, 1)

binaural_return = '''            self.apply_output_mode_fade(&mut output, 2);
            // Cascaded mode returns the virtual mix diagnostics: they index
'''
binaural_return_new = '''            self.apply_output_mode_fade(&mut output, 2);
            self.stream_sample_cursor = self
                .stream_sample_cursor
                .saturating_add(sample_length as u64);
            // Cascaded mode returns the virtual mix diagnostics: they index
'''
if binaural_return not in mod:
    raise SystemExit("binaural return cursor anchor not found")
mod = mod.replace(binaural_return, binaural_return_new, 1)

speaker_return = '''        diag.object_band_sq.sort_by_key(|(idx, _)| *idx);
        Ok(RenderedFrame {
'''
speaker_return_new = '''        diag.object_band_sq.sort_by_key(|(idx, _)| *idx);
        self.stream_sample_cursor = self
            .stream_sample_cursor
            .saturating_add(sample_length as u64);
        Ok(RenderedFrame {
'''
if speaker_return not in mod:
    raise SystemExit("speaker return cursor anchor not found")
mod = mod.replace(speaker_return, speaker_return_new, 1)

# 2) Constructor state.
ctor_anchor = '''            reset_requested: std::sync::atomic::AtomicBool::new(false),
            sample_rate,
            distance_model,
'''
ctor_new = '''            reset_requested: std::sync::atomic::AtomicBool::new(false),
            sample_rate,
            stream_sample_cursor: 0,
            distance_model,
'''
if ctor_anchor not in construction:
    raise SystemExit("constructor sample_rate anchor not found")
construction = construction.replace(ctor_anchor, ctor_new, 1)

# 3) HRIR kernel fades are measured in audio samples, not caller block length.
fade_old = '''                // Kernel changes (moving object / head) crossfade over the block
                // — capped at HRIR_LEN samples for large offline blocks — so the
                // transfer function never jumps at a block boundary (issue #155).
                let fade = sample_length.min(HRIR_LEN);
                dsp.conv_l.set_coeffs_smooth(&self.hrir_scratch.left, fade);
                dsp.conv_r.set_coeffs_smooth(&self.hrir_scratch.right, fade);
'''
fade_new = '''                // Kernel changes crossfade over one fixed HRIR-length audio
                // interval. Tying this to `sample_length` made an identical move
                // fade for 24, 40, 240, ... samples depending on where the host
                // happened to split its callback. EarConvolver carries an
                // unfinished fade across calls, so a fixed sample count is both
                // click-safe and transport invariant.
                let fade = HRIR_LEN;
                dsp.conv_l.set_coeffs_smooth(&self.hrir_scratch.left, fade);
                dsp.conv_r.set_coeffs_smooth(&self.hrir_scratch.right, fade);
'''
if fade_old not in binaural:
    raise SystemExit("HRIR fade anchor not found")
binaural = binaural.replace(fade_old, fade_new, 1)

# 4) Promote the callback-quantization reproducer to a positive contract and add
# an awkward 64-sample partition that cuts across the 40-sample motion quantum.
fixture_old = '''/// Position is the remaining half of the same portability bug family.
///
/// At present the binaural branch evaluates the canonical object ramp once at
/// the *start* of each callback, sends that single position to the HRTF stage,
/// then advances the ramp by the callback length. A 40-sample caller therefore
/// supplies many directional updates while a 960-sample caller holds the start
/// direction for the entire 20 ms move. HRTF coefficient crossfades smooth those
/// steps but cannot recover the missing sample-time trajectory.
///
/// This defect-presence test stays ignored until the parent renderer publishes a
/// real position trajectory segment. Once repaired, invert it exactly like the
/// gain gate above and remove `#[ignore]`.
#[test]
#[ignore = "known defect: binaural position/HRTF trajectory is still quantized to callback boundaries"]
fn known_defect_binaural_position_is_block_quantized() {
    let fine = render_position_motion(40);
    let medium = render_position_motion(240);
    let whole_motion = render_position_motion(TOTAL_SAMPLES);

    assert_eq!(fine.len(), medium.len());
    assert_eq!(fine.len(), whole_motion.len());

    let fine_vs_medium = peak_residual_dbfs(&fine, &medium);
    let fine_vs_whole = peak_residual_dbfs(&fine, &whole_motion);
    eprintln!(
        "known binaural motion block dependence: 40-vs-240={fine_vs_medium:.2} dBFS, 40-vs-960={fine_vs_whole:.2} dBFS"
    );

    assert!(
        fine_vs_medium > -60.0 || fine_vs_whole > -60.0,
        "the known position/HRTF block-quantization defect no longer reproduces; convert this into the positive invariance gate"
    );
}
'''
fixture_new = '''/// Position/HRTF motion belongs to the same sample timeline as metadata gain.
/// The renderer therefore uses a fixed internal motion clock (40 samples at
/// 48 kHz) and keeps that clock phase across host callbacks. An awkward
/// 64-sample callback deliberately cuts through those internal boundaries: if
/// host partitioning leaks back into HRTF scheduling, this test catches it.
#[test]
fn binaural_position_is_invariant_to_host_block_size() {
    let fine = render_position_motion(40);
    let awkward = render_position_motion(64);
    let medium = render_position_motion(240);
    let whole_motion = render_position_motion(TOTAL_SAMPLES);

    assert_eq!(fine.len(), awkward.len());
    assert_eq!(fine.len(), medium.len());
    assert_eq!(fine.len(), whole_motion.len());

    let fine_vs_awkward = peak_residual_dbfs(&fine, &awkward);
    let fine_vs_medium = peak_residual_dbfs(&fine, &medium);
    let fine_vs_whole = peak_residual_dbfs(&fine, &whole_motion);
    eprintln!(
        "binaural motion callback invariance: 40-vs-64={fine_vs_awkward:.2} dBFS, 40-vs-240={fine_vs_medium:.2} dBFS, 40-vs-960={fine_vs_whole:.2} dBFS"
    );

    const MAX_RESIDUAL_DBFS: f32 = -90.0;
    assert!(
        fine_vs_awkward <= MAX_RESIDUAL_DBFS
            && fine_vs_medium <= MAX_RESIDUAL_DBFS
            && fine_vs_whole <= MAX_RESIDUAL_DBFS,
        "binaural position/HRTF trajectory depends on host callback size: 40-vs-64={fine_vs_awkward:.2} dBFS, 40-vs-240={fine_vs_medium:.2} dBFS, 40-vs-960={fine_vs_whole:.2} dBFS (required <= {MAX_RESIDUAL_DBFS:.1})"
    );
}
'''
if fixture_old not in fixture:
    raise SystemExit("known binaural position defect fixture anchor not found")
fixture = fixture.replace(fixture_old, fixture_new, 1)

MOD.write_text(mod, encoding="utf-8")
CONSTRUCTION.write_text(construction, encoding="utf-8")
BINAURAL.write_text(binaural, encoding="utf-8")
FIXTURE.write_text(fixture, encoding="utf-8")
print("staged callback-invariant direct-binaural motion scheduling")
