use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Obj(BTreeMap<String, Value>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(map) => map
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Value::Str(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Obj(_) => None,
        }
    }

    pub fn as_obj(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Obj(m) => Some(m),
            Value::Str(_) => None,
        }
    }

    pub fn path(&self, keys: &[&str]) -> Option<&str> {
        let mut cur = self;
        for k in keys {
            cur = cur.get(k)?;
        }
        cur.as_str()
    }
}

pub fn parse(text: &str) -> Option<Value> {
    let mut tokens = Tokenizer {
        chars: text.as_bytes(),
        pos: 0,
    };
    let _root_key = tokens.next_token()?;
    let body = tokens.next_token()?;
    if body != Token::OpenBrace {
        return None;
    }
    tokens.parse_object(0)
}

#[derive(Debug, PartialEq)]
enum Token {
    Str(String),
    OpenBrace,
    CloseBrace,
}

struct Tokenizer<'a> {
    chars: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn next_token(&mut self) -> Option<Token> {
        loop {
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.chars.get(self.pos) == Some(&b'/') && self.chars.get(self.pos + 1) == Some(&b'/')
            {
                while self.pos < self.chars.len() && self.chars[self.pos] != b'\n' {
                    self.pos += 1;
                }
                continue;
            }
            break;
        }

        let c = *self.chars.get(self.pos)?;
        match c {
            b'{' => {
                self.pos += 1;
                Some(Token::OpenBrace)
            }
            b'}' => {
                self.pos += 1;
                Some(Token::CloseBrace)
            }
            b'"' => {
                self.pos += 1;
                let mut out = Vec::new();
                while let Some(&ch) = self.chars.get(self.pos) {
                    match ch {
                        b'"' => {
                            self.pos += 1;
                            return Some(Token::Str(String::from_utf8_lossy(&out).into_owned()));
                        }
                        b'\\' => {
                            self.pos += 1;
                            match self.chars.get(self.pos) {
                                Some(b'n') => out.push(b'\n'),
                                Some(b't') => out.push(b'\t'),
                                Some(other) => out.push(*other),
                                None => break,
                            }
                            self.pos += 1;
                        }
                        _ => {
                            out.push(ch);
                            self.pos += 1;
                        }
                    }
                }
                None
            }
            _ => {
                let start = self.pos;
                while let Some(&ch) = self.chars.get(self.pos) {
                    if ch.is_ascii_whitespace() || ch == b'{' || ch == b'}' || ch == b'"' {
                        break;
                    }
                    self.pos += 1;
                }
                if start == self.pos {
                    return None;
                }
                Some(Token::Str(
                    String::from_utf8_lossy(&self.chars[start..self.pos]).into_owned(),
                ))
            }
        }
    }

    fn parse_object(&mut self, depth: usize) -> Option<Value> {
        if depth > 64 {
            return None;
        }
        let mut map = BTreeMap::new();
        loop {
            match self.next_token()? {
                Token::CloseBrace => return Some(Value::Obj(map)),
                Token::OpenBrace => return None,
                Token::Str(key) => match self.next_token()? {
                    Token::Str(v) => {
                        map.insert(key, Value::Str(v));
                    }
                    Token::OpenBrace => {
                        let obj = self.parse_object(depth + 1)?;
                        map.insert(key, obj);
                    }
                    Token::CloseBrace => return None,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APPMANIFEST: &str = r#"
"AppState"
{
	"appid"		"1778820"
	"name"		"TEKKEN 8"
	"installdir"		"TEKKEN 8"
	"LastUpdated"		"1779922432"
	"buildid"		"23440336"
	"InstalledDepots"
	{
		"1778821"
		{
			"manifest"		"123"
		}
	}
}
"#;

    #[test]
    fn reads_appmanifest_scalars() {
        let v = parse(APPMANIFEST).unwrap();
        assert_eq!(v.path(&["buildid"]), Some("23440336"));
        assert_eq!(v.path(&["installdir"]), Some("TEKKEN 8"));
        assert_eq!(v.path(&["InstalledDepots", "1778821", "manifest"]), Some("123"));
    }

    #[test]
    fn key_lookup_is_case_insensitive() {
        let v = parse(APPMANIFEST).unwrap();
        assert_eq!(v.path(&["BuildID"]), Some("23440336"));
        assert_eq!(v.path(&["lastupdated"]), Some("1779922432"));
    }

    #[test]
    fn reads_library_folders() {
        let text = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"apps"
		{
			"1778820"		"137201057480"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
		"apps"
		{
		}
	}
}
"#;
        let v = parse(text).unwrap();
        assert_eq!(v.path(&["0", "path"]), Some(r"C:\Program Files (x86)\Steam"));
        assert_eq!(v.path(&["1", "path"]), Some(r"D:\SteamLibrary"));
        assert!(v.path(&["0", "apps", "1778820"]).is_some());
    }

    #[test]
    fn tolerates_comments_and_unquoted_tokens() {
        let text = "\"root\"\n{\n// a comment\nkey value\n}\n";
        let v = parse(text).unwrap();
        assert_eq!(v.path(&["key"]), Some("value"));
    }

    #[test]
    fn rejects_garbage_instead_of_panicking() {
        assert!(parse("").is_none());
        assert!(parse("\"unterminated").is_none());
        assert!(parse("\"root\" {").is_none());
    }
}
