use crate::source_scene::SourceSceneEvidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourcePresentationIdentity {
    PersistentPart(u64),
    Source(u64),
}

/// Presentation continuity follows the strongest identity the source path has
/// actually earned. A persistent musical part may survive physical voice/slot
/// migration; otherwise the bounded source identity owns the renderer ramp.
pub(crate) fn source_presentation_identity(
    source: &SourceSceneEvidence,
) -> Option<SourcePresentationIdentity> {
    if let Some(id) = source.persistent_part_id.filter(|id| *id != 0) {
        return Some(SourcePresentationIdentity::PersistentPart(id));
    }
    (source.source_id != 0).then_some(SourcePresentationIdentity::Source(source.source_id))
}
