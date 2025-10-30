use std::{
    fmt,
    fs,
    io,
    path::{Path, PathBuf},
};

#[derive(PartialEq, Clone, Copy)]
pub enum ThresholdKind {
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

pub struct Thresholds {
    pub start: u8,
    pub end: u8,
}

impl Thresholds {
    pub fn load(base_path: &Path) -> io::Result<Self> {
        let start_path = get_path_for_kind(base_path, &ThresholdKind::Start);
        let end_path = get_path_for_kind(base_path, &ThresholdKind::End);

        let start = match read_threshold(&start_path) {
            Ok(value) => value,
            Err(err) if err.kind() == io::ErrorKind::NotFound => 0,
            Err(err) => return Err(err),
        };
        let end = read_threshold(&end_path)?;

        Ok(Self { start, end })
    }

    pub fn save(&self, base_path: &Path) -> io::Result<()> {
        let start_path = get_path_for_kind(base_path, &ThresholdKind::Start);
        let end_path = get_path_for_kind(base_path, &ThresholdKind::End);

        if start_path.exists() {
            write_threshold(&start_path, self.start)?;
        }
        write_threshold(&end_path, self.end)?;

        Ok(())
    }

    pub fn get(&self, kind: ThresholdKind) -> u8 {
        match kind {
            ThresholdKind::Start => self.start,
            ThresholdKind::End => self.end,
        }
    }

    pub fn set(&mut self, kind: ThresholdKind, value: u8) -> Result<(), String> {
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

pub fn get_path_for_kind(base_path: &Path, kind: &ThresholdKind) -> PathBuf {
    match kind {
        ThresholdKind::Start => base_path.join("charge_control_start_threshold"),
        ThresholdKind::End => base_path.join("charge_control_end_threshold"),
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
