//! Defines structs for storing register values of commands in the SSD1322 that are associated with
//! relatively-static configuration.

use crate::command::*;

#[cfg(feature = "async")]
use display_interface::AsyncWriteOnlyDataCommand;
use display_interface::{DisplayError, WriteOnlyDataCommand};

/// A configuration for the display. Builder methods offer a declarative way to either sent a
/// configuration command at init time, or to leave it at the chip's POR default.
#[maybe_async_cfg::maybe(sync(keep_self), async(feature = "async", idents(Command)))]
pub struct Config {
    com_scan_direction: ComScanDirection,
    com_layout: ComLayout,
    increment_axis: IncrementAxis,
    column_remap: ColumnRemap,
    nibble_remap: NibbleRemap,
    contrast_current_cmd: Option<Command>,
    phase_lengths_cmd: Option<Command>,
    clock_fosc_divset_cmd: Option<Command>,
    display_enhancements_cmd: Option<Command>,
    second_precharge_period_cmd: Option<Command>,
    precharge_voltage_cmd: Option<Command>,
    com_deselect_voltage_cmd: Option<Command>,
}

#[maybe_async_cfg::maybe(
    sync(keep_self),
    async(
        feature = "async",
        idents(Command, WriteOnlyDataCommand(async = "AsyncWriteOnlyDataCommand"))
    )
)]
impl Config {
    /// Create a new configuration. COM scan direction and COM layout are mandatory because the
    /// display will not function correctly unless they are set, so they must be provided in the
    /// constructor. All other options can be optionally set by calling the provided builder
    /// methods on `Config`.
    pub fn new(com_scan_direction: ComScanDirection, com_layout: ComLayout) -> Self {
        Config {
            com_scan_direction: com_scan_direction,
            com_layout: com_layout,
            increment_axis: IncrementAxis::Horizontal,
            column_remap: ColumnRemap::Forward,
            nibble_remap: NibbleRemap::Forward,
            contrast_current_cmd: None,
            phase_lengths_cmd: None,
            clock_fosc_divset_cmd: None,
            display_enhancements_cmd: None,
            second_precharge_period_cmd: None,
            precharge_voltage_cmd: None,
            com_deselect_voltage_cmd: None,
        }
    }

    /// Extend this `Config` to explicitly configure the increment axis. See
    /// `Command::SetRemapping`.
    pub fn increment_axis(self, increment_axis: IncrementAxis) -> Self {
        Self {
            increment_axis,
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure the column remapping. See
    /// `Command::SetRemapping`.
    pub fn column_remap(self, column_remap: ColumnRemap) -> Self {
        Self {
            column_remap,
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure the nibble remapping. See
    /// `Command::SetRemapping`.
    pub fn nibble_remap(self, nibble_remap: NibbleRemap) -> Self {
        Self {
            nibble_remap,
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure display contrast current. See
    /// `Command::SetContrastCurrent`.
    pub fn contrast_current(self, current: u8) -> Self {
        Self {
            contrast_current_cmd: Some(Command::SetContrastCurrent(current)),
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure OLED drive phase lengths. See
    /// `Command::SetPhaseLengths`.
    pub fn phase_lengths(self, reset: u8, first_precharge: u8) -> Self {
        Self {
            phase_lengths_cmd: Some(Command::SetPhaseLengths(reset, first_precharge)),
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure the display clock frequency and divider. See
    /// `Command::SetClockFoscDivset`.
    pub fn clock_fosc_divset(self, fosc: u8, divset: u8) -> Self {
        Self {
            clock_fosc_divset_cmd: Some(Command::SetClockFoscDivset(fosc, divset)),
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure display enhancement features. See
    /// `Command::SetDisplayEnhancements`.
    pub fn display_enhancements(self, external_vsl: bool, enhanced_low_gs_quality: bool) -> Self {
        Self {
            display_enhancements_cmd: Some(Command::SetDisplayEnhancements(
                external_vsl,
                enhanced_low_gs_quality,
            )),
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure OLED drive second precharge period length. See
    /// `Command::SetSecondPrechargePeriod`.
    pub fn second_precharge_period(self, period: u8) -> Self {
        Self {
            second_precharge_period_cmd: Some(Command::SetSecondPrechargePeriod(period)),
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure OLED drive precharge voltage. See
    /// `Command::SetPreChargeVoltage`.
    pub fn precharge_voltage(self, voltage: u8) -> Self {
        Self {
            precharge_voltage_cmd: Some(Command::SetPreChargeVoltage(voltage)),
            ..self
        }
    }

    /// Extend this `Config` to explicitly configure OLED drive COM deselect voltage. See
    /// `Command::SetComDeselectVoltage`.
    pub fn com_deselect_voltage(self, voltage: u8) -> Self {
        Self {
            com_deselect_voltage_cmd: Some(Command::SetComDeselectVoltage(voltage)),
            ..self
        }
    }

    /// Transmit commands to the display at `iface` necessary to put that display into the
    /// configuration encoded in `self`.
    pub(crate) async fn send<DI>(&self, iface: &mut DI) -> Result<(), CommandError<DisplayError>>
    where
        DI: WriteOnlyDataCommand,
    {
        Command::SetRemapping(
            self.increment_axis,
            self.column_remap,
            self.nibble_remap,
            self.com_scan_direction,
            self.com_layout,
        )
        .send(iface)
        .await?;

        for maybe_cmd in [
            self.phase_lengths_cmd,
            self.contrast_current_cmd,
            self.clock_fosc_divset_cmd,
            self.display_enhancements_cmd,
            self.second_precharge_period_cmd,
            self.precharge_voltage_cmd,
            self.com_deselect_voltage_cmd,
        ] {
            if let Some(cmd) = maybe_cmd {
                cmd.send(iface).await?
            }
        }
        Ok(())
    }
}
