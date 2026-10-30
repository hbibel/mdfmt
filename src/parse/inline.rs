fn html_token_end(text: &str) -> Option<usize> {
    let end = if text.starts_with("<!--") {
        text.find("-->")? + 3
    } else if text.starts_with("<![CDATA[") {
        text.find("]]>")? + 3
    } else if text.starts_with("<?") {
        text.find("?>")? + 2
    } else {
        let name = text.strip_prefix('<')?.trim_start_matches('/');
        if !name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) && !text.starts_with("<!")
        {
            return None;
        }
        let mut quote = None;
        let mut end = None;
        for (index, ch) in text.char_indices().skip(1) {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                if matches!(ch, '\'' | '"') {
                    quote = Some(ch);
                } else if ch == '>' {
                    end = Some(index + 1);
                    break;
                }
            }
        }
        end?
    };
    let candidate = format!("x {}", &text[..end]);
    let tree = markdown::to_mdast(&candidate, &markdown::ParseOptions::gfm()).ok()?;
    fn is_html(node: &markdown::mdast::Node, end: usize) -> bool {
        if matches!(node, markdown::mdast::Node::Html(_))
            && node.position().is_some_and(|position| {
                position.start.offset == 2 && position.end.offset == end + 2
            })
        {
            return true;
        }
        node.children()
            .is_some_and(|children| children.iter().any(|child| is_html(child, end)))
    }
    is_html(&tree, end).then_some(end)
}

pub(crate) fn label_end(text: &str, code: bool) -> Option<usize> {
    if !text.starts_with('[') {
        return None;
    }
    let bytes = text.as_bytes();
    let mut cursor = 1;
    let mut depth = 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => {
                cursor += 1;
            }
            b'`' if code => {
                let count = bytes[cursor..]
                    .iter()
                    .take_while(|byte| **byte == b'`')
                    .count();
                let mut end = cursor + count;
                while end < bytes.len() {
                    if bytes[end] == b'`' {
                        let closing = bytes[end..]
                            .iter()
                            .take_while(|byte| **byte == b'`')
                            .count();
                        if closing == count {
                            cursor = end + count - 1;
                            break;
                        }
                        end += closing;
                    } else {
                        end += 1;
                    }
                }
                if end == bytes.len() {
                    cursor += count - 1;
                }
            }
            b'<' if code => {
                if let Some(end) = html_token_end(&text[cursor..]) {
                    cursor += end - 1;
                }
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}
