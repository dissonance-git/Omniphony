use super::{
    ChannelRampState, RampContext, RampProgress, RampStatus, RampStrategy, RampTarget,
    compute_cached_or_direct, interpolate_position, interpolate_size,
};

pub struct PositionRampStrategy;

impl RampStrategy for PositionRampStrategy {
    fn name(&self) -> &'static str {
        "position"
    }

    fn update_target(
        &self,
        state: &mut ChannelRampState,
        target: RampTarget,
        sample_index: Option<u64>,
        _ctx: &RampContext<'_>,
    ) {
        state.ramp_length = target.ramp_length;
        state.start_position = state.current_position;
        state.start_size = state.current_size;
        state.target_position = target.position;
        state.target_size = target.size;
        state.remaining_ramp_units = Some(target.ramp_length);
        state.target_sample_index = sample_index;
        state.invalidate_cache();
    }

    fn evaluate(
        &self,
        state: &mut ChannelRampState,
        progress: RampProgress,
        ctx: &RampContext<'_>,
    ) -> RampStatus {
        state.ensure_speaker_count(ctx.speaker_count());
        let fraction = progress.fraction();
        state.output_position =
            interpolate_position(state.start_position, state.target_position, fraction);
        state.current_size = interpolate_size(state.start_size, state.target_size, fraction);
        compute_cached_or_direct(state, state.output_position, state.current_size, ctx);
        if progress.is_finished() {
            RampStatus::Finished
        } else if progress.completed_units == 0 && progress.total_units == 0 {
            RampStatus::Idle
        } else {
            RampStatus::Ramping
        }
    }
}
