//! Structured placeholder commands from Raycast Lorem Ipsum (MIT):
//! raycast/extensions@5bcd41ff46cf8ad355e98f11bb7e716c6fdafc4b,
//! extensions/lorem-ipsum/src/{utils,html,markdown}.tsx.
//! Original author AntonNiklasson; contributors retained in source provenance.
use crate::lorem::{Generator, Random};

#[derive(Clone, Copy)]
enum Emphasis {
    Plain,
    Bold,
    Italic,
}
type Segments = Vec<(String, Emphasis)>;
enum Block {
    Heading(u32, String),
    Paragraph(Segments),
    List(bool, Vec<String>),
    Quote(String),
}

fn segments(generator: &mut Generator<impl Random>) -> Segments {
    let sentence = generator.sentences(1);
    let words: Vec<_> = sentence.split(' ').collect();
    if words.len() < 3 {
        return vec![(sentence, Emphasis::Plain)];
    }
    let emphasis = match generator.random_integer(0, 2) {
        0 => return vec![(sentence, Emphasis::Plain)],
        1 => Emphasis::Bold,
        _ => Emphasis::Italic,
    };
    let index = generator.random_integer(1, words.len() as u32 - 2) as usize;
    vec![
        (format!("{} ", words[..index].join(" ")), Emphasis::Plain),
        (words[index].to_string(), emphasis),
        (
            format!(" {}", words[index + 1..].join(" ")),
            Emphasis::Plain,
        ),
    ]
}

fn render_segments(segments: &Segments, html: bool) -> String {
    segments
        .iter()
        .map(|(text, emphasis)| match (html, emphasis) {
            (_, Emphasis::Plain) => text.clone(),
            (true, Emphasis::Bold) => format!("<strong>{text}</strong>"),
            (true, Emphasis::Italic) => format!("<em>{text}</em>"),
            (false, Emphasis::Bold) => format!("**{text}**"),
            (false, Emphasis::Italic) => format!("*{text}*"),
        })
        .collect()
}

fn render(blocks: &[Block], html: bool) -> String {
    blocks
        .iter()
        .map(|block| match block {
            Block::Heading(level, text) if html => format!("<h{level}>{text}</h{level}>"),
            Block::Heading(level, text) => format!("{} {text}", "#".repeat(*level as usize)),
            Block::Paragraph(segments) if html => {
                format!("<p>{}</p>", render_segments(segments, true))
            }
            Block::Paragraph(segments) => render_segments(segments, false),
            Block::Quote(text) if html => format!("<blockquote>\n  <p>{text}</p>\n</blockquote>"),
            Block::Quote(text) => format!("> {text}"),
            Block::List(ordered, items) if html => {
                let tag = if *ordered { "ol" } else { "ul" };
                format!(
                    "<{tag}>\n{}\n</{tag}>",
                    items
                        .iter()
                        .map(|item| format!("  <li>{item}</li>"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            Block::List(ordered, items) => items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    if *ordered {
                        format!("{}. {item}", index + 1)
                    } else {
                        format!("- {item}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn generate(generator: &mut Generator<impl Random>, html: bool) -> String {
    let count = generator.random_integer(3, 8);
    let level = generator.random_integer(2, 3);
    let words = generator.random_integer(2, 5);
    let mut blocks = vec![Block::Heading(level, generator.words(words))];
    for _ in 1..count {
        let block = match generator.random_integer(0, 4) {
            3 => {
                let count = generator.random_integer(3, 5);
                let ordered = generator.random_integer(0, 1) == 0;
                let items = (0..count)
                    .map(|_| {
                        let words = generator.random_integer(3, 6);
                        generator.words(words)
                    })
                    .collect();
                Block::List(ordered, items)
            }
            4 => Block::Quote(generator.sentences(1)),
            _ => Block::Paragraph(segments(generator)),
        };
        blocks.push(block);
    }
    render(&blocks, html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_blocks_match_source_format() {
        let blocks = vec![
            Block::Heading(2, "Lorem ipsum".into()),
            Block::Paragraph(vec![
                ("Plain ".into(), Emphasis::Plain),
                ("bold".into(), Emphasis::Bold),
                (" italic".into(), Emphasis::Italic),
            ]),
            Block::List(true, vec!["first".into(), "second".into()]),
            Block::List(false, vec!["third".into()]),
            Block::Quote("Quoted.".into()),
        ];
        assert_eq!(render(&blocks, false), "## Lorem ipsum\n\nPlain **bold*** italic*\n\n1. first\n2. second\n\n- third\n\n> Quoted.");
        assert_eq!(render(&blocks, true), "<h2>Lorem ipsum</h2>\n\n<p>Plain <strong>bold</strong><em> italic</em></p>\n\n<ol>\n  <li>first</li>\n  <li>second</li>\n</ol>\n\n<ul>\n  <li>third</li>\n</ul>\n\n<blockquote>\n  <p>Quoted.</p>\n</blockquote>");
    }

    #[test]
    fn random_documents_obey_source_block_bounds_and_begin_with_heading() {
        let mut seen_min = false;
        let mut seen_max = false;
        for seed in 0..512 {
            for html in [false, true] {
                let document = generate(&mut Generator::new(fastrand::Rng::with_seed(seed)), html);
                let blocks = document.split("\n\n").count();
                assert!((3..=8).contains(&blocks));
                seen_min |= blocks == 3;
                seen_max |= blocks == 8;
                assert!(if html {
                    document.starts_with("<h2>") || document.starts_with("<h3>")
                } else {
                    document.starts_with("## ") || document.starts_with("### ")
                });
            }
        }
        assert!(seen_min && seen_max);
    }
}
