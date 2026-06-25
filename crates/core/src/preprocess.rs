use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PreprocessError {
    pub path: PathBuf,
    pub kind: PreprocessKind,
}

#[derive(Debug)]
pub enum PreprocessKind {
    Io(std::io::Error),
    Cyclic,
}

impl std::fmt::Display for PreprocessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            PreprocessKind::Io(e) => write!(f, "include {:?}: {}", self.path, e),
            PreprocessKind::Cyclic => write!(f, "cyclic include: {:?}", self.path),
        }
    }
}

impl std::error::Error for PreprocessError {}

pub fn preprocess(path: &Path) -> Result<String, PreprocessError> {
    let mut out = String::new();
    let mut stack: Vec<PathBuf> = Vec::new();
    preprocess_into(path, &mut out, &mut stack)?;
    Ok(out)
}

fn preprocess_into(
    path: &Path,
    out: &mut String,
    stack: &mut Vec<PathBuf>,
) -> Result<(), PreprocessError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if stack.contains(&canonical) {
        return Err(PreprocessError {
            path: canonical,
            kind: PreprocessKind::Cyclic,
        });
    }
    let content = fs::read_to_string(path).map_err(|e| PreprocessError {
        path: path.to_path_buf(),
        kind: PreprocessKind::Io(e),
    })?;
    stack.push(canonical);
    scan_and_substitute(&content, path, out, stack)?;
    stack.pop();
    Ok(())
}

fn scan_and_substitute(
    content: &str,
    base_dir: &Path,
    out: &mut String,
    stack: &mut Vec<PathBuf>,
) -> Result<(), PreprocessError> {
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // comment: ;; to end of line
        if bytes[i] == b';' && bytes.get(i + 1) == Some(&b';') {
            let line_end = bytes[i..]
                .iter()
                .position(|b| *b == b'\n')
                .map(|p| i + p + 1)
                .unwrap_or(bytes.len());
            out.push_str(&content[i..line_end]);
            i = line_end;
            continue;
        }
        // detect (include ...)
        if bytes[i] == b'('
            && bytes[i + 1..].starts_with(b"include")
            && is_boundary(bytes.get(i + 8).copied())
        {
            let form_start = i;
            // find closing )
            let mut depth = 1;
            let mut j = i + 1;
            let mut in_str = false;
            let mut str_hashes = 0;
            while j < bytes.len() && depth > 0 {
                let b = bytes[j];
                if in_str {
                    if b == b'"' && bytes[j + 1..j + 1 + str_hashes].iter().all(|x| *x == b'#') {
                        j += 1 + str_hashes;
                        in_str = false;
                        str_hashes = 0;
                        continue;
                    }
                    if str_hashes == 0 && b == b'\\' {
                        j += 2;
                        continue;
                    }
                    if str_hashes == 0 && b == b'"' {
                        in_str = false;
                        j += 1;
                        continue;
                    }
                    j += 1;
                    continue;
                }
                match b {
                    b';' if bytes.get(j + 1) == Some(&b';') => {
                        while j < bytes.len() && bytes[j] != b'\n' {
                            j += 1;
                        }
                    }
                    b'"' => {
                        in_str = true;
                        str_hashes = 0;
                        j += 1;
                    }
                    b'r' if bytes.get(j + 1) == Some(&b'#') => {
                        let mut h = 0;
                        let mut k = j + 1;
                        while bytes.get(k) == Some(&b'#') {
                            h += 1;
                            k += 1;
                        }
                        if bytes.get(k) == Some(&b'"') {
                            in_str = true;
                            str_hashes = h;
                            j = k + 1;
                        } else {
                            j += 1;
                        }
                    }
                    b'(' => {
                        depth += 1;
                        j += 1;
                    }
                    b')' => {
                        depth -= 1;
                        j += 1;
                    }
                    _ => j += 1,
                }
            }
            if depth != 0 {
                return Err(PreprocessError {
                    path: base_dir.to_path_buf(),
                    kind: PreprocessKind::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "unclosed include form",
                    )),
                });
            }
            let form_end = j; // position after ')'
            let inner = &content[form_start + 8..form_end - 1].trim();
            let inc_path = strip_quotes(inner);
            let resolved = base_dir.parent().unwrap_or(Path::new(".")).join(inc_path);
            preprocess_into(&resolved, out, stack)?;
            i = form_end;
            continue;
        }
        // default: copy one char
        let ch_end = next_char_end(bytes, i);
        out.push_str(&content[i..ch_end]);
        i = ch_end;
    }
    Ok(())
}

fn is_boundary(b: Option<u8>) -> bool {
    match b {
        Some(c) => c.is_ascii_whitespace(),
        None => true,
    }
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn next_char_end(bytes: &[u8], i: usize) -> usize {
    // UTF-8: lead byte tells length
    let b = bytes[i];
    let len = if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    };
    i + len
}
