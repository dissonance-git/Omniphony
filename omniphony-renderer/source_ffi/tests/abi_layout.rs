use omniphony_source::{OmniphonySourceEvidenceEventV1, OmniphonySourceEvidenceV1};
use std::mem::{align_of, offset_of, size_of};

#[test]
fn source_evidence_v1_layout_matches_c_and_gmi_transport() {
    assert_eq!(size_of::<OmniphonySourceEvidenceV1>(), 72);
    assert_eq!(align_of::<OmniphonySourceEvidenceV1>(), 8);
    assert_eq!(offset_of!(OmniphonySourceEvidenceV1, lane_kind), 0);
    assert_eq!(offset_of!(OmniphonySourceEvidenceV1, flags), 4);
    assert_eq!(offset_of!(OmniphonySourceEvidenceV1, source_id), 8);
    assert_eq!(
        offset_of!(OmniphonySourceEvidenceV1, persistent_part_id),
        16
    );
    assert_eq!(offset_of!(OmniphonySourceEvidenceV1, left_gain), 24);
    assert_eq!(offset_of!(OmniphonySourceEvidenceV1, authored_x), 32);
    assert_eq!(offset_of!(OmniphonySourceEvidenceV1, foundation), 44);
    assert_eq!(offset_of!(OmniphonySourceEvidenceV1, confidence), 64);
}

#[test]
fn timed_event_v1_layout_matches_c_and_gmi_transport() {
    assert_eq!(size_of::<OmniphonySourceEvidenceEventV1>(), 80);
    assert_eq!(align_of::<OmniphonySourceEvidenceEventV1>(), 8);
    assert_eq!(offset_of!(OmniphonySourceEvidenceEventV1, frame_offset), 0);
    assert_eq!(offset_of!(OmniphonySourceEvidenceEventV1, lane_index), 4);
    assert_eq!(offset_of!(OmniphonySourceEvidenceEventV1, evidence), 8);
}
