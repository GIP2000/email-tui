mod body;
mod inbox;
pub mod message;

use anyhow::{bail, Context, Result};
use body::BodyStructure;
use core::str;
use inbox::{Inbox, InboxRangeStr};
use message::Message;
use openssl::ssl::{SslConnector, SslMethod, SslStream};
use std::io::BufRead;
use std::ops::RangeBounds;
use std::str::FromStr;
use std::{
    io::{BufReader, Write},
    net::TcpStream,
};

pub struct IMap {
    stream: SslStream<TcpStream>,
    selected_inbox: Option<Inbox>,
}

impl IMap {
    pub fn connect(server: &str, port: u32) -> Result<Self> {
        let connector = SslConnector::builder(SslMethod::tls())?.build();
        let stream = TcpStream::connect(format!("{}:{}", server, port))?;
        let stream = connector.connect(server, stream)?;
        let mut obj = Self {
            stream,
            selected_inbox: None,
        };
        obj.drop_line()?;
        return Ok(obj);
    }

    pub fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let cmd = format!("? LOGIN \"{}\" \"{}\"", username, password);
        self.run_cmd(cmd.as_str())?;
        self.drop_line()?;
        return Ok(());
    }

    pub fn list_inbox(&mut self) -> Result<Vec<Inbox>> {
        let result = self.execute_cmd("? LIST \"*\" \"*\"")?;
        return result.trim_end().split('\n').map(Inbox::from_str).collect();
    }

    pub fn select_inbox(&mut self, inbox: Inbox) -> Result<()> {
        if !inbox.selectable {
            bail!("Error: Inbox not selectable")
        }
        _ = self.execute_cmd(format!("? SELECT \"{}\"", inbox.name).as_str())?;
        self.selected_inbox = Some(inbox);
        return Ok(());
    }

    pub fn get_inbox_count(&mut self) -> Result<usize> {
        let val = match &self.selected_inbox {
            Some(x) => &x.name,
            None => bail!(""),
        };
        let cmd = format!("? STATUS {} (MESSAGES)", val);
        let result = self.execute_cmd(cmd.as_str())?;

        // * STATUS INBOX (MESSAGES {NUMBER})
        let val = result
            .split_whitespace()
            .rev()
            .next()
            .context("No messages found")?;

        return val[0..val.len() - 1]
            .parse()
            .context("Invalid no number found");
    }

    pub fn get_n_email_headers<R: RangeBounds<usize>>(
        &mut self,
        range: R,
    ) -> Result<Box<[Message]>> {
        let InboxRangeStr(lhs, rhs) = range.into();
        let cmd = format!(
            "? FETCH {}:{} (FLAGS BODY.PEEK[HEADER.FIELDS (SUBJECT FROM TO CC BCC)])",
            lhs, rhs
        );
        let val = self.execute_cmd(cmd.as_str())?;
        return val.split("\n*").map(Message::from_str).collect();
    }

    pub fn get_body_structure(&mut self, id: usize) -> Result<BodyStructure> {
        let cmd = format!("? FETCH {} (BODYSTRUCTURE)", id);
        let raw_bodystruct = self.execute_cmd(cmd.as_str())?;
        return raw_bodystruct.parse();
    }

    pub fn read_email(&mut self, id: usize) -> Result<Box<str>> {
        let body_structue = self.get_body_structure(id)?;
        let section = body_structue.find_text().context("No Text found")?;
        let cmd = format!("? FETCH {} BODY[{}]", id, section);
        return self.execute_cmd(cmd.as_str());
    }

    fn read_response(&mut self) -> Result<Box<str>> {
        let mut result: String = String::new();
        let mut reader = BufReader::new(&mut self.stream);
        loop {
            let mut buf = Vec::new();
            let count = Self::readline(&mut reader, &mut buf)?;

            if count <= 0 {
                bail!("connection ended");
            }
            let resp = str::from_utf8(&buf)?;
            //TODO: read the spec this is based on observation
            if resp.starts_with("?") {
                if resp.contains("BAD") {
                    bail!("CMD FAILED: {}", resp.trim_end())
                }
                break;
            }
            result.push_str(resp)
        }
        return Ok(result.into());
    }

    fn drop_line(&mut self) -> Result<()> {
        let mut buf = Vec::new();
        let mut reader = BufReader::new(&mut self.stream);
        Self::readline(&mut reader, &mut buf)?;
        return Ok(());
    }

    fn readline(
        reader: &mut BufReader<&mut SslStream<TcpStream>>,
        buf: &mut Vec<u8>,
    ) -> Result<usize> {
        return reader
            .read_until(0x0a, buf)
            .context("Failed to read line from buffer");
    }

    fn run_cmd(&mut self, cmd: &str) -> Result<()> {
        write!(self.stream, "{}\r\n", cmd)?;
        self.stream.flush()?;
        return Ok(());
    }

    fn execute_cmd(&mut self, cmd: &str) -> Result<Box<str>> {
        self.run_cmd(cmd)?;
        return self.read_response();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use dotenv::dotenv;

    fn get_imap() -> IMap {
        dotenv().ok();
        let mut imap = IMap::connect("imap.gmail.com", 993).expect("Failed to connect");
        imap.login(
            &std::env::var("EMAIL_USERNAME").unwrap(),
            &std::env::var("EMAIL_PASSWORD").unwrap(),
        )
        .expect("Failed to login");
        return imap;
    }

    #[test]
    fn test_get_emails() {
        let mut imap = get_imap();
        let inboxes = imap.list_inbox().expect("Failed to get inboxes");
        let inbox = inboxes
            .into_iter()
            .find(|inbox| &*inbox.name == "INBOX")
            .expect("Could not find default inbox");
        imap.select_inbox(inbox)
            .expect("Invalid Failed to get inbox");
        let count = imap.get_inbox_count().expect("Failed to find number");
        let range = (count - 20)..;
        println!(
            "{:?}",
            imap.get_n_email_headers(range)
                .expect("Couldn't get email headers"),
        );
        println!("{}", imap.read_email(60830).expect("Failed to find email"));
    }
}
