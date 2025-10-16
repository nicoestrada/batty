use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Set or read battery charge threshold on ASUS laptops"
)]
struct Cli {
    #[arg(short, long)]
    path: Option<PathBuf>,

    #[arg(short, long)]
    value: Option<u8>,

    #[arg(
        short = 'k',
        long,
        default_value = "end",
        help = "Which threshold kind to set (start or end)"
    )]
    kind: String,

    #[arg(long, help = "Launch the interactive terminal UI")]
    tui: bool,
}

fn main() {
    let cli = Cli::parse();

    let bat0_path = cli
        .path
        .unwrap_or_else(|| PathBuf::from("/sys/class/power_supply/BAT0"));

    if cli.tui {
        if cli.value.is_some() {
            eprintln!("Error: --value cannot be used with --tui");
            std::process::exit(1);
        }

        if let Err(err) = run_tui(bat0_path) {
            eprintln!("Failed to run TUI: {}", err);
            std::process::exit(1);
        }

        return;
    }

    if let Some(value) = cli.value {
        let kind = match cli.kind.to_lowercase().as_str() {
            "start" => ThresholdKind::Start,
            "end" => ThresholdKind::End,
            _ => {
                eprintln!("Error: kind must be either 'start' or 'end'");
                std::process::exit(1);
            }
        };

        let mut thresholds = match Thresholds::load(&bat0_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to load current thresholds: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = thresholds.set(kind, value) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }

        if let Err(e) = thresholds.save(&bat0_path) {
            eprintln!("Failed to save thresholds: {}", e);
            std::process::exit(1);
        }

        println!("Battery charge {} threshold set to {}%", kind, value);
    } else {
        match Thresholds::load(&bat0_path) {
            Ok(thresholds) => {
                println!("Current battery thresholds:");
                println!("  Start: {}%", thresholds.start);
                println!("  End:   {}%", thresholds.end);
            }
            Err(e) => {
                eprintln!("Failed to read thresholds: {}", e);
                std::process::exit(1);
            }
        }
    }
}

type BattyBackend = CrosstermBackend<io::Stdout>;
type BattyTerminal = Terminal<BattyBackend>;

fn run_tui(path: PathBuf) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, path);
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

fn run_app(terminal: &mut BattyTerminal, path: PathBuf) -> io::Result<()> {
    let mut app = App::new(path);

    loop {
        terminal.draw(|frame| draw_ui(frame, &app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Up | KeyCode::Char('+') => app.increment(),
                    KeyCode::Down | KeyCode::Char('-') => app.decrement(),
                    KeyCode::Enter => app.save(),
                    KeyCode::Char('j') | KeyCode::Char('k') => app.select_next_threshold_kind(),
                    _ => {}
                }
            }
        }
    }
}

struct App {
    base_path: PathBuf,
    curr_threshold_kind: ThresholdKind,
    thresholds: Thresholds,
    status: Option<String>,
    error: Option<String>,
}

#[derive(PartialEq, Clone, Copy)]
enum ThresholdKind {
    Start,
    End,
}

impl fmt::Display for ThresholdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThresholdKind::Start => write!(f, "start"),
            ThresholdKind::End => write!(f, "end"),
        }
    }
}

struct Thresholds {
    start: u8,
    end: u8,
}

impl Thresholds {
    fn load(base_path: &Path) -> io::Result<Self> {
        let start_path = get_path_for_kind(base_path, &ThresholdKind::Start);
        let end_path = get_path_for_kind(base_path, &ThresholdKind::End);

        let start = read_threshold(&start_path)?;
        let end = read_threshold(&end_path)?;

        Ok(Self { start, end })
    }

    fn save(&self, base_path: &Path) -> io::Result<()> {
        let start_path = get_path_for_kind(base_path, &ThresholdKind::Start);
        let end_path = get_path_for_kind(base_path, &ThresholdKind::End);

        write_threshold(&start_path, self.start)?;
        write_threshold(&end_path, self.end)?;

        Ok(())
    }

    fn get(&self, kind: ThresholdKind) -> u8 {
        match kind {
            ThresholdKind::Start => self.start,
            ThresholdKind::End => self.end,
        }
    }

    fn set(&mut self, kind: ThresholdKind, value: u8) -> Result<(), String> {
        if value > 100 {
            return Err("threshold must be between 0 and 100".to_string());
        }

        match kind {
            ThresholdKind::Start => {
                if value >= self.end {
                    return Err("start threshold must be less than end threshold".to_string());
                }
                self.start = value;
            }
            ThresholdKind::End => {
                if value <= self.start {
                    return Err("end threshold must be greater than start threshold".to_string());
                }
                self.end = value;
            }
        }

        Ok(())
    }
}

impl Default for Thresholds {
    fn default() -> Self {
        Self { start: 40, end: 80 }
    }
}

impl App {
    fn new(path: PathBuf) -> Self {
        let thresholds = Thresholds::load(&path).unwrap_or_default();

        Self {
            curr_threshold_kind: ThresholdKind::Start,
            base_path: path,
            thresholds,
            status: None,
            error: None,
        }
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
}

fn draw_ui(frame: &mut Frame<'_>, app: &App) {
    let area = frame.size();

    let start_path = get_path_for_kind(&app.base_path, &ThresholdKind::Start);
    let end_path = get_path_for_kind(&app.base_path, &ThresholdKind::End);

    let start_selected = app.curr_threshold_kind == ThresholdKind::Start;

    let mut lines = vec![
        Line::from(format!("Path: {}", start_path.display())),
        Line::from(format_selected(
            start_selected,
            &format!("Start threshold: {}%", app.thresholds.start),
        )),
        Line::from(""),
        Line::from(format!("Path: {}", end_path.display())),
        Line::from(format_selected(
            !start_selected,
            &format!("End threshold: {}%", app.thresholds.end),
        )),
        Line::from(""),
        Line::from("• ↑/↓ or +/- : adjust thresholds. "),
        Line::from("• j/k: select threshold"),
        Line::from("• Enter: save"),
        Line::from("If saving fails, rerun with sudo or adjust udev permissions."),
    ];

    if let Some(status) = &app.status {
        lines.push(Line::from(status.clone()));
    }

    if let Some(error) = &app.error {
        lines.push(Line::from(vec![Span::styled(
            error.clone(),
            Style::default().fg(Color::Red),
        )]));
    }

    let widget = Paragraph::new(lines).block(Block::default().title("batty").borders(Borders::ALL));

    frame.render_widget(widget, area);
}

fn read_threshold(path: &Path) -> io::Result<u8> {
    let current = fs::read_to_string(path)?;
    let trimmed = current.trim();
    trimmed.parse::<u8>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid threshold value: {}", trimmed),
        )
    })
}

fn write_threshold(path: &Path, value: u8) -> io::Result<()> {
    fs::write(path, value.to_string())
}

fn get_path_for_kind(base_path: &Path, kind: &ThresholdKind) -> PathBuf {
    match kind {
        ThresholdKind::Start => base_path.join("charge_control_start_threshold"),
        ThresholdKind::End => base_path.join("charge_control_end_threshold"),
    }
}

fn format_selected(selected: bool, text: &str) -> String {
    if selected {
        format!("‣ {}", text)
    } else {
        format!("  {}", text)
    }
}
