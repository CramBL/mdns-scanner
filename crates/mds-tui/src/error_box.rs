use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::Constraint,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_popup::{Popup, SizedWrapper};

use crate::{components, message::Message, util};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptResponse {
    Ok,
    Cancel,
}

pub struct ErrorBox {
    msg: String,
    prompt: Option<Vec<(String, Style)>>,
    selected: Option<PromptResponse>,
}

impl ErrorBox {
    const TITLE: &str = "Error";

    pub fn new(msg: impl AsRef<str>) -> Self {
        Self {
            msg: msg.as_ref().into(),
            prompt: None,
            selected: None,
        }
    }

    pub fn with_prompt(self, prompt: Vec<(String, Style)>) -> Self {
        Self {
            msg: self.msg,
            prompt: Some(prompt),
            selected: None,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = util::center(
            frame.area(),
            Constraint::Min(40),
            Constraint::Percentage(50),
        );

        let mut msg_lines = vec![];
        for line in self.msg.lines() {
            msg_lines.push(Line::from(Span::styled(line, Style::new().red())));
        }
        let mut text = msg_lines;
        if let Some(p_lines) = self.prompt.as_deref() {
            text.push(Line::from(""));
            for (pl, pstyle) in p_lines {
                text.push(Line::from(Span::styled(pl, *pstyle)).centered());
            }
            let selected_ok = vec![
                Span::styled("<", Style::new().fg(Color::White)),
                Span::styled("OK", Style::new().fg(Color::White)),
                Span::styled(">", Style::new().fg(Color::White)),
                Span::raw("   "),
                Span::styled("CANCEL", Style::new().fg(Color::DarkGray)),
                Span::raw(" "),
            ];
            let selected_cancel = vec![
                Span::raw(" "),
                Span::styled("OK", Style::new().fg(Color::DarkGray)),
                Span::raw("   "),
                Span::styled("<", Style::new().fg(Color::White)),
                Span::styled("CANCEL", Style::new().fg(Color::White)),
                Span::styled(">", Style::new().fg(Color::White)),
            ];

            let select_text = match self.selected {
                Some(PromptResponse::Ok) => selected_ok,
                Some(PromptResponse::Cancel) => selected_cancel,
                None => vec![Span::styled(
                    " OK    CANCEL ",
                    Style::new().fg(Color::DarkGray),
                )],
            };

            let select_content = Line::from_iter(select_text).centered();
            text.push(Line::from(""));
            text.push(select_content);
        }

        let mut max_width = 0;
        for t in &text {
            if t.width() > max_width {
                max_width = t.width();
            }
        }
        let height = text.len();

        let paragraph = Paragraph::new(text);
        let sized_paragraph = SizedWrapper {
            inner: paragraph,
            width: max_width,
            height,
        };

        let popup = Popup::new(sized_paragraph)
            .title(Self::TITLE)
            .border_style(Style::new().green())
            .style(Style::new().red());

        frame.render_widget(&popup, area);
    }

    pub(crate) fn input(&mut self, key: KeyEvent) -> Option<PromptResponse> {
        match key.code {
            KeyCode::Enter => return self.selected,
            KeyCode::Left => {
                self.selected = Some(PromptResponse::Ok);
            }
            KeyCode::Right => self.selected = Some(PromptResponse::Cancel),
            _ => (),
        };
        None
    }
}

impl From<&str> for ErrorBox {
    fn from(err: &str) -> Self {
        Self::new(err)
    }
}

impl From<String> for ErrorBox {
    fn from(err: String) -> Self {
        Self::new(err)
    }
}

impl components::MdsKeyHandler for ErrorBox {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Message>, ErrorBox> {
        let resp = self.input(key).map(Message::PromptResponse);
        Ok(resp)
    }

    // Currently an error box is kept in an optional field so we can only call this
    // if it is `Some` and so it must be focused
    fn is_focused(&self) -> bool {
        true
    }
}
