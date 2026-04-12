use ratatui::{
    Frame,
    layout::Constraint,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_popup::{KnownSizeWrapper, Popup};

use crate::{table_pane::TableColors, util};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PromptResponse {
    Ok,
    Cancel,
}

pub struct ErrorBox {
    msg: Box<str>,
    prompt: Option<Box<[(String, Style)]>>,
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
            prompt: Some(prompt.into_boxed_slice()),
            selected: None,
        }
    }

    pub fn render(&self, frame: &mut Frame, theme: &TableColors) {
        let area = util::center(
            frame.area(),
            Constraint::Min(40),
            Constraint::Percentage(50),
        );

        let mut msg_lines = vec![];
        for line in self.msg.lines() {
            msg_lines.push(Line::from(Span::styled(line, theme.log_err())));
        }
        let mut text = msg_lines;
        if let Some(p_lines) = self.prompt.as_deref() {
            text.push(Line::from(""));
            for (pl, pstyle) in p_lines {
                let style = if pl.is_empty() {
                    theme.row()
                } else if *pstyle == Style::default() {
                    theme.log_warn()
                } else {
                    theme.title().patch(*pstyle)
                };
                text.push(Line::from(Span::styled(pl, style)).centered());
            }
            let selected_ok = vec![
                Span::styled("<", theme.title()),
                Span::styled("OK", theme.title()),
                Span::styled(">", theme.title()),
                Span::raw("   "),
                Span::styled("CANCEL", theme.log_trace()),
                Span::raw(" "),
            ];
            let selected_cancel = vec![
                Span::raw(" "),
                Span::styled("OK", theme.log_trace()),
                Span::raw("   "),
                Span::styled("<", theme.title()),
                Span::styled("CANCEL", theme.title()),
                Span::styled(">", theme.title()),
            ];

            let select_text = match self.selected {
                Some(PromptResponse::Ok) => selected_ok,
                Some(PromptResponse::Cancel) => selected_cancel,
                None => vec![Span::styled(" OK    CANCEL ", theme.log_trace())],
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

        let paragraph = Paragraph::new(text).style(theme.base());
        let sized_paragraph = KnownSizeWrapper {
            inner: paragraph,
            width: max_width,
            height,
        };

        let popup = Popup::new(sized_paragraph)
            .title(Span::styled(Self::TITLE, theme.log_err()))
            .border_style(theme.success())
            .style(theme.base());

        frame.render_widget(&popup, area);
    }

    pub(crate) fn select(&self) -> Option<PromptResponse> {
        self.selected
    }
    pub(crate) fn navigate_toggle(&mut self) {
        self.selected = match self.selected {
            Some(s) => match s {
                PromptResponse::Ok => Some(PromptResponse::Cancel),
                PromptResponse::Cancel => Some(PromptResponse::Ok),
            },
            None => Some(PromptResponse::Cancel),
        }
    }
    pub(crate) fn navigate_left(&mut self) {
        self.selected = Some(PromptResponse::Ok);
    }
    pub(crate) fn navigate_right(&mut self) {
        self.selected = Some(PromptResponse::Cancel);
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
