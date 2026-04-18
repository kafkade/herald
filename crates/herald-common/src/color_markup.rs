use crate::types::Color;

/// A character with an optional color from markup parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColoredChar {
    pub ch: char,
    pub color: Option<Color>,
}

/// Resolve a tag name (case-insensitive) to a `Color`.
fn parse_color_tag(tag: &str) -> Option<Color> {
    match tag.to_lowercase().as_str() {
        "red" => Some(Color::Red),
        "orange" => Some(Color::Orange),
        "yellow" => Some(Color::Yellow),
        "green" => Some(Color::Green),
        "blue" => Some(Color::Blue),
        "violet" => Some(Color::Violet),
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        _ => None,
    }
}

/// Parse text with color markup tags.
///
/// Supported syntax:
/// - `{red}text{/red}` — applies red color to "text"
/// - `{red}text{}` — shorthand: `{}` closes the current color
/// - Nested tags: innermost color wins (stack-based)
/// - Invalid/unrecognized tags are rendered literally including braces
///
/// Returns a `Vec<ColoredChar>` where each char may have an associated color.
pub fn parse_color_markup(input: &str) -> Vec<ColoredChar> {
    let mut result = Vec::new();
    let mut color_stack: Vec<Color> = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '{' {
            // Try to find the closing '}'
            if let Some(close) = chars[i + 1..].iter().position(|&c| c == '}') {
                let close = i + 1 + close;
                let tag_content: String = chars[i + 1..close].iter().collect();

                if tag_content.is_empty() {
                    // `{}` — shorthand close: pop top of stack
                    color_stack.pop();
                    i = close + 1;
                } else if let Some(stripped) = tag_content.strip_prefix('/') {
                    // Closing tag like `{/red}`
                    if let Some(color) = parse_color_tag(stripped) {
                        // Pop the matching color from the stack (search from top)
                        if let Some(pos) = color_stack.iter().rposition(|c| *c == color) {
                            color_stack.remove(pos);
                        } else {
                            // No matching open tag — render literally
                            emit_literal(&chars[i..=close], &color_stack, &mut result);
                        }
                    } else {
                        // Invalid closing tag — render literally
                        emit_literal(&chars[i..=close], &color_stack, &mut result);
                    }
                    i = close + 1;
                } else if let Some(color) = parse_color_tag(&tag_content) {
                    // Valid opening tag
                    color_stack.push(color);
                    i = close + 1;
                } else {
                    // Invalid tag name — render literally
                    emit_literal(&chars[i..=close], &color_stack, &mut result);
                    i = close + 1;
                }
            } else {
                // No closing '}' found — emit '{' literally, rest will follow
                result.push(ColoredChar {
                    ch: '{',
                    color: color_stack.last().copied(),
                });
                i += 1;
            }
        } else {
            result.push(ColoredChar {
                ch: chars[i],
                color: color_stack.last().copied(),
            });
            i += 1;
        }
    }

    result
}

/// Emit a slice of characters literally with the current color.
fn emit_literal(chars: &[char], color_stack: &[Color], result: &mut Vec<ColoredChar>) {
    let color = color_stack.last().copied();
    for &ch in chars {
        result.push(ColoredChar { ch, color });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars_text(chars: &[ColoredChar]) -> String {
        chars.iter().map(|c| c.ch).collect()
    }

    #[test]
    fn plain_text_no_markup() {
        let result = parse_color_markup("HELLO");
        assert_eq!(result.len(), 5);
        assert_eq!(chars_text(&result), "HELLO");
        assert!(result.iter().all(|c| c.color.is_none()));
    }

    #[test]
    fn single_color_tag() {
        let result = parse_color_markup("{red}HI{/red}");
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            ColoredChar {
                ch: 'H',
                color: Some(Color::Red)
            }
        );
        assert_eq!(
            result[1],
            ColoredChar {
                ch: 'I',
                color: Some(Color::Red)
            }
        );
    }

    #[test]
    fn shorthand_close() {
        let result = parse_color_markup("{red}HI{}");
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            ColoredChar {
                ch: 'H',
                color: Some(Color::Red)
            }
        );
        assert_eq!(
            result[1],
            ColoredChar {
                ch: 'I',
                color: Some(Color::Red)
            }
        );
    }

    #[test]
    fn mixed_colored_and_plain() {
        let result = parse_color_markup("A{blue}BC{}D");
        assert_eq!(result.len(), 4);
        assert_eq!(
            result[0],
            ColoredChar {
                ch: 'A',
                color: None
            }
        );
        assert_eq!(
            result[1],
            ColoredChar {
                ch: 'B',
                color: Some(Color::Blue)
            }
        );
        assert_eq!(
            result[2],
            ColoredChar {
                ch: 'C',
                color: Some(Color::Blue)
            }
        );
        assert_eq!(
            result[3],
            ColoredChar {
                ch: 'D',
                color: None
            }
        );
    }

    #[test]
    fn nested_colors() {
        let result = parse_color_markup("{red}A{blue}B{/blue}C{/red}");
        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0],
            ColoredChar {
                ch: 'A',
                color: Some(Color::Red)
            }
        );
        assert_eq!(
            result[1],
            ColoredChar {
                ch: 'B',
                color: Some(Color::Blue)
            }
        );
        assert_eq!(
            result[2],
            ColoredChar {
                ch: 'C',
                color: Some(Color::Red)
            }
        );
    }

    #[test]
    fn invalid_tag_rendered_literally() {
        let result = parse_color_markup("{invalid}HI");
        assert_eq!(chars_text(&result), "{invalid}HI");
        assert!(result.iter().all(|c| c.color.is_none()));
    }

    #[test]
    fn empty_input() {
        let result = parse_color_markup("");
        assert!(result.is_empty());
    }

    #[test]
    fn unclosed_brace() {
        let result = parse_color_markup("{red");
        assert_eq!(chars_text(&result), "{red");
        assert!(result.iter().all(|c| c.color.is_none()));
    }

    #[test]
    fn close_without_open() {
        let result = parse_color_markup("{/red}HI");
        // No matching open → rendered literally, then HI
        assert_eq!(chars_text(&result), "{/red}HI");
        assert!(result.iter().all(|c| c.color.is_none()));
    }

    #[test]
    fn all_colors_supported() {
        let colors = [
            ("red", Color::Red),
            ("orange", Color::Orange),
            ("yellow", Color::Yellow),
            ("green", Color::Green),
            ("blue", Color::Blue),
            ("violet", Color::Violet),
            ("white", Color::White),
            ("black", Color::Black),
        ];
        for (name, expected) in colors {
            let input = format!("{{{name}}}X{{/{name}}}");
            let result = parse_color_markup(&input);
            assert_eq!(result.len(), 1, "failed for {name}");
            assert_eq!(result[0].color, Some(expected), "wrong color for {name}");
            assert_eq!(result[0].ch, 'X');
        }
    }

    #[test]
    fn case_insensitive_tags() {
        let result = parse_color_markup("{RED}HI{/RED}");
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            ColoredChar {
                ch: 'H',
                color: Some(Color::Red)
            }
        );
        assert_eq!(
            result[1],
            ColoredChar {
                ch: 'I',
                color: Some(Color::Red)
            }
        );
    }

    #[test]
    fn adjacent_tags() {
        let result = parse_color_markup("{red}A{/red}{blue}B{/blue}");
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            ColoredChar {
                ch: 'A',
                color: Some(Color::Red)
            }
        );
        assert_eq!(
            result[1],
            ColoredChar {
                ch: 'B',
                color: Some(Color::Blue)
            }
        );
    }

    #[test]
    fn color_with_spaces() {
        let result = parse_color_markup("{red}A B{/red}");
        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0],
            ColoredChar {
                ch: 'A',
                color: Some(Color::Red)
            }
        );
        assert_eq!(
            result[1],
            ColoredChar {
                ch: ' ',
                color: Some(Color::Red)
            }
        );
        assert_eq!(
            result[2],
            ColoredChar {
                ch: 'B',
                color: Some(Color::Red)
            }
        );
    }
}
