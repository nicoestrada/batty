use crate::{
    battery::Battery,
    thresholds::{ThresholdKind, Thresholds},
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Flex, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use std::{io, path::PathBuf, time::Duration};

type BattyBackend = CrosstermBackend<io::Stdout>;
type BattyTerminal = Terminal<BattyBackend>;

pub fn run_tui(bat_paths: Vec<PathBuf>) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, bat_paths);
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> io::Result<BattyTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut BattyTerminal) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut BattyTerminal, bat_paths: Vec<PathBuf>) -> io::Result<()> {
    let mut app = App::new(bat_paths)?;

    loop {
        terminal.draw(|frame| draw_ui(frame, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Up | KeyCode::Char('+') => app.increment(),
                    KeyCode::Down | KeyCode::Char('-') => app.decrement(),
                    KeyCode::Enter => app.save(),
                    KeyCode::Char('j') | KeyCode::Char('k') => app.select_next_threshold_kind(),
                    KeyCode::Left | KeyCode::Char('[') => app.prev_tab(),
                    KeyCode::Right | KeyCode::Char(']') => app.next_tab(),
                    _ => {}
                }
            }
        }
    }
}

struct App {
    battery: Battery,
    bat_paths: Vec<PathBuf>,
    base_path: PathBuf,
    selected_tab: usize,
    curr_threshold_kind: ThresholdKind,
    thresholds: Thresholds,
    status: Option<String>,
    error: Option<String>,
    warnings: Vec<String>,
}

impl App {
    fn new(bat_paths: Vec<PathBuf>) -> io::Result<Self> {
        let initial_path = bat_paths[0].clone();
        let thresholds = Thresholds::load(&initial_path).unwrap_or_default();
        let (battery, warnings) = Battery::new(&initial_path)?;

        Ok(Self {
            battery,
            curr_threshold_kind: ThresholdKind::Start,
            base_path: initial_path,
            bat_paths,
            selected_tab: 0,
            thresholds,
            status: None,
            error: None,
            warnings,
        })
    }

    fn increment(&mut self) {
        let current = self.thresholds.get(self.curr_threshold_kind);
        let new_val = if current < 100 { current + 1 } else { current };

        match self.thresholds.set(self.curr_threshold_kind, new_val) {
            Ok(_) => {
                self.status = None;
                self.error = None;
            }
            Err(err) => {
                self.error = Some(err);
            }
        }
    }

    fn decrement(&mut self) {
        let current = self.thresholds.get(self.curr_threshold_kind);
        let new_val = current.saturating_sub(1);

        match self.thresholds.set(self.curr_threshold_kind, new_val) {
            Ok(_) => {
                self.status = None;
                self.error = None;
            }
            Err(err) => {
                self.error = Some(err);
            }
        }
    }

    fn save(&mut self) {
        match self.thresholds.save(&self.base_path) {
            Ok(_) => {
                self.status = Some(format!(
                    "Battery thresholds set to {}%-{}%",
                    self.thresholds.start, self.thresholds.end
                ));
                self.error = None;
            }
            Err(err) => {
                self.error = Some(format!("Failed to save thresholds: {}", err));
                self.status = None;
            }
        }
    }

    fn select_next_threshold_kind(&mut self) {
        match self.curr_threshold_kind {
            ThresholdKind::Start => self.curr_threshold_kind = ThresholdKind::End,
            ThresholdKind::End => self.curr_threshold_kind = ThresholdKind::Start,
        }
    }

    fn next_tab(&mut self) {
        if self.selected_tab < self.bat_paths.len() - 1 {
            self.selected_tab += 1;
            self.base_path = self.bat_paths[self.selected_tab].clone();
            self.thresholds = Thresholds::load(&self.base_path).unwrap_or_default();

            match Battery::new(&self.base_path) {
                Ok((battery, warnings)) => {
                    self.battery = battery;
                    self.warnings = warnings;
                    self.status = None;
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(format!("Failed to load battery: {}", e));
                    self.status = None;
                    self.warnings.clear();
                }
            }
        }
    }

    fn prev_tab(&mut self) {
        if self.selected_tab > 0 {
            self.selected_tab -= 1;
            self.base_path = self.bat_paths[self.selected_tab].clone();
            self.thresholds = Thresholds::load(&self.base_path).unwrap_or_default();

            match Battery::new(&self.base_path) {
                Ok((battery, warnings)) => {
                    self.battery = battery;
                    self.warnings = warnings;
                    self.status = None;
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(format!("Failed to load battery: {}", e));
                    self.status = None;
                    self.warnings.clear();
                }
            }
        }
    }
}

fn draw_ui(frame: &mut Frame<'_>, app: &mut App) {
    match app.battery.refresh() {
        Ok(warnings) => {
            app.warnings = warnings;
        }
        Err(e) => {
            app.error = Some(format!("Failed to refresh battery data: {}", e));
            app.warnings.clear();
        }
    }

    let show_tabs = app.bat_paths.len() > 1;
    let has_footer = !app.warnings.is_empty() || app.error.is_some() || app.status.is_some();

    // Calculate footer height based on number of lines needed
    let footer_height = if has_footer {
        let mut lines = 0;
        if app.error.is_some() {
            lines += 1;
        }
        if app.status.is_some() {
            lines += 1;
        }
        lines += app.warnings.len();
        (lines.min(3) + 2) as u16 // Add 2 for borders
    } else {
        0
    };

    // Main layout: tabs (optional), content, footer (optional)
    let main_layout = if show_tabs && has_footer {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(footer_height),
            ])
            .split(frame.size())
    } else if show_tabs {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(frame.size())
    } else if has_footer {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(footer_height)])
            .split(frame.size())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(frame.size())
    };

    // Render tabs at very top if multiple batteries
    if show_tabs {
        let tab_titles: Vec<String> = app
            .bat_paths
            .iter()
            .map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            })
            .collect();

        let tabs_widget = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Batteries"))
            .select(app.selected_tab)
            .style(Style::default())
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(tabs_widget, main_layout[0]);
    }

    // Get the area for the battery container
    let battery_container_area = if show_tabs {
        main_layout[1]
    } else {
        main_layout[0]
    };

    // Get battery name for the container title
    let battery_name = app
        .base_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Battery");

    // Create the main battery container block
    let battery_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", battery_name))
        .title_alignment(Alignment::Center)
        .style(Style::default());

    let inner_area = battery_block.inner(battery_container_area);
    frame.render_widget(battery_block, battery_container_area);

    // Layout inside the battery container: stats header + configuration
    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner_area);

    // Header stats layout
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .flex(Flex::SpaceAround)
        .split(inner_layout[0]);

    let bat_percent = format!("{:.2}%", app.battery.percentage());
    let percentage_widget = Paragraph::new(bat_percent)
        .block(
            Block::default()
                .title("Charge")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .centered();

    let status = app.battery.status.as_str();
    let status_widget = Paragraph::new(status)
        .block(
            Block::default()
                .title("Status")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .centered();

    let cycles = app
        .battery
        .cycles
        .map(|c| c.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let cycles_widget = Paragraph::new(cycles)
        .block(
            Block::default()
                .title("Cycles")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        )
        .centered();

    frame.render_widget(percentage_widget, header_layout[0]);
    frame.render_widget(status_widget, header_layout[1]);
    frame.render_widget(cycles_widget, header_layout[2]);

    let start_selected = app.curr_threshold_kind == ThresholdKind::Start;

    let mut lines = vec![
        Line::from(format_selected(
            start_selected,
            &format!("Start threshold: {}%", app.thresholds.start),
        )),
        Line::from(format_selected(
            !start_selected,
            &format!("End threshold:   {}%", app.thresholds.end),
        )),
        Line::from(""),
    ];

    if show_tabs {
        lines.push(Line::from("• ←/→ or [/]: switch battery tabs"));
    }

    lines.extend_from_slice(&[
        Line::from("• ↑/↓ or +/-: adjust thresholds"),
        Line::from("• j/k: select threshold"),
        Line::from("• Enter: save"),
        Line::from("If saving fails, rerun with sudo or adjust udev permissions."),
    ]);

    let config_widget = Paragraph::new(lines).block(
        Block::default()
            .title("Threshold Configuration")
            .borders(Borders::ALL),
    );

    frame.render_widget(config_widget, inner_layout[1]);

    // Render footer with warnings, errors, and status messages
    if has_footer {
        let footer_area = if show_tabs {
            main_layout[2]
        } else {
            main_layout[1]
        };

        let mut footer_lines = Vec::new();

        if let Some(error) = &app.error {
            footer_lines.push(Line::from(vec![Span::styled(
                format!("Error: {}", error),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]));
        }

        if let Some(status) = &app.status {
            footer_lines.push(Line::from(vec![Span::styled(
                status.clone(),
                Style::default().fg(Color::Green),
            )]));
        }

        for warning in &app.warnings {
            footer_lines.push(Line::from(vec![Span::styled(
                format!("Warning: {}", warning),
                Style::default().fg(Color::Yellow),
            )]));
        }

        let footer_widget = Paragraph::new(footer_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default()),
        );

        frame.render_widget(footer_widget, footer_area);
    }
}

fn format_selected(selected: bool, text: &str) -> String {
    if selected {
        format!("‣ {}", text)
    } else {
        format!("  {}", text)
    }
}
