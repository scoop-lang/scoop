use std::path::PathBuf;

/// Read one GCC/Clang depfile with an explicitly selected, unescaped target.
pub fn parse_make_dependencies(input: &str, target: &str) -> Option<Vec<PathBuf>> {
    let input = input.strip_prefix(target)?.strip_prefix(':')?;
    let mut paths = Vec::new();
    let mut word = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.peek().copied()? {
                '\n' => {
                    chars.next();
                }
                '\r' => {
                    chars.next();
                    if chars.next() != Some('\n') {
                        return None;
                    }
                }
                ' ' | '\t' | '#' | ':' | '\\' => word.push(chars.next()?),
                // Clang on Windows emits ordinary path separators verbatim.
                // Make only consumes a backslash before a quoted delimiter.
                _ => word.push('\\'),
            },
            '$' if chars.peek() == Some(&'$') => {
                chars.next();
                word.push('$');
            }
            ch if ch.is_whitespace() => {
                if !word.is_empty() {
                    paths.push(PathBuf::from(std::mem::take(&mut word)));
                }
            }
            ch => word.push(ch),
        }
    }
    if !word.is_empty() {
        paths.push(PathBuf::from(word));
    }
    (!paths.is_empty()).then_some(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_clang_dependencies_preserve_unescaped_path_separators() {
        let input = concat!(
            "probe: E:\\scoop\\main.c \\\r\n",
            "  E:\\scoop\\include\\unwind.h C:\\Program\\ Files\\LLVM\\include\\stddef.h\r\n",
        );
        assert_eq!(
            parse_make_dependencies(input, "probe").unwrap(),
            vec![
                PathBuf::from(r"E:\scoop\main.c"),
                PathBuf::from(r"E:\scoop\include\unwind.h"),
                PathBuf::from(r"C:\Program Files\LLVM\include\stddef.h"),
            ]
        );
    }

    #[test]
    fn make_quoting_keeps_spaces_hashes_dollars_and_continuations() {
        let input = "probe: dir\\ name/header\\#$$.h \\\n    plain.h\n";
        assert_eq!(
            parse_make_dependencies(input, "probe").unwrap(),
            vec![
                PathBuf::from("dir name/header#$.h"),
                PathBuf::from("plain.h"),
            ]
        );
    }
}
