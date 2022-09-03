#![no_main]
#![no_std]

use core::ops::{Range, RangeInclusive};

use common::{Host2TargetMessage, Target2HostMessage};
use cortex_m::peripheral::SCB;
use heapless::Vec;
use nrf52840_hal::{
    clocks::Clocks,
    gpio::{
        self,
        p0::{self, P0_06, P0_08, P0_14, P0_15, P0_16},
        Disconnected, Level, Output, PushPull,
    },
    prelude::*,
    usbd::{UsbPeripheral, Usbd},
};
use ramloader as _; // global logger + panicking-behavior + memory layout
use usb_device::device::{UsbDeviceBuilder, UsbVidPid};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

const RAM_PROGRAM_START_ADDRESS: u32 = 0x2002_0000;
const RAM_PROGRAM_END_ADDRESS: u32 = 0x2004_0000;
const VALID_RAM_PROGRAM_ADDRESS: RangeInclusive<u32> =
    RAM_PROGRAM_START_ADDRESS..=RAM_PROGRAM_END_ADDRESS;

#[cortex_m_rt::entry]
fn main() -> ! {
    let core_peripherals = cortex_m::Peripherals::take().unwrap();
    let nrf_peripherals = nrf52840_hal::pac::Peripherals::take().unwrap();
    let clocks = Clocks::new(nrf_peripherals.CLOCK);
    let clocks = clocks.enable_ext_hfosc();

    // let port0_pins = p0::Parts::new(nrf_peripherals.P0);

    let usb_bus = Usbd::new(UsbPeripheral::new(nrf_peripherals.USBD, &clocks));
    let mut serial_port = SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(USB_CLASS_CDC)
        .max_packet_size_0(64) // (makes control transfers 8x faster)
        .build();

    let mut rx_buffer = [0; 1024];
    let mut postcard_buffer = Vec::<_, { common::POSTCARD_BUFFER_SIZE }>::new();

    defmt::info!("ready to receive firmware image");
    loop {
        if !usb_dev.poll(&mut [&mut serial_port]) {
            continue;
        }
        if let Ok(count) = serial_port.read(&mut rx_buffer) {
            let byte = rx_buffer[count - 1];
            for b in &rx_buffer[..count] {
                postcard_buffer.push(*b).unwrap();
            }

            if byte == common::COBS_DELIMITER {
                let request: Host2TargetMessage =
                    postcard::from_bytes_cobs(&mut postcard_buffer).unwrap();

                let response = handle_request(request, &core_peripherals.SCB);
                let response_bytes =
                    postcard::to_vec_cobs::<_, { common::POSTCARD_BUFFER_SIZE }>(&response)
                        .unwrap();

                serial_port.write(&response_bytes).unwrap();
                postcard_buffer.clear();
            }
        }
    }
}

fn handle_request(request: Host2TargetMessage, scb: &SCB) -> Target2HostMessage {
    match request {
        Host2TargetMessage::Write {
            start_address,
            data,
        } => {
            let end_address = start_address + data.len() as u32;
            if is_valid_address_range(start_address..end_address) {
                let src = data.as_ptr();
                let dst = start_address as usize as *mut u8;
                let len = data.len();

                unsafe { core::ptr::copy_nonoverlapping(src, dst, len) }

                Target2HostMessage::WriteOk
            } else {
                defmt::error!(
                    "address range `{}..{}` is invalid",
                    start_address,
                    end_address
                );

                Target2HostMessage::InvalidAddress
            }
        }

        Host2TargetMessage::Execute => {
            defmt::info!("booting into new firmware...");

            // point VTOR to new vector table
            unsafe { scb.vtor.write(RAM_PROGRAM_START_ADDRESS) }

            //leds.off();

            // flush defmt messages
            cortex_m::asm::delay(1_000_000);

            unsafe { cortex_m::asm::bootload(RAM_PROGRAM_START_ADDRESS as *const u32) }
        }
    }
}

fn is_valid_address_range(range: Range<u32>) -> bool {
    VALID_RAM_PROGRAM_ADDRESS.contains(&range.start)
        && VALID_RAM_PROGRAM_ADDRESS.contains(&range.end)
}
