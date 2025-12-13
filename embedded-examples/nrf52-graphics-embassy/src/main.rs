//! Full example code for setting up an SSD1322 display and drawing to it using
//! `embedded_graphics`. This runs on an NRF52840, using a Newhaven Displays NHD-2.7-12864WD*.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::{bind_interrupts, peripherals, spim};
use embassy_time::Timer;
use embedded_hal_bus::spi as spi_bus;

use display_interface_spi::SPIInterface;
use embedded_graphics::{
    framebuffer::{buffer_size, Framebuffer},
    pixelcolor::{raw::LittleEndian, Gray4},
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use ssd1322 as oled;

use panic_probe as _;

bind_interrupts!(struct Irqs {
  SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let pin_rst = p.P0_05;
    let pin_sck = p.P1_12;
    let pin_sdo = p.P1_14;
    let pin_cs = p.P1_15;
    let pin_dc = p.P1_13;
    let spi_instance = p.SPI3;

    // My dev board has a switchable power regulator, turn that on
    Output::new(p.P0_21, Level::High, OutputDrive::Standard).persist();

    // Assert the display's /RESET.
    let mut out_rst = Output::new(pin_rst, Level::Low, OutputDrive::Standard);
    Timer::after_millis(2).await;
    out_rst.set_high();
    Timer::after_millis(2).await;

    // Set up the push-pull outputs for CS and D/C signals.
    let out_cs = Output::new(pin_cs, Level::High, OutputDrive::Standard);
    let out_dc = Output::new(pin_dc, Level::High, OutputDrive::Standard);

    // Set up the SPI master interface.
    let mut spi_config = spim::Config::default();
    spi_config.frequency = spim::Frequency::M8;
    let spi_bus = spim::Spim::new_txonly(spi_instance, Irqs, pin_sck, pin_sdo, spi_config);
    let spi_dev = spi_bus::ExclusiveDevice::new(spi_bus, out_cs, embassy_time::Delay).unwrap();

    // Wrap all I/O in the `display-interface` SPI implementation.
    // This is what the `ssd1322` crate interacts with.
    let spi_iface = SPIInterface::new(spi_dev, out_dc);

    // Create the Display instance.
    let mut display = oled::DisplayAsync::new(
        spi_iface,
        oled::PixelCoord(256, 64), // double width because of duplicate pixels, see below
        oled::PixelCoord(56 * 2, 0),
    );

    display
        .init(
            oled::ConfigAsync::new(
                oled::ComScanDirection::RowZeroLast,
                oled::ComLayout::Progressive,
            )
            .column_remap(oled::command::ColumnRemap::Reverse)
            .clock_fosc_divset(9, 1)
            .display_enhancements(true, true)
            .contrast_current(0x7f)
            .phase_lengths(5, 15)
            .precharge_voltage(0x1f)
            .com_deselect_voltage(0x04),
        )
        .await
        .unwrap();

    let mut fb =
        Framebuffer::<Gray4, _, LittleEndian, 128, 64, { buffer_size::<Gray4>(128, 64) }>::new();

    let mut i = 0;
    let mut shade = 0;
    loop {
        // fb.clear(Gray4::BLACK).unwrap();

        if i > 128 + 64 {
            i = (i + 1) % 8;
            shade = (shade + 1) % 16;
        }

        let point = if i < 128 {
            Point::new(i, 0)
        } else {
            Point::new(127, i - 128)
        };

        Line::new(Point::new(0, 64), point)
            .into_styled(PrimitiveStyle::with_stroke(Gray4::new(16 - shade), 1))
            .draw(&mut fb)
            .unwrap();

        i += 8;

        // the NHD-2.7-12864WD is a little odd in that each visible pixel is driven as two
        // consecutive virtual pixels. This duplicates each nibble to account for that.
        let pixels = fb.data().iter().flat_map(|n| {
            let upper = n & 0xf0;
            let lower = n & 0x0f;
            [upper | (upper >> 4), lower | (lower << 4)]
        });

        // send framebuffer to display: get a region and send the packed pixel data.
        display
            .region(oled::PixelCoord(0, 0), oled::PixelCoord(256, 64))
            .unwrap()
            .draw_packed(pixels)
            .await
            .unwrap();

        Timer::after_millis(1).await;
    }
}
