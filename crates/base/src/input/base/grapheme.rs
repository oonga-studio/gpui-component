//! Grapheme movement over rope chunks without flattening the input.
use ropey::Rope;
use sum_tree::Bias;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

pub(super) fn boundary(text: &Rope, offset: usize, direction: Bias) -> usize {
    let offset = text.floor_char_boundary(offset.min(text.len()));
    let mut cursor = GraphemeCursor::new(offset, text.len(), true);
    let (mut chunk, mut start) = text.chunk(offset);
    loop {
        let result = if direction == Bias::Left {
            cursor.prev_boundary(chunk, start)
        } else {
            cursor.next_boundary(chunk, start)
        };
        match result {
            Ok(Some(boundary)) => return boundary,
            Ok(None) => {
                return if direction == Bias::Left {
                    0
                } else {
                    text.len()
                };
            }
            Err(GraphemeIncomplete::PreContext(end)) => {
                let (context, context_start) = text.chunk(end - 1);
                cursor.provide_context(&context[..end - context_start], context_start);
            }
            Err(GraphemeIncomplete::PrevChunk) => {
                (chunk, start) = text.chunk(start - 1);
            }
            Err(GraphemeIncomplete::NextChunk) => {
                (chunk, start) = text.chunk(start + chunk.len());
            }
            Err(GraphemeIncomplete::InvalidOffset) => {
                unreachable!("the cursor is on a character boundary in the supplied rope chunk")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_segmentation::UnicodeSegmentation;

    fn assert_boundaries(text: &str) {
        let rope = Rope::from(text);
        let stops: Vec<_> = text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        for offset in text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
        {
            assert_eq!(
                boundary(&rope, offset, Bias::Left),
                stops
                    .iter()
                    .copied()
                    .rev()
                    .find(|i| *i < offset)
                    .unwrap_or(0),
                "left at {offset}"
            );
            assert_eq!(
                boundary(&rope, offset, Bias::Right),
                stops
                    .iter()
                    .copied()
                    .find(|i| *i > offset)
                    .unwrap_or(text.len()),
                "right at {offset}"
            );
        }
    }

    #[test]
    fn grapheme_boundaries_keep_accents_emoji_flags_and_crlf_whole() {
        for text in [
            "",
            "abc",
            "e\u{301}",
            "A👩‍💻B",
            "🇬🇧🇯🇵",
            "👍🏽",
            "a\r\nb",
            "\r\n",
            "क्\u{200d}ष",
        ] {
            assert_boundaries(text);
        }
    }

    #[test]
    fn grapheme_boundaries_span_rope_chunks() {
        // Long combining and regional-indicator runs exercise requested lookbehind
        // as well as both chunk-crossing directions. The oracle is contiguous text.
        for text in [
            format!("{}Ae{}B", "x".repeat(1000), "\u{301}".repeat(1400)),
            format!("{}!", "🇬🇧🇯🇵".repeat(150)),
            "a👩‍💻👍🏽e\u{301}\r\n".repeat(160),
        ] {
            assert!(Rope::from(text.as_str()).chunks().count() > 1);
            assert_boundaries(&text);
        }
    }
}
