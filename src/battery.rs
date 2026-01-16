//! Battery information reader for Linux ACPI/sysfs.
//!
//! Supports both energy-based (µWh) and charge-based (µAh) battery attributes.
//! The kernel exposes one or the other depending on the battery firmware's
//! power_unit setting. This module automatically detects and uses whichever
//! is available.
//!
//! Energy-based attributes (preferred):
//! - energy_now, energy_full, energy_full_design
//!
//! Charge-based attributes (fallback):
//! - charge_now, charge_full, charge_full_design
//!
//! Percentage calculations work with either unit since they're ratios.

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
    DesignPower,
}

impl BatteryAttribute {
    /// Returns possible file names for this attribute, in order of preference.
    /// First is energy-based (µWh), second is charge-based (µAh) where applicable.
    fn file_names(&self) -> &[&'static str] {
        match self {
            Self::CurrPower => &["energy_now", "charge_now"],
            Self::TotalPower => &["energy_full", "charge_full"],
            Self::DesignPower => &["energy_full_design", "charge_full_design"],
            Self::Status => &["status"],
            Self::Cycles => &["cycle_count"],
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
            Self::DesignPower => write!(f, "design power"),
        }
    }
}

pub struct Battery {
    path: PathBuf,
    pub total_power: u32,
    pub curr_power: u32,
    pub status: BatteryStatus,
    pub cycles: Option<u8>,
    pub battery_health: Option<f32>,
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

        let design_power: Option<u32> =
            read_num_battery_attribute(path, BatteryAttribute::DesignPower).ok();

        let battery_health: Option<f32> = match design_power {
            Some(design) if design > 0 => Some((total_power as f32 / design as f32) * 100.0),
            _ => {
                warnings.push(format!(
                    "Failed to read design power for {}. Battery health unavailable.",
                    battery_name
                ));
                None
            }
        };

        Ok((
            Self {
                path: path.to_path_buf(),
                curr_power,
                total_power,
                status,
                cycles,
                battery_health,
            },
            warnings,
        ))
    }

    pub fn refresh(&mut self) -> io::Result<Vec<String>> {
        let (battery, warnings) = Self::new(&self.path)?;
        *self = battery;
        Ok(warnings)
    }

    pub fn charge_percentage(&self) -> f32 {
        (self.curr_power as f32 / self.total_power as f32) * 100.0
    }

    pub fn health_percentage(&self) -> Option<f32> {
        self.battery_health
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
    let file_names = attr.file_names();
    let mut last_error = None;

    // Try each possible file name in order
    for file_name in file_names {
        let path = bat_path.join(file_name);
        match fs::read_to_string(&path) {
            Ok(content) => return Ok(content),
            Err(e) => {
                last_error = Some((path, e));
            }
        }
    }

    // If we tried multiple files, provide helpful error
    if file_names.len() > 1 {
        let tried_names: Vec<&str> = file_names.to_vec();
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Failed to read {} (tried: {})",
                attr,
                tried_names.join(", ")
            ),
        ))
    } else if let Some((path, e)) = last_error {
        Err(io::Error::new(
            e.kind(),
            format!("Failed to read {}: {}", path.display(), e),
        ))
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No file names configured for {}", attr),
        ))
    }
}
