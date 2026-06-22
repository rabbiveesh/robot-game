//! Spoken-number parsing. Pure text processing — no audio, no network.
//!
//! Turns a speech-recognition transcript into the integer the kid said, for the
//! "Say it" answer mode (voice-input-impl-spec). The platform layer captures
//! audio + timing and confidence; this module owns only the
//! transcript → number contract, which is fully unit-testable.
//!
//! Handles digit words (zero..nineteen), tens (twenty..ninety), "hundred",
//! combinations ("twenty three" / "twenty-three" → 23, "one hundred and 44" →
//! 144), bare digits, stripped filler words, and self-correction where the LAST
//! stated number wins ("thirteen no twelve" → 12). Range is 0..=144 (covers
//! division answers at the top band); anything outside, or unrecognizable,
//! returns `None` so the UI asks again rather than marking a wrong answer.

/// Words that carry no numeric meaning and are dropped before parsing.
/// Includes articles/connectors ("a", "and") so "a hundred" and
/// "one hundred and 44" parse cleanly.
const FILLERS: &[&str] = &[
    "um", "umm", "uh", "uhh", "erm", "like", "maybe", "i", "think", "its",
    "it", "is", "a", "an", "and", "the", "answer",
];

/// Words that mark a self-correction boundary — the kid abandoned the previous
/// number and is about to say the real one.
const BOUNDARIES: &[&str] = &["no", "not", "nope", "wait", "scratch", "actually"];

const MAX_ANSWER: u32 = 144;

pub fn parse_spoken_number(transcript: &str) -> Option<u32> {
    // Normalize: lowercase, hyphens→spaces, drop other punctuation (including
    // apostrophes so "it's" → "its").
    let normalized: String = transcript
        .to_lowercase()
        .chars()
        .map(|c| if c == '-' { ' ' } else { c })
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();

    // Accumulate numbers, splitting on self-correction boundaries; the last
    // completed number wins.
    let mut completed: Vec<u32> = Vec::new();
    let mut current: Option<u32> = None;

    for token in normalized.split_whitespace() {
        if BOUNDARIES.contains(&token) {
            if let Some(v) = current.take() {
                completed.push(v);
            }
            continue;
        }
        if FILLERS.contains(&token) {
            continue;
        }
        match word_value(token) {
            Some(Word::Add(v)) => {
                current = Some(current.unwrap_or(0) + v);
            }
            Some(Word::Hundred) => {
                current = Some(current.map(|c| c.max(1)).unwrap_or(1) * 100);
            }
            Some(Word::Digits(v)) => {
                // A bare numeral: small ones add (so "twenty 3" works),
                // 100+ replace the running value.
                current = Some(if v < 100 { current.unwrap_or(0) + v } else { v });
            }
            None => {
                // An unrecognized non-filler token (e.g. "firteen") makes the
                // whole transcript unparseable — better to ask again than guess.
                return None;
            }
        }
    }
    if let Some(v) = current.take() {
        completed.push(v);
    }

    match completed.last().copied() {
        Some(v) if v <= MAX_ANSWER => Some(v),
        _ => None,
    }
}

enum Word {
    Add(u32),
    Hundred,
    Digits(u32),
}

fn word_value(token: &str) -> Option<Word> {
    if let Ok(n) = token.parse::<u32>() {
        return Some(Word::Digits(n));
    }
    let v = match token {
        "zero" => 0,
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "thirteen" => 13,
        "fourteen" => 14,
        "fifteen" => 15,
        "sixteen" => 16,
        "seventeen" => 17,
        "eighteen" => 18,
        "nineteen" => 19,
        "twenty" => 20,
        "thirty" => 30,
        "forty" => 40,
        "fifty" => 50,
        "sixty" => 60,
        "seventy" => 70,
        "eighty" => 80,
        "ninety" => 90,
        "hundred" => return Some(Word::Hundred),
        _ => return None,
    };
    Some(Word::Add(v))
}

/// Whether the transcript contained any stripped filler words — a small
/// engagement signal the platform layer can fold into the voice event.
pub fn had_filler_words(transcript: &str) -> bool {
    transcript
        .to_lowercase()
        .split_whitespace()
        .any(|t| FILLERS.contains(&t.trim_matches(|c: char| !c.is_alphanumeric())))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── The spec's contract table ──────────────────────
    #[test]
    fn spec_contract_table() {
        assert_eq!(parse_spoken_number("thirteen"), Some(13));
        assert_eq!(parse_spoken_number("13"), Some(13));
        assert_eq!(parse_spoken_number("twenty three"), Some(23));
        assert_eq!(parse_spoken_number("twenty-three"), Some(23));
        assert_eq!(parse_spoken_number("one hundred"), Some(100));
        assert_eq!(parse_spoken_number("a hundred"), Some(100));
        assert_eq!(parse_spoken_number("one hundred and 44"), Some(144));
        assert_eq!(parse_spoken_number("umm thirteen"), Some(13));
        assert_eq!(parse_spoken_number("I think it's thirteen"), Some(13));
        assert_eq!(parse_spoken_number("thirteen no twelve"), Some(12));
        assert_eq!(parse_spoken_number("firteen"), None);
        assert_eq!(parse_spoken_number(""), None);
    }

    #[test]
    fn zero_is_a_valid_answer() {
        assert_eq!(parse_spoken_number("zero"), Some(0));
        assert_eq!(parse_spoken_number("0"), Some(0));
    }

    #[test]
    fn teens_and_tens_combine() {
        assert_eq!(parse_spoken_number("seventeen"), Some(17));
        assert_eq!(parse_spoken_number("forty two"), Some(42));
        assert_eq!(parse_spoken_number("ninety nine"), Some(99));
    }

    #[test]
    fn multiple_self_corrections_take_the_last() {
        assert_eq!(parse_spoken_number("ten no eleven no twelve"), Some(12));
        assert_eq!(parse_spoken_number("twenty wait thirty"), Some(30));
    }

    #[test]
    fn mixed_words_and_digits() {
        assert_eq!(parse_spoken_number("twenty 3"), Some(23));
        assert_eq!(parse_spoken_number("144"), Some(144));
        assert_eq!(parse_spoken_number("one hundred and five"), Some(105));
    }

    #[test]
    fn out_of_range_is_none() {
        // Beyond the 0..=144 answer space → ask again rather than submit junk.
        assert_eq!(parse_spoken_number("two hundred"), None);
        assert_eq!(parse_spoken_number("999"), None);
    }

    #[test]
    fn pure_gibberish_is_none() {
        assert_eq!(parse_spoken_number("banana wobble"), None);
        assert_eq!(parse_spoken_number("um uh like"), None);
    }

    #[test]
    fn leading_and_trailing_fillers_stripped() {
        assert_eq!(parse_spoken_number("um maybe it's like fifteen"), Some(15));
        assert_eq!(parse_spoken_number("fifteen i think"), Some(15));
    }

    #[test]
    fn filler_detection_signal() {
        assert!(had_filler_words("um thirteen"));
        assert!(!had_filler_words("thirteen"));
    }
}
