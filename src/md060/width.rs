use icu_properties::{
    CodePointMapData, CodePointSetData, EmojiSetData,
    props::{
        BasicEmoji, DefaultIgnorableCodePoint, EastAsianWidth, EmojiModifier, EmojiModifierBase,
        ExtendedPictographic, GeneralCategory,
    },
};
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;

static ANSI: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
    r"(?:\x1b\]|\x{009d})[^\x07\x1b\x{009c}\x{009d}]*(?:\x07|\x1b\\|\x{009c})|[\x1b\x{009b}][\[\]()#;?]*(?:[0-9]{1,4}(?:[;:][0-9]{0,4})*)?[0-9A-PR-TZcf-nq-uy=><~]"
).unwrap()
});

// Unicode 17 emoji-sequences.txt, RGI_Emoji_Flag_Sequence. Arbitrary regional
// indicator pairs are not RGI emoji and use their first scalar's narrow width.
const FLAG_CODES: &str = "AC AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CP CQ CR CU CV CW CX CY CZ DE DG DJ DK DM DO DZ EA EC EE EG EH ER ES ET EU FI FJ FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU IC ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TA TC TD TF TG TH TJ TK TL TM TN TO TR TT TV TW TZ UA UG UM UN US UY UZ VA VC VE VG VI VN VU WF WS XK YE YT ZA ZM ZW";

fn visible(ch: char) -> bool {
    !CodePointSetData::new::<DefaultIgnorableCodePoint>().contains(ch)
        && !matches!(
            CodePointMapData::<GeneralCategory>::new().get(ch),
            GeneralCategory::Control
                | GeneralCategory::Format
                | GeneralCategory::NonspacingMark
                | GeneralCategory::EnclosingMark
                | GeneralCategory::Surrogate
        )
}

fn east_asian(ch: char) -> usize {
    if matches!(
        CodePointMapData::<EastAsianWidth>::new().get(ch),
        EastAsianWidth::Wide | EastAsianWidth::Fullwidth
    ) {
        2
    } else {
        1
    }
}

fn emoji(segment: &str) -> bool {
    if EmojiSetData::new::<BasicEmoji>().contains_str(segment) {
        return true;
    }
    let chars: Vec<_> = segment.chars().collect();
    if chars.len() == 2 {
        if (chars[0].is_ascii_digit() || matches!(chars[0], '#' | '*')) && chars[1] == '\u{20e3}' {
            return true;
        }
        if CodePointSetData::new::<EmojiModifierBase>().contains(chars[0])
            && CodePointSetData::new::<EmojiModifier>().contains(chars[1])
        {
            return true;
        }
        if chars
            .iter()
            .all(|ch| ('\u{1f1e6}'..='\u{1f1ff}').contains(ch))
        {
            let code = [
                (chars[0] as u32 - 0x1f1e6) as u8 + b'A',
                (chars[1] as u32 - 0x1f1e6) as u8 + b'A',
            ];
            return FLAG_CODES
                .as_bytes()
                .chunks(3)
                .any(|item| item.get(..2) == Some(&code));
        }
    }
    if chars.len() == 3
        && (chars[0].is_ascii_digit() || matches!(chars[0], '#' | '*'))
        && chars[1..] == ['\u{fe0f}', '\u{20e3}']
    {
        return true;
    }
    crate::utf16_len(segment) <= 50
        && segment.contains('\u{200d}')
        && chars
            .iter()
            .filter(|ch| CodePointSetData::new::<ExtendedPictographic>().contains(**ch))
            .take(2)
            .count()
            == 2
}

fn leading(cp: u32) -> bool {
    matches!(cp,0x1100..=0x115f | 0xa960..=0xa97c)
}
fn vowel(cp: u32) -> bool {
    matches!(cp,0x1160..=0x11a7 | 0xd7b0..=0xd7c6)
}
fn trailing(cp: u32) -> bool {
    matches!(cp,0x11a8..=0x11ff | 0xd7cb..=0xd7fb)
}

fn cluster(segment: &str) -> usize {
    let Some(start) = segment
        .char_indices()
        .find(|(_, ch)| visible(*ch))
        .map(|(i, _)| i)
    else {
        return 0;
    };
    if emoji(segment) {
        return 2;
    }
    let visible_segment = &segment[start..];
    let chars: Vec<_> = visible_segment.chars().filter(|ch| visible(*ch)).collect();
    if leading(chars[0] as u32) || vowel(chars[0] as u32) || trailing(chars[0] as u32) {
        let mut width = 0;
        let mut cursor = 0;
        while cursor < chars.len() {
            if leading(chars[cursor] as u32)
                && chars.get(cursor + 1).is_some_and(|ch| vowel(*ch as u32))
            {
                width += 2;
                cursor += if chars.get(cursor + 2).is_some_and(|ch| trailing(*ch as u32)) {
                    3
                } else {
                    2
                };
            } else {
                width += east_asian(chars[cursor]);
                cursor += 1;
            }
        }
        return width;
    }
    east_asian(chars[0])
        + visible_segment
            .chars()
            .skip(1)
            .filter(|ch| {
                CodePointMapData::<GeneralCategory>::new().get(*ch) == GeneralCategory::SpacingMark
                    || ('\u{ff00}'..='\u{ffef}').contains(ch)
            })
            .map(east_asian)
            .sum::<usize>()
}

pub(super) fn width(input: &str) -> usize {
    let stripped = if input.contains(['\x1b', '\u{009b}']) {
        ANSI.replace_all(input, "")
    } else {
        std::borrow::Cow::Borrowed(input)
    };
    if stripped.bytes().all(|ch| (0x20..=0x7e).contains(&ch)) {
        return stripped.len();
    }
    stripped.graphemes(true).map(cluster).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grapheme_width_matches_the_audited_node_unicode_runtime() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../tools/parser-oracle/md060-width-observations.json"
        ))
        .unwrap();
        for row in cases.as_array().unwrap() {
            let input = row["input"].as_str().unwrap();
            assert_eq!(
                width(input),
                row["width"].as_u64().unwrap() as usize,
                "{input:?}"
            );
        }
    }
}
