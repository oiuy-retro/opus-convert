//! Title and file-name tools for one folder's table.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertAt {
    Start,
    End,
    Before,
    After,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextTools {
    pub remove: String,
    pub insert: String,
    pub at: InsertAt,
    pub find: String,
    pub strip: bool,
}

impl Default for TextTools {
    fn default() -> Self {
        Self {
            remove: String::new(),
            insert: String::new(),
            at: InsertAt::Start,
            find: String::new(),
            strip: false,
        }
    }
}

pub fn tools_ready(tools: &TextTools) -> bool {
    if tools.strip || !tools.remove.is_empty() {
        return true;
    }
    if tools.insert.is_empty() {
        return false;
    }
    match tools.at {
        InsertAt::Start | InsertAt::End => true,
        InsertAt::Before | InsertAt::After => !tools.find.is_empty(),
    }
}

pub fn apply_tools(title: &str, tools: &TextTools) -> String {
    let mut text = title.to_string();
    if tools.strip {
        text = strip_title(&text);
    }
    if !tools.remove.is_empty() {
        text = text.replace(&tools.remove, "");
    }
    if !tools.insert.is_empty()
        && tools_ready(&TextTools {
            remove: String::new(),
            strip: false,
            ..tools.clone()
        })
    {
        text = insert_text(&text, tools);
    }
    text
}

/// Drop the text before the first " - ", then drop the text from the first
/// opening bracket. The space before that bracket goes with it.
pub fn strip_title(input: &str) -> String {
    let mut text = input.to_string();
    if let Some(index) = text.find(" - ") {
        text = text[index + 3..].to_string();
    }
    if let Some(index) = text.find(is_opener) {
        let end = text[..index].trim_end().len();
        text = text[..end].to_string();
    }
    text.trim().to_string()
}

fn insert_text(text: &str, tools: &TextTools) -> String {
    match tools.at {
        InsertAt::Start => format!("{}{}", tools.insert, text),
        InsertAt::End => format!("{}{}", text, tools.insert),
        InsertAt::Before => match text.find(&tools.find) {
            Some(index) => format!("{}{}{}", &text[..index], tools.insert, &text[index..]),
            None => text.to_string(),
        },
        InsertAt::After => match text.find(&tools.find) {
            Some(index) => {
                let end = index + tools.find.len();
                format!("{}{}{}", &text[..end], tools.insert, &text[end..])
            }
            None => text.to_string(),
        },
    }
}

fn is_opener(ch: char) -> bool {
    matches!(
        ch,
        '(' | '['
            | '{'
            | '<'
            | '\u{FF08}'
            | '\u{FF3B}'
            | '\u{FF5B}'
            | '\u{3008}'
            | '\u{300A}'
            | '\u{300C}'
            | '\u{300E}'
            | '\u{3010}'
            | '\u{3014}'
            | '\u{3016}'
            | '\u{27E8}'
            | '\u{2329}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_matches_the_dash_and_any_opening_bracket() {
        assert_eq!(strip_title("Artist - Song (Official Video)"), "Song");
        assert_eq!(strip_title("Artist - Song [Live]"), "Song");
        assert_eq!(strip_title("Artist - Song {Demo}"), "Song");
        assert_eq!(strip_title("Artist - Song <HD>"), "Song");
        assert_eq!(strip_title("Artist - Song （Live）"), "Song");
        assert_eq!(strip_title("Song (Live)"), "Song");
        assert_eq!(strip_title("No Dash Here"), "No Dash Here");
        assert_eq!(strip_title("A - B - C (D)"), "B - C");
        assert_eq!(strip_title("Artist-Song (Live)"), "Artist-Song");
    }

    #[test]
    fn apply_runs_strip_then_remove_then_insert() {
        let tools = TextTools {
            remove: "Video".into(),
            insert: "Live ".into(),
            at: InsertAt::Start,
            find: String::new(),
            strip: true,
        };
        assert_eq!(apply_tools("Artist - Song (Official Video)", &tools), "Live Song");
    }

    #[test]
    fn insert_before_and_after_the_first_match() {
        let before = TextTools {
            insert: "My ".into(),
            at: InsertAt::Before,
            find: "Song".into(),
            ..TextTools::default()
        };
        assert_eq!(apply_tools("A Song Song", &before), "A My Song Song");
        let after = TextTools {
            insert: "!".into(),
            at: InsertAt::After,
            find: "Song".into(),
            ..TextTools::default()
        };
        assert_eq!(apply_tools("A Song B", &after), "A Song! B");
        assert!(!tools_ready(&TextTools {
            insert: "X".into(),
            at: InsertAt::Before,
            ..TextTools::default()
        }));
        assert!(tools_ready(&before));
        assert!(tools_ready(&TextTools {
            strip: true,
            ..TextTools::default()
        }));
    }

    #[test]
    fn remove_is_literal_and_repeats() {
        let tools = TextTools {
            remove: "Official ".into(),
            ..TextTools::default()
        };
        assert_eq!(apply_tools("Official Official Video", &tools), "Video");
    }
}
