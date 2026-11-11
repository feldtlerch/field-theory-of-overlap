// Resource preflight only. The established TOML parser still owns admission.
// Tokens are streamed; neither the file array nor an AST is allocated here.
struct Scan<'a> {
    source: &'a str,
    at: usize,
    look: Option<&'a str>,
    records: usize,
    maximum: usize,
}
impl<'a> Scan<'a> {
    fn take(&mut self) -> Option<&'a str> {
        if let Some(token) = self.look.take() {
            return Some(token);
        }
        let bytes = self.source.as_bytes();
        while let Some(&byte) = bytes.get(self.at) {
            if matches!(byte, b' ' | b'\t' | b'\r') {
                self.at += 1;
            } else if byte == b'#' {
                while bytes.get(self.at).is_some_and(|byte| *byte != b'\n') {
                    self.at += 1;
                }
            } else {
                break;
            }
        }
        let start = self.at;
        let &byte = bytes.get(start)?;
        self.at += 1;
        if matches!(byte, b'\'' | b'"') {
            let triple = bytes.get(start..start + 3).is_some_and(|s| s == [byte; 3]);
            if triple {
                self.at += 2;
            }
            while let Some(&next) = bytes.get(self.at) {
                if next == b'\\' && byte == b'"' {
                    self.at = (self.at + 2).min(bytes.len());
                    continue;
                }
                if next == byte
                    && (!triple
                        || bytes
                            .get(self.at..self.at + 3)
                            .is_some_and(|s| s == [byte; 3]))
                {
                    self.at += if triple { 3 } else { 1 };
                    if triple {
                        for _ in 0..2 {
                            if bytes.get(self.at) == Some(&byte) {
                                self.at += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    break;
                }
                self.at += 1;
            }
        } else if !b"\n[]=.,{}".contains(&byte) {
            while bytes
                .get(self.at)
                .is_some_and(|b| !b" \t\r\n#[]=.,{}\"'".contains(b))
            {
                self.at += 1;
            }
        }
        Some(&self.source[start..self.at])
    }
    fn peek(&mut self) -> Option<&'a str> {
        if self.look.is_none() {
            self.look = self.take();
        }
        self.look
    }
    fn key(&mut self) -> Option<Vec<String>> {
        let mut path = Vec::new();
        loop {
            let token = self.take()?;
            let key = if token.starts_with(['\'', '"']) {
                token.parse::<toml_edit::Value>().ok()?.as_str()?.to_owned()
            } else {
                if token.is_empty()
                    || !token
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                {
                    return None;
                }
                token.into()
            };
            path.push(key);
            if self.peek() != Some(".") {
                return Some(path);
            }
            self.take();
        }
    }
    fn count(&mut self) -> Result<(), ()> {
        if self.records == self.maximum {
            return Err(());
        }
        self.records += 1;
        Ok(())
    }
    fn value(&mut self, path: &[String], depth: usize) -> Result<bool, ()> {
        // Excessive malformed nesting is left to toml_edit's syntax limit.
        if depth > 128 {
            return Ok(false);
        }
        match self.take() {
            Some("[") => {
                let files = path == ["bundle", "files"];
                loop {
                    match self.peek() {
                        Some("]") => {
                            self.take();
                            return Ok(true);
                        }
                        Some("\n" | ",") => {
                            self.take();
                        }
                        None => return Ok(false),
                        _ => {
                            if files {
                                self.count()?;
                            }
                            if !self.value(&[], depth + 1)? {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
            Some("{") => loop {
                if self.peek() == Some("}") {
                    self.take();
                    return Ok(true);
                }
                let Some(key) = self.key() else {
                    return Ok(false);
                };
                if self.take() != Some("=") {
                    return Ok(false);
                }
                let mut child = path.to_vec();
                child.extend(key);
                if !self.value(&child, depth + 1)? {
                    return Ok(false);
                }
                if self.peek() == Some(",") {
                    self.take();
                } else if self.peek() != Some("}") {
                    return Ok(false);
                }
            },
            Some("\n" | "," | "]" | "}") | None => Ok(false),
            Some(_) => {
                while self
                    .peek()
                    .is_some_and(|s| !["\n", ",", "]", "}"].contains(&s))
                {
                    self.take();
                }
                Ok(true)
            }
        }
    }
}
pub(super) fn records(source: &str, maximum: usize) -> Result<(), ()> {
    let mut scan = Scan {
        source,
        at: 0,
        look: None,
        records: 0,
        maximum,
    };
    let mut table = Vec::new();
    while let Some(token) = scan.peek() {
        if token == "\n" {
            scan.take();
            continue;
        }
        if token == "[" {
            scan.take();
            let array = scan.peek() == Some("[");
            if array {
                scan.take();
            }
            let Some(path) = scan.key() else {
                return Ok(());
            };
            if scan.take() != Some("]") || (array && scan.take() != Some("]")) {
                return Ok(());
            }
            if array && path == ["bundle", "files"] {
                scan.count()?;
            }
            table = path;
        } else {
            let Some(key) = scan.key() else {
                return Ok(());
            };
            if scan.take() != Some("=") {
                return Ok(());
            }
            let mut path = table.clone();
            path.extend(key);
            if !scan.value(&path, 0)? {
                return Ok(());
            }
        }
    }
    Ok(())
}
