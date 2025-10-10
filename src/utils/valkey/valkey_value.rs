use crate::utils::valkey::{Len, ToResp, ToVec, find_crlf};
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub enum ValkeyValue<'a> {
    SimpleString(&'a str),
    SimpleError(&'a str),
    Integer(i64),
    BulkString(Vec<u8>),
    Array(Vec<ValkeyValue<'a>>),

    Null,
    Boolean(bool),
    Double(f64),
    BigNumber(&'a str),
    BulkErrors(Vec<u8>),
    VerbatimString { format: &'a str, data: &'a str },
    Maps(HashMap<ValkeyValue<'a>, ValkeyValue<'a>>),
    Sets(Vec<ValkeyValue<'a>>),
    Pushes(Vec<ValkeyValue<'a>>),
}

impl PartialEq for ValkeyValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        use ValkeyValue::*;
        match (self, other) {
            (SimpleString(a), SimpleString(b)) => **a == **b,
            (SimpleError(a), SimpleError(b)) => **a == **b,
            (Integer(a), Integer(b)) => *a == *b,
            (BulkString(a), BulkString(b)) => *a == *b,
            (Array(a), Array(b)) => *a == *b,
            (Null, Null) => true,
            (Boolean(a), Boolean(b)) => *a == *b,
            (Double(a), Double(b)) => *a == *b,
            (BigNumber(a), BigNumber(b)) => **a == **b,
            (BulkErrors(a), BulkErrors(b)) => **a == **b,
            (
                VerbatimString {
                    format: fmt_a,
                    data: data_a,
                },
                VerbatimString {
                    format: fmt_b,
                    data: data_b,
                },
            ) => **fmt_a == **fmt_b && **data_a == **data_b,
            (Maps(_), Maps(_)) => true,
            (Sets(a), Sets(b)) => *a == *b,
            (Pushes(a), Pushes(b)) => *a == *b,
            _ => false,
        }
    }
}

impl Eq for ValkeyValue<'_> {}

impl Hash for ValkeyValue<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use ValkeyValue::*;
        std::mem::discriminant(self).hash(state);
        match self {
            SimpleString(s) => {
                s.hash(state);
            }
            SimpleError(s) => {
                s.hash(state);
            }
            Integer(i) => {
                i.hash(state);
            }
            BulkString(vec) => {
                vec.hash(state);
            }
            Array(vec) => {
                vec.hash(state);
            }
            Null => {}
            Boolean(b) => {
                b.hash(state);
            }
            Double(d) => {
                d.to_bits().hash(state);
            }
            BigNumber(s) => {
                s.hash(state);
            }
            BulkErrors(s) => {
                s.hash(state);
            }
            VerbatimString { format, data } => {
                format.hash(state);
                data.hash(state);
            }
            Maps(_) => {
                0x7f_ff_ff_ff_u32.hash(state);
            }
            Sets(vec) => {
                vec.hash(state);
            }
            Pushes(vec) => {
                vec.hash(state);
            }
        }
    }
}

impl ToVec for ValkeyValue<'_> {
    fn to_vec(&self) -> Vec<String> {
        if let ValkeyValue::Array(v) | ValkeyValue::Sets(v) | ValkeyValue::Pushes(v) = self {
            v.iter().flat_map(|v| v.to_vec()).collect()
        } else if let ValkeyValue::Maps(m) = self {
            m.iter()
                .flat_map(|(k, v)| vec![k.to_string(), v.to_string()])
                .collect()
        } else {
            vec![self.to_string()]
        }
    }
}

impl ToResp for ValkeyValue<'_> {
    fn to_resp(&self) -> String {
        use ValkeyValue::*;
        match self {
            SimpleString(s) => format!("+{}\r\n", *s),
            SimpleError(s) => format!("-{}\r\n", *s),
            Integer(i) => format!(":{}\r\n", *i),
            BulkString(v) => format!("${}\r\n{}\r\n", v.len(), String::from_utf8_lossy(v)),
            Array(v) => {
                let mut s = String::new();
                s.push('*');
                s.push_str(&v.len().to_string());
                s.push_str("\r\n");
                for item in v {
                    s.push_str(&item.to_resp());
                }
                s.to_string()
            }
            Null => "$-1\r\n".to_string(),
            Boolean(b) => format!("#{}\r\n", if *b { "t" } else { "f" }),
            Double(d) => format!(",{}\r\n", *d),
            BigNumber(s) => format!("({}\r\n", *s),
            BulkErrors(e) => format!("!{}\r\n{}\r\n", e.len(), String::from_utf8_lossy(e)),
            VerbatimString { format, data } => format!(
                "={}\r\n{}:{}\r\n",
                format.len() + data.len() + 1,
                *format,
                BulkString(data.as_bytes().to_vec())
            ),
            Maps(m) => {
                let mut s = String::new();
                for (k, v) in m.iter() {
                    s.push_str(&v.to_resp());
                    s.push_str(&k.to_resp());
                }
                format!("%{}\r\n{}", m.len(), s)
            }
            Sets(sets) => {
                let mut s = String::new();
                let mut sets = sets.clone();
                sets.dedup();
                for item in &sets {
                    s.push_str(&item.to_resp());
                }
                format!("~{}\r\n{}", sets.len(), s)
            }
            Pushes(pushes) => {
                let mut s = String::new();
                for push in pushes {
                    s.push_str(&push.to_resp());
                }
                format!(">{}\r\n{}", pushes.len(), s)
            }
        }
    }
}

impl ValkeyValue<'_> {
    pub fn parse_value<'b, I>(lines: &mut I) -> ValkeyValue<'b>
    where
        I: Iterator<Item = &'b str>,
    {
        use ValkeyValue::*;
        let header = match lines.next() {
            Some(l) => l,
            None => return Null,
        };

        let mut chars = header.chars();
        let first_char = chars.next().unwrap_or('_');

        match first_char {
            '+' => {
                let content = header.trim_start_matches('+').trim_end();
                SimpleString(content)
            }
            '-' => {
                let content = header.trim_start_matches('-').trim_end();
                SimpleError(content)
            }
            ':' => {
                let int_str = header.trim_start_matches(':').trim();
                match int_str.parse::<i64>() {
                    Ok(num) => Integer(num),
                    Err(_) => Null,
                }
            }
            '$' => {
                let length_str = header.trim_start_matches('$').trim();
                if let Ok(length) = length_str.parse::<i64>() {
                    if length < 0 {
                        Null
                    } else if let Some(data_line) = lines.next() {
                        let data = data_line.as_bytes().to_vec();
                        BulkString(data)
                    } else {
                        Null
                    }
                } else {
                    Null
                }
            }
            '*' => {
                let count_str = header.trim_start_matches('*').trim();
                let count: i64 = count_str.parse::<i64>().unwrap_or(-1);
                if count < 0 {
                    return Null;
                }
                let mut elements = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let elem = ValkeyValue::parse_value(lines);
                    elements.push(elem);
                }
                Array(elements)
            }
            '#' => {
                let bool_str = header.trim_start_matches('#').trim();
                Boolean(bool_str.eq_ignore_ascii_case("t"))
            }
            ',' => {
                let double_str = header.trim_start_matches(',').trim();
                match double_str.parse::<f64>() {
                    Ok(num) => Double(num),
                    Err(_) => Null,
                }
            }
            '(' => {
                let big_int_str = header.trim_start_matches('(').trim();
                BigNumber(big_int_str)
            }
            '!' => {
                let length_str = header.trim_start_matches('!').trim();
                if let Ok(length) = length_str.parse::<i64>() {
                    if length < 0 {
                        SimpleError("Error")
                    } else if let Some(data_line) = lines.next() {
                        let data = data_line.as_bytes().to_vec();
                        BulkErrors(data)
                    } else {
                        SimpleError("Error")
                    }
                } else {
                    SimpleError("Error")
                }
            }
            '=' => {
                let length_str = header.trim_start_matches('=').trim();
                if let Ok(length) = length_str.parse::<i64>() {
                    if length < 0 {
                        Null
                    } else if let Some(data_line) = lines.next() {
                        let data: Vec<_> = data_line.split(':').collect();
                        VerbatimString {
                            format: data.first().unwrap_or(&""),
                            data: data.last().unwrap_or(&""),
                        }
                    } else {
                        Null
                    }
                } else {
                    Null
                }
            }
            '%' => {
                let len_str = header.trim_start_matches('%').trim();
                let len = len_str.parse::<usize>().unwrap_or(0);
                let mut map = HashMap::with_capacity(len);
                for _ in 0..len {
                    let key = Self::parse_value(lines);
                    let value = Self::parse_value(lines);
                    map.insert(key, value);
                }
                Maps(map)
            }
            '~' => {
                let len_str = header.trim_start_matches('~').trim();
                let len = len_str.parse::<usize>().unwrap_or(0);
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    let elem = Self::parse_value(lines);
                    items.push(elem);
                }
                items.dedup();
                Sets(items)
            }
            '>' => {
                let len_str = header.trim_start_matches('>').trim();
                let len = len_str.parse::<usize>().unwrap_or(0);
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    let elem = Self::parse_value(lines);
                    items.push(elem);
                }
                Pushes(items)
            }
            _ => Null,
        }
    }

    pub fn parse_from_bytes(data: &[u8]) -> Result<(ValkeyValue<'_>, usize), &'static str> {
        Self::parse_value_from_bytes(data, 0)
    }

    fn parse_value_from_bytes(
        data: &[u8],
        start: usize,
    ) -> Result<(ValkeyValue<'_>, usize), &'static str> {
        if start >= data.len() {
            return Err("Incomplete data");
        }

        let first_byte = data[start];

        match first_byte {
            b'+' => {
                // Simple string
                if let Some(end_pos) = find_crlf(data, start) {
                    let content = std::str::from_utf8(&data[start + 1..end_pos])
                        .map_err(|_| "Invalid UTF-8 in simple string")?;
                    Ok((ValkeyValue::SimpleString(content), end_pos + 2))
                } else {
                    Err("Incomplete simple string")
                }
            }
            b'-' => {
                // Simple error
                if let Some(end_pos) = find_crlf(data, start) {
                    let content = std::str::from_utf8(&data[start + 1..end_pos])
                        .map_err(|_| "Invalid UTF-8 in simple error")?;
                    Ok((ValkeyValue::SimpleError(content), end_pos + 2))
                } else {
                    Err("Incomplete simple error")
                }
            }
            b':' => {
                // Integer
                if let Some(end_pos) = find_crlf(data, start) {
                    let int_str = std::str::from_utf8(&data[start + 1..end_pos])
                        .map_err(|_| "Invalid UTF-8 in integer")?;
                    let num = int_str.parse::<i64>().map_err(|_| "Invalid integer")?;
                    Ok((ValkeyValue::Integer(num), end_pos + 2))
                } else {
                    Err("Incomplete integer")
                }
            }
            b'$' => {
                // Bulk string
                if let Some(header_end) = find_crlf(data, start) {
                    let length_str = std::str::from_utf8(&data[start + 1..header_end])
                        .map_err(|_| "Invalid UTF-8 in bulk string header")?;
                    let length: i64 = length_str
                        .parse()
                        .map_err(|_| "Invalid bulk string length")?;

                    if length < 0 {
                        Ok((ValkeyValue::Null, header_end + 2))
                    } else {
                        let data_start = header_end + 2;
                        let data_end = data_start + length as usize;

                        if data_end + 2 <= data.len()
                            && data[data_end] == b'\r'
                            && data[data_end + 1] == b'\n'
                        {
                            let bulk_data = data[data_start..data_end].to_vec();
                            Ok((ValkeyValue::BulkString(bulk_data), data_end + 2))
                        } else {
                            Err("Incomplete bulk string data")
                        }
                    }
                } else {
                    Err("Incomplete bulk string header")
                }
            }
            b'*' => {
                // Array
                if let Some(header_end) = find_crlf(data, start) {
                    let count_str = std::str::from_utf8(&data[start + 1..header_end])
                        .map_err(|_| "Invalid UTF-8 in array header")?;
                    let count: i64 = count_str.parse().map_err(|_| "Invalid array count")?;

                    if count < 0 {
                        Ok((ValkeyValue::Null, header_end + 2))
                    } else {
                        let mut elements = Vec::with_capacity(count as usize);
                        let mut pos = header_end + 2;

                        for _ in 0..count {
                            let (element, new_pos) = Self::parse_value_from_bytes(data, pos)?;
                            elements.push(element);
                            pos = new_pos;
                        }

                        Ok((ValkeyValue::Array(elements), pos))
                    }
                } else {
                    Err("Incomplete array header")
                }
            }
            _ => Err("Unknown RESP type"),
        }
    }
}

impl<'a> From<&'a str> for ValkeyValue<'a> {
    fn from(value: &'a str) -> Self {
        match Self::parse_from_bytes(value.as_bytes()) {
            Ok((parsed_value, _)) => parsed_value,
            Err(_) => {
                // Fallback to line-based parsing for backward compatibility
                let mut lines = value.lines();
                Self::parse_value(&mut lines)
            }
        }
    }
}

/*
impl<'a> From<&'a str> for ValkeyValue<'a> {
    fn from(value: &'a str) -> Self {
        let mut lines = value.lines();
        Self::parse_value(&mut lines)
    }
}
 */

impl<'a> From<&'a String> for ValkeyValue<'a> {
    fn from(value: &'a String) -> Self {
        let mut lines = value.lines();
        Self::parse_value(&mut lines)
    }
}

impl ValkeyValue<'_> {
    pub fn parse_all_values(input: &str) -> Vec<ValkeyValue<'_>> {
        let mut lines = input.lines().peekable();
        let mut values = Vec::new();

        while lines.peek().is_some() {
            let value = Self::parse_value(&mut lines);
            values.push(value);
        }

        values
    }

    pub fn parse_complete(input: &str) -> ValkeyValue<'_> {
        let mut lines = input.lines();
        Self::parse_value(&mut lines)
    }
}

impl Display for ValkeyValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ValkeyValue::*;
        match self {
            SimpleString(s) => write!(f, "{}", *s),
            SimpleError(s) => write!(f, "{}", *s),
            Integer(i) => write!(f, "{}", *i),
            BulkString(v) => write!(f, "{}", String::from_utf8_lossy(v)),
            Array(v) => {
                let mut s = String::new();
                for item in v {
                    s.push_str(&item.to_resp());
                }
                write!(f, "{s}")
            }
            Boolean(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Double(d) => write!(f, "{}", *d),
            BigNumber(s) => write!(f, "{}", *s),
            BulkErrors(e) => write!(f, "{}", String::from_utf8_lossy(e)),
            VerbatimString { format, data } => write!(f, "{}:{}", *format, *data),
            Maps(m) => {
                let mut s = String::new();
                for (k, v) in m {
                    s.push_str(&k.to_resp());
                    s.push_str(&v.to_resp());
                }
                write!(f, "{s}")
            }
            Sets(sets) => {
                let mut s = String::new();
                let mut sets = sets.clone();
                sets.dedup();
                for item in sets {
                    s.push_str(&item.to_resp());
                }
                write!(f, "{s}")
            }
            Pushes(pushes) => {
                let mut s = String::new();
                for push in pushes {
                    s.push_str(&push.to_resp());
                }
                write!(f, "{s}")
            }
            _ => {
                write!(f, "")
            }
        }
    }
}

impl Len for ValkeyValue<'_> {
    fn len(&self) -> usize {
        use ValkeyValue::*;
        match self {
            SimpleString(_)
            | SimpleError(_)
            | Integer(_)
            | BulkString(_)
            | Null
            | Boolean(_)
            | Double(_)
            | BigNumber(_)
            | BulkErrors(_)
            | VerbatimString { .. } => 1,
            Array(v) | Sets(v) | Pushes(v) => v.len(),
            Maps(m) => m.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ValkeyValue::*;

    #[test]
    fn test_simple_string() {
        let input = "+OK\r\n";
        let expected = SimpleString("OK");
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_simple_error() {
        let input = "-Error message\r\n";
        let expected = SimpleError("Error message");
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_integer() {
        let input = ":1000\r\n";
        let expected = Integer(1000);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_bulk_string_non_nil() {
        let input = "$5\r\nhello\r\n";
        let expected = BulkString(b"hello".to_vec());
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_empty_bulk_string() {
        let input = "$0\r\n\r\n";
        let expected = BulkString(b"".to_vec());
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_empty_array() {
        let input = "*0\r\n";
        let expected = Array(vec![]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_simple_string_array() {
        let input = "*2\r\n+hello\r\n+world\r\n";
        let expected = Array(vec![SimpleString("hello"), SimpleString("world")]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_bulk_string_array() {
        let input = "*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let expected = Array(vec![
            BulkString("hello".as_bytes().to_vec()),
            BulkString("world".as_bytes().to_vec()),
        ]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_int_array() {
        let input = "*3\r\n:1\r\n:2\r\n:3\r\n";
        let expected = Array(vec![Integer(1), Integer(2), Integer(3)]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_mixed_array() {
        let input = "*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$5\r\nhello\r\n";
        let expected = Array(vec![
            Integer(1),
            Integer(2),
            Integer(3),
            Integer(4),
            BulkString("hello".as_bytes().to_vec()),
        ]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_nested_array() {
        let input = "*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Hello\r\n-World\r\n";
        let expected = Array(vec![
            Array(vec![Integer(1), Integer(2), Integer(3)]),
            Array(vec![SimpleString("Hello"), SimpleError("World")]),
        ]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_nulls() {
        let input = "_\r\n";
        let expected = Null;
        let result = ValkeyValue::from(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_bulk_string_nil() {
        let input = "$-1\r\n";
        let expected = Null;
        let result = ValkeyValue::from(input);
        assert_eq!(result, expected);
        assert_eq!(expected.to_resp(), result.to_resp());
    }

    #[test]
    fn test_null_arrays() {
        let input = "*-1\r\n";
        let expected = Null;
        let result = ValkeyValue::from(input);
        assert_eq!(result, expected);
        assert_eq!(expected.to_resp(), result.to_resp());
    }

    #[test]
    fn test_null_elements_in_array() {
        let input = "*3\r\n$5\r\nhello\r\n$-1\r\n$5\r\nworld\r\n";
        let expected = Array(vec![
            BulkString("hello".as_bytes().to_vec()),
            Null,
            BulkString("world".as_bytes().to_vec()),
        ]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_boolean_true() {
        let input = "#t\r\n";
        let expected = Boolean(true);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_boolean_false() {
        let input = "#f\r\n";
        let expected = Boolean(false);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_doubles() {
        let input = ",1.23\r\n";
        let expected = Double(1.23);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_doubles_with_int() {
        let input = ",10\r\n";
        let expected = Double(10.0);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_positive_infinity() {
        let input = ",inf\r\n";
        let expected = Double(f64::INFINITY);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_negative_infinity() {
        let input = ",-inf\r\n";
        let expected = Double(f64::NEG_INFINITY);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_not_a_number() {
        let input = ",nan\r\n";
        let result = ValkeyValue::from(input);
        match result {
            Double(result_value) => {
                assert!(result_value.is_nan(), "Expected NaN, got {}", result_value);
            }
            _ => panic!("Expected a ValkeyValue::Double variant"),
        }
    }

    #[test]
    fn test_big_numbers() {
        let input = "(3492890328409238509324850943850943825024385\r\n";
        let expected = BigNumber("3492890328409238509324850943850943825024385");
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_bulk_errors() {
        let input = "!21\r\nSYNTAX invalid syntax\r\n";
        let expected = BulkErrors("SYNTAX invalid syntax".as_bytes().to_vec());
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_verbatim_strings() {
        let input = "=15\r\ntxt:Some string\r\n";
        let expected = VerbatimString {
            format: "txt",
            data: "Some string",
        };
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_maps() {
        let input = "%2\r\n+first\r\n:1\r\n+second\r\n:2\r\n";
        let expected = Maps(HashMap::from([
            (SimpleString("first"), Integer(1)),
            (SimpleString("second"), Integer(2)),
        ]));
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        let output = ValkeyValue::from(&output);
        assert_eq!(result, expected);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_sets() {
        let input = "~2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let expected = Sets(vec![
            BulkString("hello".as_bytes().to_vec()),
            BulkString("world".as_bytes().to_vec()),
        ]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_sets_with_duplicates() {
        let input = "~3\r\n$5\r\nhello\r\n$5\r\nworld\r\n$5\r\nworld\r\n";
        let expected = Sets(vec![
            BulkString("hello".as_bytes().to_vec()),
            BulkString("world".as_bytes().to_vec()),
        ]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        let expected_output = "~2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        assert_eq!(result, expected);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_pushes() {
        let input = ">2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let expected = Pushes(vec![
            BulkString("hello".as_bytes().to_vec()),
            BulkString("world".as_bytes().to_vec()),
        ]);
        let result = ValkeyValue::from(input);
        let output = result.to_resp();
        assert_eq!(result, expected);
        assert_eq!(input, output);
    }

    #[test]
    fn test_invalid_input() {
        let input = "Some invalid input\r\n";
        let expected = Null;
        let result = ValkeyValue::from(input);
        assert_eq!(result, expected);
        assert_eq!(expected.to_resp(), result.to_resp());
    }
}
