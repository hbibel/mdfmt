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
