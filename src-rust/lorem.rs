//! Placeholder text engine.
//!
//! Faithful Rust port of the generator used by the original extension:
//! `src/lib/lorem.ts` of this repository on top of the ISC-licensed npm package
//! `lorem-ipsum` 2.0.10 (`src/constants/words.ts`, `src/lib/generator.ts`,
//! `src/lib/LoremIpsum.ts`, `src/util/capitalize.ts`). The word list, the
//! inclusive random-integer formula, sentence capitalisation, the `0 means a
//! random amount` behaviour and the paragraph `\r\n\r\n` separator are kept.

/// `WORDS` from lorem-ipsum 2.0.10, unchanged and in the same order.
pub const WORDS: [&str; 62] = [
    "ad",
    "adipisicing",
    "aliqua",
    "aliquip",
    "amet",
    "anim",
    "aute",
    "cillum",
    "commodo",
    "consectetur",
    "consequat",
    "culpa",
    "cupidatat",
    "deserunt",
    "do",
    "dolor",
    "dolore",
    "duis",
    "ea",
    "eiusmod",
    "elit",
    "enim",
    "esse",
    "est",
    "et",
    "eu",
    "ex",
    "excepteur",
    "exercitation",
    "fugiat",
    "id",
    "in",
    "incididunt",
    "ipsum",
    "irure",
    "labore",
    "laboris",
    "laborum",
    "Lorem",
    "magna",
    "minim",
    "mollit",
    "nisi",
    "non",
    "nostrud",
    "nulla",
    "occaecat",
    "officia",
    "pariatur",
    "proident",
    "qui",
    "quis",
    "reprehenderit",
    "sint",
    "sit",
    "sunt",
    "tempor",
    "ullamco",
    "ut",
    "velit",
    "veniam",
    "voluptate",
];

/// `LOREM_IPSUM_MAX_NUMBER` from `src/lib/lorem.ts`.
pub const MAX_NUMBER: i64 = 1000;
/// `loremIpsumOptions` from `src/lib/lorem.ts`.
pub const SENTENCES_PER_PARAGRAPH: (u32, u32) = (4, 8);
pub const WORDS_PER_SENTENCE: (u32, u32) = (4, 16);
/// Paragraph separator used by `generateParagraphs` in `src/lib/lorem.ts`.
pub const PARAGRAPH_SEPARATOR: &str = "\r\n\r\n";

pub fn invalid_number_message() -> String {
    format!("Please enter a valid integer number, no more than {MAX_NUMBER}")
}

/// ECMAScript `StrWhiteSpaceChar` (WhiteSpace + LineTerminator), as skipped by
/// `parseInt`. Unlike `char::is_whitespace`, U+0085 is not included and U+FEFF is.
fn js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// `parseInt(text, 10)`: leading whitespace, optional sign, the longest run of
/// ASCII digits; trailing text is ignored. `None` is `NaN`. Magnitudes beyond
/// `i64` saturate, which still fails the `> 1000` check exactly like JS.
pub fn js_parse_int(text: &str) -> Option<i64> {
    let rest = text.trim_start_matches(js_whitespace);
    let (negative, rest) = match rest.as_bytes().first() {
        Some(b'-') => (true, &rest[1..]),
        Some(b'+') => (false, &rest[1..]),
        _ => (false, rest),
    };
    let digits = rest.bytes().take_while(u8::is_ascii_digit);
    let mut value: i64 = 0;
    let mut any = false;
    for d in digits {
        any = true;
        value = value.saturating_mul(10).saturating_add(i64::from(d - b'0'));
    }
    any.then_some(if negative { -value } else { value })
}

/// `safeLoremIpsumNumberArg`: a missing or empty argument means 1; `NaN` and
/// values above 1000 are rejected with the original message. Negative values
/// crash the original with a JavaScript `RangeError` (`Array(-n)`); the port
/// rejects them with the same validation message instead. `-0` is `0`.
pub fn safe_number(arg: Option<&str>) -> Result<u32, String> {
    let text = match arg {
        None | Some("") => "1",
        Some(text) => text,
    };
    match js_parse_int(text) {
        Some(n) if (0..=MAX_NUMBER).contains(&n) => Ok(n as u32),
        _ => Err(invalid_number_message()),
    }
}

/// Random source; `f64` in `[0, 1)` exactly like `Math.random`.
pub trait Random {
    fn next_f64(&mut self) -> f64;
}

impl Random for fastrand::Rng {
    fn next_f64(&mut self) -> f64 {
        self.f64()
    }
}

pub struct Generator<R: Random> {
    pub random: R,
}

impl<R: Random> Generator<R> {
    pub fn new(random: R) -> Self {
        Self { random }
    }

    /// `Math.floor(random() * (max - min + 1) + min)`.
    pub fn random_integer(&mut self, min: u32, max: u32) -> u32 {
        let value = (self.random.next_f64() * f64::from(max - min + 1) + f64::from(min)).floor();
        // `random() < 1` keeps this inside the bounds; clamp guards float edge cases.
        (value as u32).clamp(min, max)
    }

    fn pluck_word(&mut self) -> &'static str {
        WORDS[self.random_integer(0, WORDS.len() as u32 - 1) as usize]
    }

    /// `generateRandomWords(num)`: `0` (falsy) picks 4..=16 words.
    pub fn words(&mut self, num: u32) -> String {
        let length = if num == 0 {
            self.random_integer(WORDS_PER_SENTENCE.0, WORDS_PER_SENTENCE.1)
        } else {
            num
        };
        // The original reduce prepends each plucked word; keep the same order.
        let mut words: Vec<&str> = (0..length).map(|_| self.pluck_word()).collect();
        words.reverse();
        words.join(" ")
    }

    /// `generateRandomSentence()`: capitalised random-length word run plus a period.
    pub fn sentence(&mut self) -> String {
        format!("{}.", capitalize(&self.words(0)))
    }

    /// `generateRandomParagraph(num)`: `0` (falsy) picks 4..=8 sentences.
    pub fn sentences(&mut self, num: u32) -> String {
        let length = if num == 0 {
            self.random_integer(SENTENCES_PER_PARAGRAPH.0, SENTENCES_PER_PARAGRAPH.1)
        } else {
            num
        };
        let mut sentences: Vec<String> = (0..length).map(|_| self.sentence()).collect();
        sentences.reverse();
        sentences.join(" ")
    }

    /// `generateParagraphs(count)` of `src/lib/lorem.ts`: each paragraph has
    /// 4..=8 sentences; `0` paragraphs produce an empty string, as in the original.
    pub fn paragraphs(&mut self, count: u32) -> String {
        (0..count)
            .map(|_| {
                let n = self.random_integer(SENTENCES_PER_PARAGRAPH.0, SENTENCES_PER_PARAGRAPH.1);
                self.sentences(n)
            })
            .collect::<Vec<_>>()
            .join(PARAGRAPH_SEPARATOR)
    }
}

/// `capitalize` of lorem-ipsum: trim, then upper-case the first character.
pub fn capitalize(text: &str) -> String {
    let trimmed = text.trim();
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Prompt built by `constructPrompt` in `src/ai-generate.tsx`, byte-for-byte
/// including the template literal's indentation and surrounding newlines.
pub fn ai_prompt(topic: Option<&str>) -> String {
    let topic_instructions = match topic {
        Some(topic) => format!("The topic you should write about is \"{topic}\"."),
        None => "Pick a completely random topic to write about. Just make sure all paragraphs are about that topic.".to_owned(),
    };
    format!(
        "\n    You are only capable of outputting a series of paragraphs. What follows is a set of rules I'd like you to adhere to:\n    - separate paragraphs with a blank line, nothing else\n    - do not write in first person\n    - do not explain what topic you are writing about\n    - use brief and concise language, with a casual tone\n\n    Generate between two and five short paragraphs. {topic_instructions}\n  "
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Replays a fixed `Math.random` sequence.
    struct Script(Vec<f64>, usize);
    impl Random for Script {
        fn next_f64(&mut self) -> f64 {
            let value = self.0[self.1 % self.0.len()];
            self.1 += 1;
            value
        }
    }

    fn seeded(seed: u64) -> Generator<fastrand::Rng> {
        Generator::new(fastrand::Rng::with_seed(seed))
    }

    #[test]
    fn word_list_matches_npm_lorem_ipsum_2_0_10() {
        assert_eq!(WORDS.len(), 62);
        assert_eq!(WORDS[0], "ad");
        assert_eq!(WORDS[38], "Lorem");
        assert_eq!(WORDS[61], "voluptate");
        let mut sorted = WORDS.map(str::to_lowercase).to_vec();
        sorted.sort();
        assert_eq!(
            sorted,
            WORDS.map(str::to_lowercase).to_vec(),
            "upstream order is alphabetical"
        );
    }

    #[test]
    fn random_integer_is_inclusive_js_formula() {
        let mut g = Generator::new(Script(vec![0.0, 0.999_999_999, 0.5], 0));
        assert_eq!(g.random_integer(4, 16), 4);
        assert_eq!(g.random_integer(4, 16), 16);
        assert_eq!(g.random_integer(4, 16), 10); // floor(0.5 * 13 + 4)
    }

    #[test]
    fn deterministic_words_and_sentences_follow_upstream_shape() {
        // random()==0 always plucks WORDS[0]; length bounds pick the minimum.
        let mut g = Generator::new(Script(vec![0.0], 0));
        assert_eq!(g.words(3), "ad ad ad");
        assert_eq!(g.words(0), "ad ad ad ad");
        assert_eq!(g.sentence(), "Ad ad ad ad.");
        assert_eq!(g.sentences(2), "Ad ad ad ad. Ad ad ad ad.");
        assert_eq!(
            g.paragraphs(2),
            "Ad ad ad ad. Ad ad ad ad. Ad ad ad ad. Ad ad ad ad.\r\n\r\nAd ad ad ad. Ad ad ad ad. Ad ad ad ad. Ad ad ad ad."
        );
        assert_eq!(g.paragraphs(0), "");
        // Prepending reduce keeps the reverse pluck order.
        let mut g = Generator::new(Script(vec![0.0, 0.99], 0));
        assert_eq!(g.words(2), "voluptate ad");
    }

    #[test]
    fn generated_counts_and_bounds() {
        let mut g = seeded(7);
        for n in [1, 5, 1000] {
            let words = g.words(n);
            assert_eq!(words.split(' ').count(), n as usize);
            assert!(words.split(' ').all(|w| WORDS.contains(&w)));
        }
        for _ in 0..200 {
            let random = g.words(0).split(' ').count();
            assert!((4..=16).contains(&random), "{random}");
            let sentence = g.sentence();
            assert!(sentence.ends_with('.'));
            assert!(sentence.chars().next().unwrap().is_uppercase());
            assert!((4..=16).contains(&sentence.split(' ').count()));
            let paragraph_sentences = g.sentences(0).matches(". ").count() + 1;
            assert!((4..=8).contains(&paragraph_sentences));
        }
        let three = g.sentences(3);
        assert_eq!(three.matches('.').count(), 3);
        assert!(!three.contains("\r\n"));
        let paragraphs = g.paragraphs(25);
        let parts: Vec<_> = paragraphs.split(PARAGRAPH_SEPARATOR).collect();
        assert_eq!(parts.len(), 25);
        for part in parts {
            let count = part.matches('.').count();
            assert!((4..=8).contains(&count), "{count}");
            assert!(!part.contains('\n'));
        }
    }

    #[test]
    fn upper_bound_output_fits_the_host_text_limit_in_practice() {
        let mut g = seeded(11);
        let text = g.paragraphs(1000);
        assert_eq!(text.split(PARAGRAPH_SEPARATOR).count(), 1000);
        assert!(text.len() < 1024 * 1024, "{}", text.len());
    }

    #[test]
    fn js_parse_int_semantics() {
        for (text, expected) in [
            ("7", Some(7)),
            ("  12", Some(12)),
            ("\u{00A0}\t\n3", Some(3)),
            ("\u{FEFF}4", Some(4)),
            ("5abc", Some(5)),
            ("3.9", Some(3)),
            ("1e3", Some(1)),
            ("+8", Some(8)),
            ("-3", Some(-3)),
            ("-0", Some(0)),
            ("007", Some(7)),
            ("99999999999999999999999", Some(i64::MAX)),
            ("abc", None),
            ("", None),
            (" ", None),
            ("-", None),
            ("+-1", None),
            ("\u{0085}1", None),
            ("٣", None),
        ] {
            assert_eq!(js_parse_int(text), expected, "{text:?}");
        }
    }

    #[test]
    fn safe_number_matches_upstream_tests() {
        assert_eq!(safe_number(None), Ok(1));
        assert_eq!(safe_number(Some("")), Ok(1));
        assert_eq!(safe_number(Some("7")), Ok(7));
        assert_eq!(safe_number(Some("1000")), Ok(1000));
        assert_eq!(safe_number(Some("0")), Ok(0));
        assert_eq!(safe_number(Some("12 paragraphs")), Ok(12));
        for bad in ["1001", "not-a-number", " ", "-1", "99999999999999999999"] {
            let error = safe_number(Some(bad)).unwrap_err();
            assert_eq!(
                error,
                "Please enter a valid integer number, no more than 1000"
            );
        }
    }

    #[test]
    fn capitalize_trims_and_uppercases_first_char() {
        assert_eq!(capitalize("  lorem ipsum "), "Lorem ipsum");
        assert_eq!(capitalize("Lorem"), "Lorem");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn ai_prompt_is_byte_identical_to_upstream_template() {
        let random = ai_prompt(None);
        assert!(
            random.starts_with("\n    You are only capable of outputting a series of paragraphs.")
        );
        assert!(random.ends_with("Just make sure all paragraphs are about that topic.\n  "));
        assert!(random.contains("\n    - separate paragraphs with a blank line, nothing else\n"));
        assert!(random.contains("\n\n    Generate between two and five short paragraphs. Pick a completely random topic"));
        let topic = ai_prompt(Some("space cats"));
        assert!(topic.ends_with(
            "Generate between two and five short paragraphs. The topic you should write about is \"space cats\".\n  "
        ));
    }
}
