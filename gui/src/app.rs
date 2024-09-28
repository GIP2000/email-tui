use crate::message_collection::MessageCollection;
use anyhow::{Context, Result};
use imap::IMap;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::Text,
    widgets::{List, Paragraph},
    Terminal,
};
use std::io::Stdout;

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    messages: MessageCollection,
    hovered_message: usize,
    selected_message: Option<usize>,
    selected_body: Option<Box<str>>,
    left: bool,
}

impl Drop for App {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

impl App {
    pub fn new() -> Result<Self> {
        let mut imap = IMap::connect("imap.gmail.com", 993)?;

        let username = &std::env::var("EMAIL_USERNAME")?;
        let password = &std::env::var("EMAIL_PASSWORD")?;
        imap.login(username, password)?;

        let inbox = imap
            .list_inbox()?
            .into_iter()
            .find(|x| &*x.name == "INBOX")
            .context("No inbox to select")?;

        imap.select_inbox(inbox)?;

        let messages = MessageCollection::new(imap, 40);

        return Ok(Self {
            terminal: ratatui::init(),
            messages,
            hovered_message: 0,
            selected_message: None,
            selected_body: None,
            left: true,
        });
    }

    pub fn render(&mut self) -> bool {
        let mut exit = false;

        let draw_success = self.terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(30), Constraint::Fill(1)])
                .split(frame.area());

            let page_size = self.messages.page_size;
            let current_page_idx = self.messages.current_page;
            let current_page = self.messages.get_current_page().unwrap_or(&[]);

            let list = List::new(current_page.iter().enumerate().map(|(i, x)| {
                let style = if i == self.hovered_message {
                    Style::default().on_blue()
                } else {
                    Style::default()
                };

                return Text::styled(
                    format!("{}. {} ", i + (page_size * current_page_idx), x.subject),
                    style,
                );
            }));

            let selected_meta = self.selected_message.map(|i| &current_page[i]);

            frame.render_widget(list, layout[0]);
            frame.render_widget(
                match (selected_meta, &self.selected_body) {
                    (Some(selected_meta), Some(body)) => {
                        Paragraph::new(format!("{}\n{}", selected_meta, body))
                    }
                    _ => Paragraph::new("Select an Email to view it here"),
                },
                layout[1],
            );
            exit = false;
        });

        return (match draw_success {
            Ok(_) => false,
            Err(_) => true,
        } || exit
            || self.handle_key_press());
    }

    fn put_body(&mut self) -> Result<()> {
        self.selected_body = self.messages.get_body(self.hovered_message).ok();
        self.selected_message = Some(self.hovered_message);
        return Ok(());
    }

    fn handle_key_press(&mut self) -> bool {
        if let Ok(event::Event::Key(key)) = event::read() {
            if key.kind == KeyEventKind::Press
                && ((key.modifiers.intersects(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c'))
                    || key.code == KeyCode::Char('q'))
            {
                return true;
            }

            if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                let _ = self.put_body();
            }

            if key.kind == KeyEventKind::Press
                && key.modifiers.intersects(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('h')
            {
                self.left = true;
            }

            if key.kind == KeyEventKind::Press
                && key.modifiers.intersects(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('l')
            {
                self.left = false;
            }

            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('j') {
                if self.left {
                    if self.hovered_message < self.messages.page_size - 1 {
                        self.hovered_message += 1;
                    } else {
                        self.messages.next_page();
                        self.hovered_message = 0;
                    }
                }
            }
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('k') {
                if self.left {
                    if self.hovered_message > 0 {
                        self.hovered_message -= 1;
                    } else if self.messages.current_page > 0 {
                        self.messages.prev_page();
                        self.hovered_message = self.messages.page_size - 1;
                    }
                }
            }
        }
        return false;
    }
}
