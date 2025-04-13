mod collect_ip;
pub(crate) mod constants;
pub(crate) mod host_up;
pub(crate) mod ip_info;
pub(crate) mod log;
pub(crate) mod scan_ip;
pub(crate) mod util;

use std::{cmp, sync::mpsc::Receiver, time::Duration};

use ip_info::{AccumulatedIpInfo, IpInfo};
use log::{LogLevel, LogMessage, Logger};
use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
};

use std::{io, sync::mpsc, thread};

use ratatui::{prelude::*, widgets::*};

const ITEM_HEIGHT: usize = 1;

use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

fn constraint_len_calculator(items: &[&IpInfo]) -> (u16, u16, u16) {
    let ip_len = items
        .iter()
        .map(|m| m.ip().to_string().width())
        .max()
        .unwrap_or(0);

    let hostname_len = items
        .iter()
        .map(|m| m.max_name_unicode_width())
        .max()
        .unwrap_or(0);

    let packets_count_len = items
        .iter()
        .map(|m| m.seen_count().to_string().width())
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (ip_len as u16, hostname_len as u16, packets_count_len as u16)
}

struct Model {
    state: TableState,
    scroll_state: ScrollbarState,
    colors: TableColors,
    running_state: RunningState,
    log_level: log::LogLevel,
    rx_mdns: Receiver<IpInfo>,
    acc_mdns_info: AccumulatedIpInfo,
    longest_item_lens: (u16, u16, u16), // order is (IP, name, seen count)
    rx_logs: Receiver<LogMessage>,
    log_msgs: Vec<LogMessage>,
    logger: Logger,
}

impl Default for Model {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let (tx_logs, rx_logs) = mpsc::channel();
        let local_logger = Logger::new(tx_logs, LogLevel::default());
        let background_logger = local_logger.clone();

        // Spawn the parser in a thread
        thread::spawn(move || {
            if let Err(e) = collect_ip::collect_ip_info(tx, background_logger) {
                eprintln!("Error in mDNS parser: {}", e);
            }
        });
        Self {
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            colors: TableColors::new(&PALETTES[0]),
            running_state: Default::default(),
            log_level: log::LogLevel::Info,
            rx_mdns: rx,
            acc_mdns_info: AccumulatedIpInfo::new(),
            longest_item_lens: (10, 10, 10),
            log_msgs: vec![],
            rx_logs: rx_logs,
            logger: local_logger,
        }
    }
}

impl Model {
    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.acc_mdns_info.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }
    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.acc_mdns_info.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }
    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let mut ip_info_vec: Vec<&IpInfo> = self
            .acc_mdns_info
            .collection()
            .iter()
            .map(|(_ip, mdns_info)| mdns_info)
            .collect();
        ip_info_vec.sort_unstable_by(|a, b| a.ip().cmp(&b.ip()));

        self.longest_item_lens = constraint_len_calculator(ip_info_vec.as_slice());
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = ["IP", "Name(s)", "Packets"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = ip_info_vec.iter().enumerate().map(|(i, ip_info)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let hostname_count = ip_info.names().len() as u16;
            let item = ip_info.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(cmp::max(2, hostname_count))
        });
        let bar = " █ ";
        let table_width = [
            // + 1 is for padding.
            Constraint::Length(self.longest_item_lens.0 + 1),
            Constraint::Min(self.longest_item_lens.1 + 1),
            Constraint::Min(self.longest_item_lens.2),
        ];
        let table = Table::new(rows, table_width)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(table, area, &mut self.state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[0]);
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(PartialEq)]
enum Message {
    IncreaseVerbosity,
    DecreaseVerbosity,
    NextRow,
    PreviousRow,
    Reset,
    Quit,
}

fn main() -> color_eyre::Result<()> {
    tui::install_panic_hook();
    let mut terminal = tui::init_terminal()?;
    let mut model = Model::default();

    while model.running_state != RunningState::Done {
        // Render the current view
        terminal.draw(|f| view(&mut model, f))?;

        // Handle events and map to a Message
        let mut current_msg = handle_event(&model)?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap());
        }

        while let Ok(m) = model.rx_mdns.try_recv() {
            model.acc_mdns_info.insert(m);
        }
        while let Ok(l) = model.rx_logs.try_recv() {
            model.log_msgs.push(l);
        }
    }

    tui::restore_terminal()?;
    Ok(())
}

fn view(model: &mut Model, frame: &mut Frame) {
    let [top, bottom] = Layout::vertical([Constraint::Fill(1); 2]).areas(frame.area());

    let logs = log::latest_messages(&model.log_msgs, model.log_level, 50);
    let mut list_items: Vec<ListItem> = vec![];
    for msg in logs {
        match msg {
            LogMessage::Error(s) => {
                list_items.push(ListItem::new(s.as_ref()).red());
            }
            LogMessage::Warn(s) => list_items.push(ListItem::new(s.as_ref()).yellow()),
            LogMessage::Info(s) => list_items.push(ListItem::new(s.as_ref())),
            LogMessage::Debug(s) => list_items.push(ListItem::new(s.as_ref()).cyan()),
            LogMessage::Trace(s) => list_items.push(ListItem::new(s.as_ref()).blue()),
        }
    }

    let list = List::new(list_items);

    frame.render_widget(
        list.block(Block::bordered().title(format!("Log Level: {}", model.log_level))),
        top,
    );

    model.set_colors();
    model.render_table(frame, bottom);
    model.render_scrollbar(frame, bottom);
}

/// Convert Event to Message
///
/// We don't need to pass in a `model` to this function in this example
/// but you might need it as your project evolves
fn handle_event(_: &Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                return Ok(handle_key(key));
            }
        }
    }
    Ok(None)
}

fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') => Some(Message::IncreaseVerbosity),
        KeyCode::Char('k') => Some(Message::DecreaseVerbosity),
        KeyCode::Down => Some(Message::NextRow),
        KeyCode::Up => Some(Message::PreviousRow),
        KeyCode::Char('q') => Some(Message::Quit),
        _ => None,
    }
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::IncreaseVerbosity => {
            model.log_level = model.log_level.increase();
            model.logger.increase_verbosity();
        }
        Message::DecreaseVerbosity => {
            model.log_level = model.log_level.decrease();
            model.logger.decrease_verbosity();
        }
        Message::NextRow => model.next_row(),
        Message::PreviousRow => model.previous_row(),
        Message::Reset => (),
        Message::Quit => {
            // You can handle cleanup and exit here
            model.running_state = RunningState::Done;
        }
    };
    None
}

mod tui {
    use ratatui::{
        Terminal,
        backend::{Backend, CrosstermBackend},
        crossterm::{
            ExecutableCommand,
            terminal::{
                EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
            },
        },
    };
    use std::{io::stdout, panic};

    pub fn init_terminal() -> color_eyre::Result<Terminal<impl Backend>> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(terminal)
    }

    pub fn restore_terminal() -> color_eyre::Result<()> {
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn install_panic_hook() {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            stdout().execute(LeaveAlternateScreen).unwrap();
            disable_raw_mode().unwrap();
            original_hook(panic_info);
        }));
    }
}
