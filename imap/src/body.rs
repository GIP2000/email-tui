use anyhow::{bail, Context, Result};
use std::{io::Read, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMeta {
    file_type: Box<str>,
    name: Box<str>,
}

type NestedBodyStructure = (Box<[BodyStructure]>, Box<str>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyStructure {
    Plain,
    Html,
    Image(FileMeta),
    Application(FileMeta),
    Mixed(NestedBodyStructure),
    Related(NestedBodyStructure),
    Alternative(NestedBodyStructure),
}

#[derive(Clone)]
struct StrReader<'a> {
    val: &'a str,
    index: usize,
}

impl<'a> StrReader<'a> {
    fn new(val: &'a str) -> Self {
        Self { val, index: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.val.chars().nth(self.index)
    }

    fn read(&mut self) -> Option<char> {
        let result = self.val.chars().nth(self.index);
        if let Some(_) = result {
            self.index += 1
        }
        return result;
    }

    fn consume(&mut self, count: usize) {
        self.index = (self.index + count).min(self.val.len());
    }

    fn act_on_slice<Return, Closure>(&self, callback: Closure) -> Return
    where
        Closure: Fn(&str) -> Return,
    {
        return (callback)(&self.val[self.index..]);
    }

    fn get_quoted(&mut self) -> Option<&'a str> {
        let mut str_reader = self.clone();
        let starts_with_quote = str_reader.read().map(|x| x == '"').unwrap_or(false);
        let start = str_reader.index;
        if !starts_with_quote {
            return None;
        }
        loop {
            let val = if let Some(val) = str_reader.read() {
                val
            } else {
                return None;
            };
            let peek = str_reader.peek();

            match (val, peek) {
                ('\\', Some('"')) => {}
                (_, Some('"')) => {
                    str_reader.consume(1);
                    *self = str_reader;
                    return Some(&self.val[start..self.index - 1]);
                }
                _ => {}
            };
        }
    }

    fn consume_until_end_paren(&mut self) -> bool {
        let mut str_reader = self.clone();
        let mut p_count = 0;
        let mut in_quotes = false;
        loop {
            let val = if let Some(val) = str_reader.read() {
                val
            } else {
                return false;
            };
            let peek = str_reader.peek();

            match (val, peek, in_quotes) {
                ('(', _, false) => p_count += 1,

                (')', _, false) => p_count -= 1,

                ('"', _, false) => in_quotes = true,

                ('\\', Some('"'), true) => str_reader.consume(1),

                (_, Some('"'), true) => {
                    in_quotes = false;
                    str_reader.consume(1);
                }
                _ => {}
            };

            if p_count == -1 {
                *self = str_reader;
                return true;
            }
        }
    }
}

impl FromStr for BodyStructure {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let cmd = s
            .splitn(3, '(')
            .last()
            .context("Invalid couldn't find 2 '(' ")?;

        let mut reader = StrReader::new(cmd);
        let mut v: Vec<Self> = vec![];
        let mut splits = vec![0];

        while let Some(val) = reader.read() {
            let peek = reader.peek();
            let mut result = match (val, peek) {
                ('(', Some('(')) => {
                    splits.push(v.len());
                    None
                }
                ('T', _) => Self::parse_t(&mut reader),
                ('I', _) => Self::parse_i(&mut reader),
                ('A', _) => Self::parse_a(&mut reader),
                ('R', _) => Self::parse_r(&mut reader),
                ('M', _) => Self::parse_m(&mut reader),
                _ => None,
            };
            if let Some(BodyStructure::Alternative((arr, _)))
            | Some(BodyStructure::Mixed((arr, _)))
            | Some(BodyStructure::Related((arr, _))) = &mut result
            {
                let last_split = splits.pop().context("Invalid str couldn't find split")?;
                let (a, b) = v.split_at(last_split);
                *arr = b.into();
                v = a.into();
            }

            if let Some(x) = result {
                v.push(x);
            }
        }

        if v.len() != 1 {
            bail!("Error parsing")
        }
        // this unwrap is safe due to the above
        return Ok(v.pop().unwrap());
    }
}

impl BodyStructure {
    fn parse_t(str_reader: &mut StrReader) -> Option<Self> {
        let is_plain = str_reader.act_on_slice(|s| s.starts_with("EXT\" \"PLAIN\""));
        let is_html = str_reader.act_on_slice(|s| s.starts_with("EXT\" \"HTML\""));
        if !is_plain && !is_html {
            return None;
        }
        str_reader.consume(4);
        if !str_reader.consume_until_end_paren() {
            return None;
        }
        if is_html {
            return Some(Self::Html);
        }
        Some(Self::Plain)
    }

    fn parse_i(str_reader: &mut StrReader) -> Option<Self> {
        if !str_reader.act_on_slice(|s| s.starts_with("MAGE\" \"")) {
            return None;
        }
        let mut str_reader_copy = str_reader.clone();
        // skip the MAGE " "
        str_reader_copy.consume(6);
        let file_type = str_reader_copy.get_quoted()?.into();
        // skip the filetype and the end "
        if !str_reader_copy.act_on_slice(|s| s.starts_with(" (\"NAME\" \"")) {
            // if there is no name then we didn't find an image.
            return None;
        }
        str_reader_copy.consume(9);
        let name = str_reader_copy.get_quoted()?.into();

        if !str_reader_copy.consume_until_end_paren() {
            return None;
        }

        return Some(Self::Image(FileMeta { file_type, name }));
    }

    fn find_boundray<'a>(str_reader: &mut StrReader<'a>) -> Option<&'a str> {
        while let Some(val) = str_reader.read() {
            if val != '(' {
                continue;
            }
            if !str_reader.act_on_slice(|s| s.starts_with("\"BOUNDARY\" \"")) {
                return None;
            }
            str_reader.consume(11);
            return str_reader.get_quoted();
        }
        return None;
    }

    fn parse_a(str_reader: &mut StrReader) -> Option<Self> {
        if str_reader.act_on_slice(|s| s.starts_with("LTERNATIVE\"")) {
            let mut str_reader_copy = str_reader.clone();
            str_reader_copy.consume(11);
            let boundry = Self::find_boundray(&mut str_reader_copy)?.into();
            return Some(Self::Alternative((Default::default(), boundry)));
        }
        if !str_reader.act_on_slice(|s| s.starts_with("PPLICATION\" \"")) {
            return None;
        }
        let mut str_reader_copy = str_reader.clone();
        str_reader_copy.consume(12);
        let file_type = str_reader_copy.get_quoted()?.into();
        if !str_reader_copy.act_on_slice(|s| s.starts_with(" (\"NAME\") \"")) {
            return None;
        }
        str_reader_copy.consume(11);
        let name = str_reader_copy.get_quoted()?.into();
        if !str_reader_copy.consume_until_end_paren() {
            return None;
        }
        return Some(Self::Application(FileMeta { file_type, name }));
    }
    fn parse_r(str_reader: &mut StrReader) -> Option<Self> {
        if !str_reader.act_on_slice(|s| s.starts_with("ELATED\"")) {
            return None;
        }
        let mut str_reader_copy = str_reader.clone();
        str_reader_copy.consume(7);
        let boundry = Self::find_boundray(&mut str_reader_copy)?.into();
        return Some(Self::Related((Default::default(), boundry)));
    }

    fn parse_m(str_reader: &mut StrReader) -> Option<Self> {
        if !str_reader.act_on_slice(|s| s.starts_with("IXED\"")) {
            return None;
        }
        let mut str_reader_copy = str_reader.clone();
        str_reader_copy.consume(5);
        let boundry = Self::find_boundray(&mut str_reader_copy)?.into();
        return Some(Self::Mixed((Default::default(), boundry)));
    }
    pub fn find_text(&self) -> Option<Box<str>> {
        let (path, found) = BodyStructure::find_text_dfs(&self, vec![]);
        if !found {
            return None;
        }
        return Some(
            path.iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<String>>()
                .join(".")
                .into(),
        );
    }

    fn find_text_dfs(current: &BodyStructure, path: Vec<usize>) -> (Vec<usize>, bool) {
        use BodyStructure::*;
        return match current {
            Plain => (if path.is_empty() { vec![1] } else { path }, true),
            Html | Application(_) | Image(_) => (path, false),
            Alternative((arr, _)) | Mixed((arr, _)) | Related((arr, _)) => {
                let mut new_path = path.clone();
                new_path.push(1);
                for (i, el) in arr.iter().enumerate().map(|(i, x)| (i + 1, x)) {
                    // I just added a value so this should be safe
                    *new_path.last_mut().unwrap() = i;
                    let (resp_path, done) = BodyStructure::find_text_dfs(el, new_path.clone());
                    if done {
                        return (resp_path, true);
                    }
                }
                return (path, false);
            }
        };
    }
}

#[cfg(test)]
mod test {

    use super::*;
    const BS_STRING: &'static str = r#"* 123123 FETCH (BODYSTRUCTURE (("TEXT" "PLAIN" ("CHARSET" "utf-8") NIL NIL "QUOTED-PRINTABLE" 495 10 NIL NIL NIL)(("TEXT" "HTML" ("CHARSET" "utf-8") NIL NIL "QUOTED-PRINTABLE" 6328 127 NIL NIL NIL)("IMAGE" "PNG" ("NAME" "og-image.png" "X-UNIX-MODE" "0666") "<34A362DC-C052-41DA-B3C2-C6782B912403>" NIL "BASE64" 68590 NIL ("INLINE" ("FILENAME" "og-image.png")) NIL)("IMAGE" "PNG" ("NAME" "1*jtOTreOJuxO8FtLYyU9Uyw.png" "X-UNIX-MODE" "0666") "<E80B1254-3757-4EB9-AC92-C2E2EC312001>" NIL "BASE64" 180504 NIL ("INLINE" ("FILENAME" "1*jtOTreOJuxO8FtLYyU9Uyw.png")) NIL) "RELATED" ("BOUNDARY" "Apple-Mail=_A6722D8A-5BBB-478B-8940-7B14BCE39030" "TYPE" "text/html") NIL NIL) "ALTERNATIVE" ("BOUNDARY" "Apple-Mail=_D5EF70C3-5230-4B9A-A34D-20255319DA45") NIL NIL))
"#;

    #[test]
    fn test_bodystruct_parse() {
        use BodyStructure::*;
        let val: BodyStructure = BS_STRING.parse().unwrap();
        let expected_val = Alternative((
            Box::new([
                Plain,
                Related((
                    Box::new([
                        Html,
                        Image(FileMeta {
                            file_type: "PNG".into(),
                            name: "og-image.png".into(),
                        }),
                        Image(FileMeta {
                            file_type: "PNG".into(),
                            name: "1*jtOTreOJuxO8FtLYyU9Uyw.png".into(),
                        }),
                    ]),
                    "Apple-Mail=_A6722D8A-5BBB-478B-8940-7B14BCE39030".into(),
                )),
            ]),
            "Apple-Mail=_D5EF70C3-5230-4B9A-A34D-20255319DA45".into(),
        ));
        assert_eq!(val, expected_val);
    }
}
