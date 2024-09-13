use ::base64::write;
use anyhow::{bail, Context, Result};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

use openssl::{
    base64,
    ssl::{SslConnector, SslMethod, SslStream},
};

pub struct SMTP {
    stream: SslStream<TcpStream>,
    username: Option<Box<str>>,
}

impl SMTP {
    pub fn connect(server: &str, port: u32) -> Result<Self> {
        let connector = SslConnector::builder(SslMethod::tls())?.build();
        let stream = TcpStream::connect(format!("{}:{}", server, port))?;
        let stream = connector.connect(server, stream)?;
        let mut obj = Self {
            stream,
            username: None,
        };
        obj.check_response(220)?;
        return Ok(obj);
    }

    fn check_response(&mut self, expected_num: u32) -> Result<()> {
        let mut reader = BufReader::new(&mut self.stream);
        let num = loop {
            let mut buf = String::new();
            reader.read_line(&mut buf)?;
            let num = buf.split(' ').find_map(|x| x.parse::<u32>().ok());
            if let Some(num) = num {
                break num;
            }
        };
        if num == expected_num {
            Ok(())
        } else {
            bail!("{}", num);
        }
    }

    pub fn login(&mut self, username: Box<str>, password: &str) -> Result<()> {
        let (_, domain) = username
            .split_once("@")
            .context("Invalid domain not found")?;

        write!(self.stream, "EHLO {domain}\r\n")?;
        self.stream.flush()?;
        self.check_response(250)?;
        write!(self.stream, "AUTH LOGIN\r\n")?;
        self.check_response(334)?;
        let username_b64 = base64::encode_block(username.as_bytes());
        let password = base64::encode_block(password.as_bytes());
        write!(self.stream, "{}\r\n", username_b64)?;
        self.check_response(334)?;
        write!(self.stream, "{}\r\n", password)?;
        self.check_response(235)?;
        self.stream.flush()?;
        self.username = Some(username);
        return Ok(());
    }

    pub fn send_email(
        &mut self,
        to: &[&str],
        cc: Option<&[&str]>,
        bcc: Option<&[&str]>,
        subject: &str,
        body: &str,
    ) -> Result<()> {
        let username = self.username.as_deref().context("blah")?;
        write!(self.stream, "MAIL FROM:<{}>\r\n", username)?;
        self.stream.flush()?;
        self.check_response(250)?;

        for &recv in to
            .iter()
            .chain(cc.unwrap_or(&[]).iter())
            .chain(bcc.unwrap_or(&[]).iter())
        {
            write!(self.stream, "RCPT TO:<{}>\r\n", recv)?;
            self.stream.flush()?;
            self.check_response(250)?;
        }
        write!(self.stream, "DATA\r\n")?;
        self.stream.flush()?;
        self.check_response(354)?;
        write!(
            self.stream,
            "Subject: {}\r\nTo: {}\r\n",
            subject,
            to.join(", "),
        )?;
        if let Some(cc) = cc {
            write!(self.stream, "Cc: {}\r\n", cc.join(", "))?
        }
        if let Some(bcc) = bcc {
            write!(self.stream, "Bcc: {}\r\n", bcc.join(", "))?
        }
        write!(self.stream, "{}\r\n.\r\n", body)?;
        self.stream.flush()?;
        self.check_response(250)?;
        return Ok(());
    }
}

#[cfg(test)]
mod test {
    use dotenv::dotenv;

    use super::*;
    fn connect() -> SMTP {
        dotenv().ok();
        let mut smtp = SMTP::connect("smtp.gmail.com", 465).unwrap();
        smtp.login(
            std::env::var("EMAIL_USERNAME").unwrap().into(),
            &std::env::var("EMAIL_PASSWORD").unwrap(),
        )
        .unwrap();
        return smtp;
    }

    //     #[test]
    //     fn test_send_email() {
    //         let mut smtp = connect();
    //         smtp.send_email(
    //             &["gip.garbage@gmail.com"],
    //             None,
    //             None,
    //             "Test 2 email from rust",
    //             r#"-------------START-------------
    // THIS IS an email
    //
    // I like to send
    // emails
    //
    // --------------END_____________
    // "#,
    //         )
    //         .unwrap();
    //     }
}
