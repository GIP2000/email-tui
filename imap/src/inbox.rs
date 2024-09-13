use anyhow::{Context, Result};
use std::ops::{Range, RangeBounds};
use std::str::FromStr;

#[derive(Debug)]
pub struct Inbox {
    pub name: Box<str>,
    pub selectable: bool,
    pub has_children: bool,
}

impl FromStr for Inbox {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // assume this is one line
        // format is `* LIST (\{FLAG} \{FLAG}...) "{PATH}" "{NAME}"`
        let (selectable, has_children) = s
            .split(" ")
            .skip(2)
            .take_while(|&s| s.ends_with(")"))
            .map(|mut word| {
                if word.starts_with('(') {
                    word = &word[1..];
                }
                if word.ends_with(')') {
                    word = &word[0..word.len() - 1];
                }
                return word;
            })
            .fold((true, true), |(mut selectable, mut has_children), flag| {
                if flag == "\\Noselect" {
                    selectable = false;
                } else if flag == "\\HasNoChildren" {
                    has_children = false;
                }
                return (selectable, has_children);
            });

        let name = s
            .split('"')
            .rev()
            .skip(1)
            .next()
            .context(format!("Couldn't find name for {}", s))?;

        return Ok(Self {
            name: name.into(),
            selectable,
            has_children,
        });
    }
}

pub struct InboxRangeStr(pub String, pub String);
impl<R: RangeBounds<usize>> From<R> for InboxRangeStr {
    fn from(value: R) -> Self {
        let lhs = match value.start_bound() {
            std::ops::Bound::Included(x) => format!("{}", x),
            std::ops::Bound::Excluded(x) => format!("{}", x + 1),
            std::ops::Bound::Unbounded => "*".to_owned(),
        };
        let rhs = match value.end_bound() {
            std::ops::Bound::Included(x) => format!("{}", x),
            std::ops::Bound::Excluded(x) => format!("{}", x - 1),
            std::ops::Bound::Unbounded => "*".to_owned(),
        };

        return Self(lhs, rhs);
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse() {
        let test = "* LIST (\\HasNoChildren) \"/\" \"Deleted Messages\"";
        let inbox: Inbox = test.parse().expect("Inbox parse fails");
        assert_eq!(&*inbox.name, "Deleted Messages");
        assert!(!inbox.has_children);
        assert!(inbox.selectable);
    }
}
