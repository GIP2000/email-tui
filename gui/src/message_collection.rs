use anyhow::{Context, Result};
use imap::{message::Message, IMap};
use std::ops::Range;

pub struct MessageCollection {
    imap: IMap,
    messages: Vec<Message>,
    pub page_size: usize,
    pub current_page: usize,
}

impl MessageCollection {
    pub fn new(imap: IMap, page_size: usize) -> Self {
        return Self {
            imap,
            messages: vec![],
            page_size,
            current_page: 0,
        };
    }

    fn get_range_from_page(&self) -> Range<usize> {
        let start = self.current_page * self.page_size;
        let end = start + self.page_size;
        return start..end;
    }

    pub fn next_page(&mut self) {
        self.current_page += 1;
    }
    pub fn prev_page(&mut self) {
        self.current_page = self.current_page.saturating_sub(1);
    }

    pub fn get_body(&mut self, index: usize) -> Result<Box<str>> {
        let message_id = self.get_current_page()?[index].id;
        return self.imap.read_email(message_id);
    }

    pub fn get_current_page(&mut self) -> Result<&[Message]> {
        let range = self.get_range_from_page();
        if range.end <= self.messages.len() {
            return Ok(&self.messages[range]);
        }

        let inbox_count = self.imap.get_inbox_count()?;

        let last_loaded = self.messages.last().map(|x| x.id).unwrap_or(inbox_count);

        let headers = self
            .imap
            .get_n_email_headers((last_loaded - 1)..last_loaded - 19)?;

        self.messages.extend(headers.iter().rev().cloned());

        assert!(range.end <= self.messages.len());
        return Ok(&self.messages[range]);
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use anyhow::Context;
//     use dotenv::dotenv;
//
//     #[test]
//     fn test_next_page() -> Result<()> {
//         dotenv().ok();
//
//         let mut imap = IMap::connect("imap.gmail.com", 993)?;
//
//         let username = &std::env::var("EMAIL_USERNAME")?;
//         let password = &std::env::var("EMAIL_PASSWORD")?;
//         imap.login(username, password)?;
//
//         println!("Logged in");
//         let inbox = imap
//             .list_inbox()?
//             .into_iter()
//             .find(|x| &*x.name == "INBOX")
//             .context("Not inbox")?;
//
//         imap.select_inbox(inbox)?;
//
//         println!("selected");
//         let mut message_collection = MessageCollection::new(imap, 20);
//         println!("collection");
//
//         let page = message_collection.get_current_page()?;
//         println!(
//             "page ids {:?}",
//             page.iter().map(|x| x.id).collect::<Vec<_>>()
//         );
//         message_collection.next_page();
//         let page = message_collection.get_current_page()?;
//         println!(
//             "page ids {:?}",
//             page.iter().map(|x| x.id).collect::<Vec<_>>()
//         );
//
//         Ok(())
//     }
// }
