/// Zero-alloc JSON scanner that provides lookups, extraction, and iteration
#[derive(Clone, Copy)]
pub struct JsonScan<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> JsonScan<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let mut s = Self { data, pos: 0 };
        s.skip_ws();
        s
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    #[inline]
    fn skip_ws(&mut self) {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    #[inline]
    fn skip_string(&mut self) -> bool {
        if self.peek() != Some(b'"') {
            return false;
        }
        self.pos += 1;
        loop {
            if self.pos >= self.data.len() {
                return false;
            }
            match self.data[self.pos] {
                b'\\' => self.pos += 2,
                b'"' => {
                    self.pos += 1;
                    return true;
                }
                _ => self.pos += 1,
            }
        }
    }

    fn skip_number(&mut self) -> bool {
        let start = self.pos;
        if self.pos < self.data.len() && self.data[self.pos] == b'-' {
            self.pos += 1;
        }
        if self.pos >= self.data.len() || !self.data[self.pos].is_ascii_digit() {
            self.pos = start;
            return false;
        }
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos < self.data.len() && self.data[self.pos] == b'.' {
            self.pos += 1;
            while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        if self.pos < self.data.len()
            && (self.data[self.pos] == b'e' || self.data[self.pos] == b'E')
        {
            self.pos += 1;
            if self.pos < self.data.len()
                && (self.data[self.pos] == b'+' || self.data[self.pos] == b'-')
            {
                self.pos += 1;
            }
            while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        true
    }

    #[inline]
    fn skip_literal(&mut self, lit: &[u8]) -> bool {
        if self.data[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            true
        } else {
            false
        }
    }

    fn skip_object(&mut self) -> bool {
        if self.peek() != Some(b'{') {
            return false;
        }
        self.pos += 1;
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return true;
        }
        loop {
            self.skip_ws();
            if !self.skip_string() {
                return false;
            }
            self.skip_ws();
            if self.peek() != Some(b':') {
                return false;
            }
            self.pos += 1;
            if !self.skip_value() {
                return false;
            }
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return true;
                }
                _ => return false,
            }
        }
    }

    fn skip_array(&mut self) -> bool {
        if self.peek() != Some(b'[') {
            return false;
        }
        self.pos += 1;
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return true;
        }
        loop {
            if !self.skip_value() {
                return false;
            }
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return true;
                }
                _ => return false,
            }
        }
    }

    fn skip_value(&mut self) -> bool {
        self.skip_ws();
        match self.peek() {
            Some(b'"') => self.skip_string(),
            Some(b'{') => self.skip_object(),
            Some(b'[') => self.skip_array(),
            Some(b't') => self.skip_literal(b"true"),
            Some(b'f') => self.skip_literal(b"false"),
            Some(b'n') => self.skip_literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.skip_number(),
            _ => false,
        }
    }

    pub fn field(&self, name: &str) -> Option<Self> {
        let mut scan = *self;
        scan.skip_ws();
        if scan.peek() != Some(b'{') {
            return None;
        }
        scan.pos += 1;
        scan.skip_ws();
        if scan.peek() == Some(b'}') {
            return None;
        }

        let target = name.as_bytes();
        loop {
            scan.skip_ws();
            if scan.peek() != Some(b'"') {
                return None;
            }
            let key_start = scan.pos + 1;
            if !scan.skip_string() {
                return None;
            }
            let key_end = scan.pos - 1;

            scan.skip_ws();
            if scan.peek() != Some(b':') {
                return None;
            }
            scan.pos += 1;
            scan.skip_ws();

            if &scan.data[key_start..key_end] == target {
                return Some(Self {
                    data: scan.data,
                    pos: scan.pos,
                });
            }

            if !scan.skip_value() {
                return None;
            }
            scan.skip_ws();
            match scan.peek() {
                Some(b',') => scan.pos += 1,
                Some(b'}') => return None,
                _ => return None,
            }
        }
    }

    #[inline]
    pub fn as_str(&self) -> Option<&'a str> {
        let mut pos = self.pos;
        if self.data.get(pos).copied() != Some(b'"') {
            return None;
        }
        pos += 1;
        let start = pos;
        loop {
            if pos >= self.data.len() {
                return None;
            }
            match self.data[pos] {
                b'\\' => pos += 2,
                b'"' => return core::str::from_utf8(&self.data[start..pos]).ok(),
                _ => pos += 1,
            }
        }
    }

    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        let mut scan = *self;
        scan.skip_ws();
        let start = scan.pos;
        scan.skip_number();
        if scan.pos == start {
            return None;
        }
        let s = core::str::from_utf8(&scan.data[start..scan.pos]).ok()?;
        s.parse().ok()
    }

    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        let mut scan = *self;
        scan.skip_ws();
        let start = scan.pos;
        if scan.pos < scan.data.len() && scan.data[scan.pos] == b'-' {
            scan.pos += 1;
        }
        while scan.pos < scan.data.len() && scan.data[scan.pos].is_ascii_digit() {
            scan.pos += 1;
        }
        if scan.pos == start {
            return None;
        }
        let s = core::str::from_utf8(&scan.data[start..scan.pos]).ok()?;
        s.parse().ok()
    }

    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        if self.data[self.pos..].starts_with(b"true") {
            Some(true)
        } else if self.data[self.pos..].starts_with(b"false") {
            Some(false)
        } else {
            None
        }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.data[self.pos..].starts_with(b"null")
    }

    #[inline]
    pub fn is_object(&self) -> bool {
        self.peek() == Some(b'{')
    }

    #[inline]
    pub fn is_string(&self) -> bool {
        self.peek() == Some(b'"')
    }

    pub fn array_iter(&self) -> Option<JsonArrayIter<'a>> {
        let mut scan = *self;
        scan.skip_ws();
        if scan.peek() != Some(b'[') {
            return None;
        }
        scan.pos += 1;
        scan.skip_ws();
        let done = scan.peek() == Some(b']');
        Some(JsonArrayIter {
            data: scan.data,
            pos: scan.pos,
            done,
        })
    }

    #[inline]
    pub fn first(&self) -> Option<Self> {
        let mut iter = self.array_iter()?;
        iter.next()
    }

    #[inline]
    pub fn index(&self, idx: usize) -> Option<Self> {
        let mut iter = self.array_iter()?;
        iter.nth(idx)
    }
}

pub struct JsonArrayIter<'a> {
    data: &'a [u8],
    pos: usize,
    done: bool,
}

impl<'a> Iterator for JsonArrayIter<'a> {
    type Item = JsonScan<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
        if self.pos >= self.data.len() || self.data[self.pos] == b']' {
            self.done = true;
            return None;
        }

        let item = JsonScan {
            data: self.data,
            pos: self.pos,
        };

        let mut skip = JsonScan {
            data: self.data,
            pos: self.pos,
        };
        if !skip.skip_value() {
            self.done = true;
            return Some(item);
        }
        skip.skip_ws();
        match skip.peek() {
            Some(b',') => self.pos = skip.pos + 1,
            _ => self.done = true,
        }

        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_lookup() {
        let data = br#"{"jsonrpc":"2.0","id":1,"result":42}"#;
        let scan = JsonScan::new(data);
        assert_eq!(scan.field("jsonrpc").unwrap().as_str(), Some("2.0"));
        assert_eq!(scan.field("id").unwrap().as_f64(), Some(1.0));
        assert_eq!(scan.field("result").unwrap().as_f64(), Some(42.0));
        assert!(scan.field("missing").is_none());
    }

    #[test]
    fn test_nested_object() {
        let data = br#"{"result":{"value":{"blockhash":"abc","lastValidBlockHeight":100}}}"#;
        let scan = JsonScan::new(data);
        let value = scan.field("result").unwrap().field("value").unwrap();
        assert_eq!(value.field("blockhash").unwrap().as_str(), Some("abc"));
        assert_eq!(
            value.field("lastValidBlockHeight").unwrap().as_f64(),
            Some(100.0)
        );
    }

    #[test]
    fn test_null_and_bool() {
        let data = br#"{"a":null,"b":true,"c":false}"#;
        let scan = JsonScan::new(data);
        assert!(scan.field("a").unwrap().is_null());
        assert_eq!(scan.field("b").unwrap().as_bool(), Some(true));
        assert_eq!(scan.field("c").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn test_array_iteration() {
        let data = br#"{"items":[1,2,3]}"#;
        let scan = JsonScan::new(data);
        let items: Vec<f64> = scan
            .field("items")
            .unwrap()
            .array_iter()
            .unwrap()
            .filter_map(|v| v.as_f64())
            .collect();
        assert_eq!(items, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_string_with_escapes() {
        let data = br#"{"msg":"hello \"world\""}"#;
        let scan = JsonScan::new(data);
        assert_eq!(
            scan.field("msg").unwrap().as_str(),
            Some(r#"hello \"world\""#)
        );
    }

    #[test]
    fn test_rpc_error_response() {
        let data =
            br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let scan = JsonScan::new(data);
        let err = scan.field("error").unwrap();
        assert!(err.is_object());
        assert_eq!(err.field("code").unwrap().as_i64(), Some(-32600));
        assert_eq!(
            err.field("message").unwrap().as_str(),
            Some("Invalid Request")
        );
    }

    #[test]
    fn test_empty_array() {
        let data = br#"{"result":[]}"#;
        let scan = JsonScan::new(data);
        let count = scan.field("result").unwrap().array_iter().unwrap().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_array_of_objects() {
        let data = br#"[{"pubkey":"abc","account":{"lamports":100}},{"pubkey":"def","account":{"lamports":200}}]"#;
        let scan = JsonScan::new(data);
        let mut iter = scan.array_iter().unwrap();
        let first = iter.next().unwrap();
        assert_eq!(first.field("pubkey").unwrap().as_str(), Some("abc"));
        assert_eq!(
            first
                .field("account")
                .unwrap()
                .field("lamports")
                .unwrap()
                .as_f64(),
            Some(100.0)
        );
        let second = iter.next().unwrap();
        assert_eq!(second.field("pubkey").unwrap().as_str(), Some("def"));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_negative_number() {
        let data = br#"{"code":-32600}"#;
        let scan = JsonScan::new(data);
        assert_eq!(scan.field("code").unwrap().as_i64(), Some(-32600));
    }

    #[test]
    fn test_array_index() {
        let data = br#"["base64data","base64"]"#;
        let scan = JsonScan::new(data);
        assert_eq!(scan.first().unwrap().as_str(), Some("base64data"));
        assert_eq!(scan.index(1).unwrap().as_str(), Some("base64"));
    }

    #[test]
    fn test_large_number() {
        let data = br#"{"slot":350000000}"#;
        let scan = JsonScan::new(data);
        assert_eq!(scan.field("slot").unwrap().as_f64(), Some(350000000.0));
    }
}
