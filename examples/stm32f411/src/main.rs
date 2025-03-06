#![deny(unsafe_code)]
#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use panic_halt as _; 
use cortex_m_rt::entry;
use ra8835a::Command;
use stm32f4xx_hal as hal;
use crate::hal::{pac, prelude::*};
use ra8835a_example::display;

#[entry]
fn main() -> ! {
    if let (Some(dp), Some(cp)) = (
        pac::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(100.MHz()).freeze();
        let gpioa = dp.GPIOA.split();
        let gpiob = dp.GPIOB.split();

        let d0 = gpioa.pa0.into_dynamic();
        let d1 = gpioa.pa1.into_dynamic();
        let d2 = gpioa.pa8.into_dynamic();
        let d3 = gpioa.pa9.into_dynamic();
        let d4= gpioa.pa4.into_dynamic();
        let d5= gpioa.pa5.into_dynamic();
        let d6= gpioa.pa6.into_dynamic();
        let d7= gpioa.pa7.into_dynamic();
        let data_bus = display::DataBus::new(d0, d1, d2, d3, d4, d5, d6, d7);

        let res = gpiob.pb0.into_push_pull_output_in_state(hal::gpio::PinState::High);
        let a0 = gpiob.pb1.into_push_pull_output_in_state(hal::gpio::PinState::High);
        let cs = gpiob.pb2.into_push_pull_output_in_state(hal::gpio::PinState::High);
        let rd = gpiob.pb3.into_push_pull_output_in_state(hal::gpio::PinState::High);
        let wr = gpiob.pb4.into_push_pull_output_in_state(hal::gpio::PinState::High);
        
        let mut delay = cp.SYST.delay(&clocks);
        delay.delay_ms(100);

        let mut display = display::LcdDisplay::new(data_bus, a0, wr, rd, cs, res, &mut delay).unwrap();

        display.driver.write_command(Command::CsrDirRight);
        display.driver.write_text_at("RA8835A", 220, 75);

        display.draw_rectangle(50, 50, 150, 150);
        display.draw_rectangle(100, 100, 200, 200);
        display.draw_line(50, 50, 200, 200);



        loop {}
    }
    loop {}
}
