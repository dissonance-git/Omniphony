//! Output channel-layout export.
//!
//! Maps each output speaker (by its layout name) to an ABI-stable
//! [`RChannelLabel`], so a host like mpv can turn the renderer's output into a
//! channel map (`mp_chmap`). Names come from the user-editable layout YAML, so
//! the matcher is alias-tolerant and case-insensitive; anything unrecognised
//! maps to [`RChannelLabel::Unknown`] (the host then falls back to a custom
//! order or a plain count).

use bridge_api::RChannelLabel;

/// Resolve one speaker name to its channel label. Case-insensitive; tolerates
/// the common naming variants found across the bundled layouts and hand-edited
/// configs. Returns [`RChannelLabel::Unknown`] for names it can't place.
pub fn label_for_speaker_name(name: &str) -> RChannelLabel {
    // Normalise: uppercase, drop spaces/underscores/hyphens so "Top Front Left",
    // "TOP_FRONT_LEFT" and "TFL" all collapse to the same key.
    let key: String = name
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .flat_map(|c| c.to_uppercase())
        .collect();

    use RChannelLabel::*;
    match key.as_str() {
        "FL" | "L" | "FRONTLEFT" | "LEFTFRONT" => L,
        "FR" | "R" | "FRONTRIGHT" | "RIGHTFRONT" => R,
        "C" | "FC" | "CENTER" | "CENTRE" | "FRONTCENTER" => C,
        "LFE" | "LFE1" | "SUB" | "SUBWOOFER" | "SW" => LFE,
        "LFE2" => LFE2,
        "SL" | "LS" | "SIDELEFT" | "SURROUNDLEFT" | "LEFTSURROUND" => Ls,
        "SR" | "RS" | "SIDERIGHT" | "SURROUNDRIGHT" | "RIGHTSURROUND" => Rs,
        "BL" | "LB" | "LRS" | "BACKLEFT" | "LEFTBACK" | "REARLEFT" | "LEFTREAR" => Lb,
        "BR" | "RB" | "RRS" | "BACKRIGHT" | "RIGHTBACK" | "REARRIGHT" | "RIGHTREAR" => Rb,
        "RC" | "BC" | "CB" | "BACKCENTER" | "REARCENTER" | "CENTERBACK" => Cb,
        // Front-left/right of center (wide-front center pair).
        "LSC" | "FLC" | "FRONTLEFTCENTER" | "LEFTCENTER" => Lsc,
        "RSC" | "FRC" | "FRONTRIGHTCENTER" | "RIGHTCENTER" => Rsc,
        // Front wide.
        "FWL" | "LW" | "WL" | "WIDELEFT" | "FRONTWIDELEFT" => Lw,
        "FWR" | "RW" | "WR" | "WIDERIGHT" | "FRONTWIDERIGHT" => Rw,
        // Side direct (between side surround and back), where layouts use it.
        "LSD" => Lsd,
        "RSD" => Rsd,
        // Height / top tier.
        "TFL" | "TPFL" | "TOPFRONTLEFT" | "UFL" => Tfl,
        "TFR" | "TPFR" | "TOPFRONTRIGHT" | "UFR" => Tfr,
        "TSL" | "TPSL" | "TOPSIDELEFT" | "USL" => Tsl,
        "TSR" | "TPSR" | "TOPSIDERIGHT" | "USR" => Tsr,
        "TBL" | "TPBL" | "TOPBACKLEFT" | "UBL" | "TRL" => Tbl,
        "TBR" | "TPBR" | "TOPBACKRIGHT" | "UBR" | "TRR" => Tbr,
        "TC" | "TPC" | "TOPCENTER" | "TOPMIDDLECENTER" => Tc,
        "TFC" | "TOPFRONTCENTER" => Tfc,
        _ => Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RChannelLabel::*;

    #[test]
    fn maps_the_7_1_4_layout_names() {
        // Order matches Omniphony/layouts/7.1.4.yaml.
        let names = [
            "FL", "FR", "C", "LFE", "BL", "BR", "SL", "SR", "TFL", "TFR", "TBL", "TBR",
        ];
        let labels: Vec<_> = names.iter().map(|n| label_for_speaker_name(n)).collect();
        assert_eq!(
            labels,
            vec![L, R, C, LFE, Lb, Rb, Ls, Rs, Tfl, Tfr, Tbl, Tbr]
        );
    }

    #[test]
    fn maps_the_9_1_6_layout_names() {
        let names = [
            "FL", "FR", "C", "LFE", "FWL", "FWR", "SL", "SR", "BL", "BR", "TFL", "TFR", "TSL",
            "TSR", "TBL", "TBR",
        ];
        let labels: Vec<_> = names.iter().map(|n| label_for_speaker_name(n)).collect();
        assert_eq!(
            labels,
            vec![L, R, C, LFE, Lw, Rw, Ls, Rs, Lb, Rb, Tfl, Tfr, Tsl, Tsr, Tbl, Tbr]
        );
    }

    #[test]
    fn is_case_and_separator_insensitive() {
        assert_eq!(label_for_speaker_name("tfl"), Tfl);
        assert_eq!(label_for_speaker_name("Top Front Left"), Tfl);
        assert_eq!(label_for_speaker_name("top_back_right"), Tbr);
        assert_eq!(label_for_speaker_name("Front-Left"), L);
    }

    #[test]
    fn unknown_names_fall_back() {
        assert_eq!(label_for_speaker_name("WeirdSpeaker"), Unknown);
        assert_eq!(label_for_speaker_name(""), Unknown);
    }
}
