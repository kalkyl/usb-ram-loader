//! LED blinks if interrupts are working

#![no_std]
#![no_main]

use embedded_graphics::{
    image::{Image, ImageRaw, ImageRawLE},
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};
use nrf52840_hal::{
    self as hal,
    gpio::{p0, Level},
    prelude::*,
};
use nrf52840_hal::{
    gpio::{p0::Parts, Output, Pin, PushPull},
    pac::TIMER0,
    prelude::*,
    spim::{Frequency, Pins, Spim, MODE_0},
    Timer,
};
use panic_halt as _;
use st7735_lcd;
use st7735_lcd::Orientation;

use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let periph = hal::pac::Peripherals::take().unwrap();
    let p0 = p0::Parts::new(periph.P0);
    let cs_pin = p0.p0_24.into_push_pull_output(Level::Low).degrade();
    let rst_pin = p0.p0_22.into_push_pull_output(Level::High).degrade();
    let dc_pin = p0.p0_20.into_push_pull_output(Level::High).degrade();
    let sda_pin = p0.p0_18.into_push_pull_output(Level::High).degrade();
    let sck_pin = p0.p0_15.into_push_pull_output(Level::High).degrade();
    let mut led = p0.p0_13.into_push_pull_output(Level::High).degrade();

    let mut timer = Timer::new(periph.TIMER1);
    let spim = Spim::new(
        periph.SPIM0,
        Pins {
            sck: sck_pin,
            mosi: Some(sda_pin),
            miso: None,
        },
        Frequency::M16,
        MODE_0,
        0,
    );
    let mut display = st7735_lcd::ST7735::new(spim, dc_pin, rst_pin, true, false, 160, 128);
    display.init(&mut timer).ok();

    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::RED)
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();

    display.set_orientation(&Orientation::Landscape).unwrap();
    // display.set_offset(0, 25);
    display.clear(Rgb565::BLACK).unwrap();
    // Draw a 3px wide outline around the display.
    display
        .bounding_box()
        .into_styled(border_stroke)
        .draw(&mut display)
        .ok();
    // draw ferris
    let image_raw: ImageRawLE<Rgb565> =
        ImageRaw::new(include_bytes!("../../../nrf-play/assets/ferris.raw"), 86);
    let image: Image<_> = Image::new(&image_raw, Point::new(34, 24));
    image.draw(&mut display).unwrap();

    loop {
        cortex_m::asm::nop();
    }
}
