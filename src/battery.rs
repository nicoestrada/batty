use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Clone)]
pub enum BatteryStatus {
    Charging,
    NotCharging,
    Unknown,
}

impl BatteryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Charging => "charging",
            Self::NotCharging => "not charging",
            Self::Unknown => "unknown",
        }
    }
}

pub enum BatteryAttribute {
    CurrPower,
    TotalPower,
    Status,
    Cycles,
}

impl BatteryAttribute {
    fn file_name(&self) -> &'static str {
        match self {
            Self::CurrPower => "energy_now",
            Self::TotalPower => "energy_full",
            Self::Status => "status",
            Self::Cycles => "cycle_count",
        }
    }
}

impl fmt::Display for BatteryAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrPower => write!(f, "current power"),
            Self::TotalPower => write!(f, "total power"),
            Self::Status => write!(f, "status"),
            Self::Cycles => write!(f, "cycle count"),
        }
    }
}

pub struct Battery {
    path: PathBuf,
    pub total_power: u32,
    pub curr_power: u32,
    pub status: BatteryStatus,
    pub cycles: Option<u8>,
}

impl Battery {
    pub fn new(path: &Path) -> io::Result<(Self, Vec<String>)> {
        let mut warnings = Vec::new();
        let battery_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let curr_power: u32 = read_num_battery_attribute(path, BatteryAttribute::CurrPower)
            .map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!(
                        "Failed to read {} for {}: {}",
                        BatteryAttribute::CurrPower,
                        battery_name,
                        e
                    ),
                )
            })?;

        let total_power: u32 = read_num_battery_attribute(path, BatteryAttribute::TotalPower)
            .map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!(
                        "Failed to read {} for {}: {}",
                        BatteryAttribute::TotalPower,
                        battery_name,
                        e
                    ),
                )
            })?;

        let status = read_str_battery_attribute(path, BatteryAttribute::Status)
            .map(
                |status_str| match status_str.trim().to_lowercase().as_str() {
                    "charging" => BatteryStatus::Charging,
                    _ => BatteryStatus::NotCharging,
                },
            )
            .unwrap_or_else(|e| {
                warnings.push(format!(
                    "Failed to read status for {}: {}. Using 'unknown'.",
                    battery_name, e
                ));
                BatteryStatus::Unknown
            });

        let cycles: Option<u8> = read_num_battery_attribute(path, BatteryAttribute::Cycles).ok();
        Ok((
            Self {
                path: path.to_path_buf(),
                curr_power,
                total_power,
                status,
                cycles,
            },
            warnings,
        ))
    }

    pub fn refresh(&mut self) -> io::Result<Vec<String>> {
        let (battery, warnings) = Self::new(&self.path)?;
        *self = battery;
        Ok(warnings)
    }

    pub fn percentage(&self) -> f32 {
        (self.curr_power as f32 / self.total_power as f32) * 100.0
    }
}

pub fn find_batteries(power_supply_path: &PathBuf) -> Vec<PathBuf> {
    fs::read_dir(power_supply_path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with("BAT"))
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect()
}

fn read_num_battery_attribute<T>(bat_path: &Path, attr: BatteryAttribute) -> io::Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    let val = read_str_battery_attribute(bat_path, attr)?;
    let trimmed = val.trim();
    trimmed.parse::<T>().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid battery attribute value: {} ({})", trimmed, e),
        )
    })
}

fn read_str_battery_attribute(bat_path: &Path, attr: BatteryAttribute) -> io::Result<String> {
    let path = bat_path.join(attr.file_name());
    fs::read_to_string(&path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to read {}: {}", path.display(), e),
        )
    })
}
