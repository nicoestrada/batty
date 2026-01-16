# Changelog

## [Unreleased]
### Fixed
- Added support for charge-based ACPI battery attributes (charge_now, charge_full)
- Fixed batty failing on systems that expose charge-based instead of energy-based attributes
- Battery reader now automatically detects and uses either energy (µWh) or charge (µAh) attributes

## [0.4.2] - 2025-11-06
### Added
- Added battery health percentage display in TUI header
- Changed battery percentage() to charge_percentage() and health_percentage() for clarity

## [0.4.1] - 2025-10-30
### Changed
-  Fixed error on writing to start threshold if path not available

## [0.4.0] - 2025-10-22
### Added
- Added battery statistics display in TUI header
- Current charge percentage
- Charging status (charging/not charging/unknown)
- Battery cycle count
- Multi-battery support with tab navigation
- Visual battery container with battery name in title
- Status footer for warnings, errors, and success messages

## [0.3.0] - 2025-10-17
### Added
- Support for both start and end battery charge thresholds.
- Enhanced TUI to manage start/end thresholds interactively.
### Changed
- Updated CLI to accept --kind (start/end) argument.

## [0.2.0] - 2025-10-10
- Initial TUI release.

## [0.1.0] - 2025-09-26
- Initial release.
