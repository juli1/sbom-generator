use std::num::NonZeroU32;

use bstr::BStr;
use bstr::ByteSlice;
use derive_builder::Builder;

#[derive(Builder, Clone, Copy, Debug)]
pub struct Position {
    #[allow(dead_code)]
    pub line: NonZeroU32,
    #[allow(dead_code)]
    pub col: NonZeroU32,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            line: NonZeroU32::new(1).unwrap(),
            col: NonZeroU32::new(1).unwrap(),
        }
    }
}

/// Get position of an offset in a code and return a [Position].
pub fn get_position_in_string(content: &str, offset: usize) -> anyhow::Result<Position> {
    if offset >= content.len() {
        anyhow::bail!("offset is larger than content length");
    }

    let bstr = BStr::new(&content);

    let mut line_number: u32 = 1;
    let lines = bstr.lines_with_terminator();
    for line in lines {
        let start_index = line.as_ptr() as usize - content.as_ptr() as usize;
        let end_index = start_index + line.len();

        if (start_index..end_index).contains(&offset) {
            let mut col_number: u32 = 1;
            for (grapheme_start, grapheme_end, _) in line.grapheme_indices() {
                let grapheme_absolute_start = start_index + grapheme_start;
                let grapheme_absolute_end = start_index + grapheme_end;

                // It's exactly the index we are looking for.
                if offset == grapheme_absolute_start {
                    return Ok(Position {
                        line: NonZeroU32::new(line_number).unwrap(),
                        col: NonZeroU32::new(col_number).unwrap(),
                    });
                }

                // The offset is within the grapheme we are looking for, it's the next col.
                if (grapheme_absolute_start..grapheme_absolute_end).contains(&offset) {
                    return Ok(Position {
                        line: NonZeroU32::new(line_number).unwrap(),
                        col: NonZeroU32::new(col_number + 1).unwrap(),
                    });
                }
                col_number += 1;
            }
        }
        line_number += 1;
    }

    Err(anyhow::anyhow!("cannot find position"))
}
