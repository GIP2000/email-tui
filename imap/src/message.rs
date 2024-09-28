use anyhow::{Context, Result};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone)]
pub struct Contact {
    name: Option<Box<str>>,
    email: Box<str>,
}

impl Display for Contact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{} ({})", name, self.email),
            None => write!(f, "{}", self.email),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: usize,
    pub subject: Box<str>,
    pub from: Contact,
    pub to: Option<Box<[Contact]>>,
    pub cc: Option<Box<[Contact]>>,
    pub bcc: Option<Box<[Contact]>>,
    pub read: bool,
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "From: {}", self.from)?;

        if let Some(to_list) = &self.to {
            let to_str = to_list
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(f, "To: {}", to_str)?;
        }
        if let Some(cc_list) = &self.cc {
            let cc_str = cc_list
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(f, "CC: {}", cc_str)?;
        }
        if let Some(bcc_list) = &self.bcc {
            let bcc_str = bcc_list
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(f, "BCC: {}", bcc_str)?;
        }

        writeln!(f, "Subject: {}", self.subject)?;

        return Ok(());
    }
}

impl FromStr for Contact {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        return Ok(match s.split_once('<') {
            Some((name, email)) => Self {
                name: Some(name.into()),
                email: email[0..email.len() - 1].into(),
            },
            None => Self {
                name: None,
                email: s.into(),
            },
        });
    }
}

impl FromStr for Message {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let id: usize = s
            .split(" ")
            .find_map(|word| word.parse().ok())
            .context("No ID found")?;

        let read = s.contains("\\Seen");

        let (subject, from, to, cc, bcc) = s.lines().skip(1).fold(
            (None, None, None, None, None),
            |(mut subject, mut from, mut to, mut cc, mut bcc), val| {
                if val.starts_with("Subject:") {
                    subject = Some(&val[9..]);
                }
                if val.starts_with("From:") {
                    from = val[6..].parse::<Contact>().ok()
                }
                if val.starts_with("To:") {
                    to = val[4..]
                        .split(",")
                        .map(|contact| contact.parse().ok())
                        .collect::<Option<Box<[Contact]>>>();
                }
                if val.starts_with("Cc:") {
                    cc = val[4..]
                        .split(",")
                        .map(|contact| contact.parse().ok())
                        .collect::<Option<Box<[Contact]>>>();
                }
                if val.starts_with("Bcc:") {
                    bcc = val[5..]
                        .split(",")
                        .map(|contact| contact.parse().ok())
                        .collect::<Option<Box<[Contact]>>>();
                }
                return (subject, from, to, cc, bcc);
            },
        );

        return Ok(Self {
            id,
            subject: subject.context("No subject found")?.into(),
            from: from.context("No From found")?,
            bcc,
            cc,
            to,
            read,
        });
    }
}
