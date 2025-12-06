use lsp_types::Position;

pub struct DocumentBody {
    pub(crate) content: String,
}

impl DocumentBody {
    pub fn get_word_at(&self, position: Position) -> Option<&str> {
        let line = self.content.lines().skip(position.line as usize).next()?;

        let word_start =
            line.char_indices()
                .take(position.character as usize)
                .fold(0, |start, (i, c)| {
                    c.is_ascii_alphanumeric()
                        .then_some(start)
                        .unwrap_or_else(|| i + 1)
                });

        let len = line
            .chars()
            .skip(word_start)
            .take_while(|c| c.is_ascii_alphanumeric())
            .count();

        Some(&line[word_start..word_start + len])
    }
}
