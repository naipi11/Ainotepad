use crate::document::Document;

pub const SOFT_LIMIT_BYTES: u64 = 8 * 1024 * 1024;
pub const HARD_LIMIT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Gbk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewlineStyle {
    Lf,
    Crlf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeClass {
    Editable,
    ReadOnly,
    TooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    TooLarge,
    Decode(DecodeError),
}

pub fn classify_size(byte_len: u64) -> SizeClass {
    if byte_len > HARD_LIMIT_BYTES {
        SizeClass::TooLarge
    } else if byte_len > SOFT_LIMIT_BYTES {
        SizeClass::ReadOnly
    } else {
        SizeClass::Editable
    }
}

pub fn majority_newline(text: &str) -> NewlineStyle {
    let bytes = text.as_bytes();
    let mut crlf = 0usize;
    let mut lf = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            crlf += 1;
            i += 2;
        } else if bytes[i] == b'\n' {
            lf += 1;
            i += 1;
        } else {
            i += 1;
        }
    }
    if crlf > lf {
        NewlineStyle::Crlf
    } else {
        NewlineStyle::Lf
    }
}

pub fn decode_bytes(bytes: &[u8]) -> Result<(String, Encoding, NewlineStyle), DecodeError> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let text = std::str::from_utf8(&bytes[3..])
            .map_err(|_| DecodeError::Unsupported)?
            .to_string();
        let nl = majority_newline(&text);
        return Ok((text, Encoding::Utf8Bom, nl));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (cow, _, had_errors) = encoding_rs::UTF_16LE.decode(bytes);
        if had_errors {
            return Err(DecodeError::Unsupported);
        }
        let text = cow.into_owned();
        let nl = majority_newline(&text);
        return Ok((text, Encoding::Utf16Le, nl));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _, had_errors) = encoding_rs::UTF_16BE.decode(bytes);
        if had_errors {
            return Err(DecodeError::Unsupported);
        }
        let text = cow.into_owned();
        let nl = majority_newline(&text);
        return Ok((text, Encoding::Utf16Be, nl));
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        let nl = majority_newline(text);
        return Ok((text.to_string(), Encoding::Utf8, nl));
    }
    let (cow, _, had_errors) = encoding_rs::GBK.decode(bytes);
    if had_errors {
        return Err(DecodeError::Unsupported);
    }
    let text = cow.into_owned();
    let nl = majority_newline(&text);
    Ok((text, Encoding::Gbk, nl))
}

pub fn encode_text(text: &str, encoding: Encoding) -> Result<Vec<u8>, EncodeError> {
    match encoding {
        Encoding::Utf8 => Ok(text.as_bytes().to_vec()),
        Encoding::Utf8Bom => {
            let mut out = vec![0xEF, 0xBB, 0xBF];
            out.extend_from_slice(text.as_bytes());
            Ok(out)
        }
        Encoding::Utf16Le => {
            let mut out = vec![0xFF, 0xFE];
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_le_bytes());
            }
            Ok(out)
        }
        Encoding::Utf16Be => {
            let mut out = vec![0xFE, 0xFF];
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_be_bytes());
            }
            Ok(out)
        }
        Encoding::Gbk => {
            let (cow, _, had_errors) = encoding_rs::GBK.encode(text);
            if had_errors {
                return Err(EncodeError::Unsupported);
            }
            Ok(cow.into_owned())
        }
    }
}

impl Document {
    pub fn open_bytes(bytes: &[u8]) -> Result<Self, OpenError> {
        match classify_size(bytes.len() as u64) {
            SizeClass::TooLarge => return Err(OpenError::TooLarge),
            SizeClass::Editable | SizeClass::ReadOnly => {}
        }
        let (text, encoding, newline) =
            decode_bytes(bytes).map_err(OpenError::Decode)?;
        let mut doc = Document::from_text(text);
        doc.set_encoding(encoding);
        doc.set_newline_style(newline);
        if classify_size(bytes.len() as u64) == SizeClass::ReadOnly {
            doc.set_readonly(true);
        }
        doc.mark_clean();
        Ok(doc)
    }

    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        encode_text(&self.text(), self.encoding())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_classes_match_spec() {
        assert_eq!(classify_size(8 * 1024 * 1024), SizeClass::Editable);
        assert_eq!(classify_size(8 * 1024 * 1024 + 1), SizeClass::ReadOnly);
        assert_eq!(classify_size(16 * 1024 * 1024), SizeClass::ReadOnly);
        assert_eq!(classify_size(16 * 1024 * 1024 + 1), SizeClass::TooLarge);
    }

    #[test]
    fn utf8_round_trip() {
        let (text, enc, nl) = decode_bytes("hi\n".as_bytes()).unwrap();
        assert_eq!(text, "hi\n");
        assert_eq!(enc, Encoding::Utf8);
        assert_eq!(nl, NewlineStyle::Lf);
        assert_eq!(encode_text(&text, enc).unwrap(), b"hi\n");
    }

    #[test]
    fn utf8_bom_is_remembered() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("hi".as_bytes());
        let (text, enc, _) = decode_bytes(&bytes).unwrap();
        assert_eq!(text, "hi");
        assert_eq!(enc, Encoding::Utf8Bom);
        assert_eq!(encode_text(&text, enc).unwrap(), bytes);
    }

    #[test]
    fn gbk_round_trip_for_chinese() {
        let gbk = encoding_rs::GBK.encode("你好").0.into_owned();
        let (text, enc, _) = decode_bytes(&gbk).unwrap();
        assert_eq!(text, "你好");
        assert_eq!(enc, Encoding::Gbk);
        assert_eq!(encode_text(&text, enc).unwrap(), gbk);
    }

    #[test]
    fn crlf_majority_is_detected_but_not_rewritten() {
        let (text, _, nl) = decode_bytes(b"a\r\nb\r\nc\nd").unwrap();
        assert_eq!(nl, NewlineStyle::Crlf);
        assert_eq!(text, "a\r\nb\r\nc\nd");
        assert_eq!(encode_text(&text, Encoding::Utf8).unwrap(), b"a\r\nb\r\nc\nd");
    }

    #[test]
    fn too_large_file_is_refused() {
        let bytes = vec![0u8; (16 * 1024 * 1024 + 1) as usize];
        match Document::open_bytes(&bytes) {
            Err(OpenError::TooLarge) => {}
            other => panic!("expected TooLarge, got {other:?}"),
        }
    }
}
