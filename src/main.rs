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
    fs, io,
    io::ErrorKind,
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

    #[arg(long, help = "Launch the interactive terminal UI")]
    tui: bool,
}

fn main() {
    let cli = Cli::parse();

    let path = cli.path.unwrap_or_else(|| {
        PathBuf::from("/sys/class/power_supply/BAT0/charge_control_end_threshold")
    });

    if cli.tui {
        if cli.value.is_some() {
            eprintln!("Error: --value cannot be used with --tui");
            std::process::exit(1);
        }

        if let Err(err) = run_tui(path) {
            eprintln!("Failed to run TUI: {}", err);
            std::process::exit(1);
        }

        return;
    }

    if let Some(value) = cli.value {
        if value > 100 {
            eprintln!("Error: value must be between 0 and 100");
            std::process::exit(1);
        }

        if let Err(e) = write_threshold(&path, value) {
            eprintln!("Failed to write to {:?}: {}", path, e);
            std::process::exit(1);
        }

        println!("Battery charge threshold set to {}%", value);
    } else {
        match read_threshold(&path) {
            Ok(current) => println!("Current battery threshold: {}%", current),
            Err(e) => {
                eprintln!("Failed to read from {:?}: {}", path, e);
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
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Up | KeyCode::Char('+') => app.increment(),
                    KeyCode::Down | KeyCode::Char('-') => app.decrement(),
                    KeyCode::Enter => app.save(),
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

struct App {
    path: PathBuf,
    value: u8,
    status: Option<String>,
    error: Option<String>,
}

impl App {
    fn new(path: PathBuf) -> Self {
        let mut app = Self {
            path,
            value: 50,
            status: None,
            error: None,
        };

        match read_threshold(&app.path) {
            Ok(value) => app.value = value,
            Err(err) => {
                app.error = Some(format_read_error(&app.path, &err));
            }
        }

        app
    }

    fn increment(&mut self) {
        if self.value < 100 {
            self.value += 1;
        }
        self.status = None;
        self.error = None;
    }

    fn decrement(&mut self) {
        if self.value > 0 {
            self.value -= 1;
        }
        self.status = None;
        self.error = None;
    }

    fn save(&mut self) {
        match write_threshold(&self.path, self.value) {
            Ok(_) => {
                self.status = Some(format!("Battery threshold set to {}%", self.value));
                self.error = None;
            }
            Err(err) => {
                self.error = Some(format_write_error(&self.path, &err));
                self.status = None;
            }
        }
    }
}

fn draw_ui(frame: &mut Frame<'_>, app: &App) {
    let area = frame.size();

    let mut lines = vec![
        Line::from(format!("Path: {}", app.path.display())),
        Line::from(format!("Target threshold: {}%", app.value)),
        Line::from(""),
        Line::from("Use ↑/↓ or +/- to adjust. Press enter to save. Press q to quit."),
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

fn format_read_error(path: &Path, err: &io::Error) -> String {
    if err.kind() == ErrorKind::PermissionDenied {
        format!(
            "Permission denied reading {:?}. Try `sudo batty --tui` or adjust udev rules.",
            path
        )
    } else {
        format!("Failed to read from {:?}: {}", path, err)
    }
}

fn format_write_error(path: &Path, err: &io::Error) -> String {
    if err.kind() == ErrorKind::PermissionDenied {
        format!(
            "Permission denied writing to {:?}. Try `sudo batty --tui` or adjust udev rules.",
            path
        )
    } else {
        format!("Failed to write to {:?}: {}", path, err)
    }
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
