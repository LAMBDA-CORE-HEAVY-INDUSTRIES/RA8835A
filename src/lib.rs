#![no_std]
#![deny(unsafe_code)]

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

#[repr(u8)]
enum Command {
    /// Initialize device and display.
    SystemSet = 0x40,
    /// Enter standby mode.
    SleepIn = 0x53,
    /// Display off.
    DisplayOff = 0x58,
    /// Display on.
    DisplayOn = 0x59,
    /// Set display start address and display regions.
    Scroll = 0x44,
    /// Set cursor type.
    CsrForm = 0x5D,
    /// Set start address of character generator RAM.
    CgRamAdr = 0x5C,
    /// Set direction of cursor movement.
    CsrDirRight = 0x4C,
    /// Set direction of cursor movement.
    CsrDirLeft = 0x4D,
    /// Set direction of cursor movement.
    CsrDirUp = 0x4E,
    /// Set direction of cursor movement.
    CsrDirDown = 0x4F,
    /// Set horizontal scroll position.
    HdotScr = 0x5A,
    /// Set display overlay format.
    Ovlay = 0x5B,
    /// Set cursor address.
    Csrw = 0x46,
    /// Read cursor address.
    Csrr = 0x47,
    /// Write to display memory.
    Mwrite = 0x42,
    /// Read from display memory.
    Mread = 0x43,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub font_width: u8,
    pub font_height: u8,
    pub screen_width: u16,
    pub screen_height: u16,
}


impl Config {
    pub fn new(font_width: u8, font_height: u8, screen_width: u16, screen_height: u16) -> Result<Self, &'static str> {
        let chars_per_line = screen_width / font_width as u16;
        let bytes_per_char = ((font_width + 7) / 8) as u16;
        let cr = chars_per_line * bytes_per_char;
        if cr > 239 { return Err("CR exceeds maximum 239"); }
        Ok(Self {
            font_width, font_height,
            screen_width, screen_height,
        })
    }
}


pub struct RA8835A<DATA, A0, WR, RD, CS, RES, DELAY> {
    /// D0 to D7.
    data: DATA,
    /// Select between command and data modes.
    a0: A0,
    /// 8080 = active-LOW write control.
    /// 6800 = read/write control. HIGH=read, LOW=write
    wr: WR,
    /// 8080 = active-LOW read control signal.
    /// 6800 = active-HIGH enable clock signal. Read/write when HIGH.
    rd: RD,
    /// Active-LOW input to enable RA8835 series.
    cs: CS,
    /// Active-LOW input for hardware reset.
    res: RES,
    delay: DELAY,
    config: Config,
}

impl<DATA, A0, WR, RD, CS, RES, DELAY, E> RA8835A<DATA, A0, WR, RD, CS, RES, DELAY>
where
    DATA: ParallelBus<Error = E>,
    A0: OutputPin,
    WR: OutputPin,
    RD: OutputPin,
    CS: OutputPin,
    RES: OutputPin,
    DELAY: DelayNs,
{
    pub fn new(data: DATA, a0: A0, wr: WR, rd: RD, cs: CS, res: RES, delay: DELAY, config: Config) -> Result<Self, E> {
        let mut display = Self {
            data,
            a0,
            wr,
            rd,
            cs,
            res,
            delay,
            config,
        };
        display.cs.set_low();
        display.hardware_reset()?;
        display.initialize()?;
        display.clear_display();
        display.test_draw()?;
        Ok(display)
    }

    fn hardware_reset(&mut self) -> Result<(), E> {
        self.res.set_low();
        self.delay.delay_ms(10);
        self.res.set_high();
        self.delay.delay_ms(3);
        Ok(())
    }

    fn initialize(&mut self) -> Result<(), E> {
        self.write_command(Command::SystemSet);
        let fx = self.config.font_width - 1;
        let fy = self.config.font_height - 1;
        let chars_per_line = self.config.screen_width / self.config.font_width as u16;
        let bytes_per_char = ((self.config.font_width + 7) / 8) as u16;
        let cr = chars_per_line * bytes_per_char;
        let lf = self.config.screen_height - 1;
        let params = [
            0x30,             // P1: Control byte (default configuration)
            (0x01 << 7) + fx, // P2: WF 0 0 0 0 FX
            fy,               // P3: FY
            cr as u8,         // P4: CR
            cr as u8 + 4,     // P5: TCR
            lf as u8,         // P6: LF
            cr as u8,         // P7: APL
            0x00,             // P8: APH
        ];
        for param in params {
            self.write_data(param);
        }
        Ok(())
    }

    pub fn clear_display(&mut self) -> Result<(), E> {
        self.write_command(Command::Csrw);
        self.write_data(0x00);

        self.write_command(Command::CsrDirRight);
        self.write_command(Command::Mwrite);
        let mut i = 32768;
        while i >= 0 {
            self.write_data(0x00);
            i-=1;
        }
        Ok(())
    }

    pub fn write_text(&mut self, text: &str) -> Result<(), E> {
        // TODO: set csrw? We otherwise write at current cursor address.
        self.write_command(Command::Mwrite);
        for &char in text.as_bytes() {
            self.write_data(char);
        }
        Ok(())
    }

    pub fn write_char(&mut self, char: u8) -> Result<(), E> {
        self.write_command(Command::Mwrite);
        self.write_data(char);
        Ok(())
    }

    // NOTE: Just for testing purposes. Delete this.
    // https://threefivedisplays.com/wp-content/uploads/datasheets/lcd_driver_datasheets/RA8835_REV_3_0_DS.pdf
    // Page 60
    fn test_draw(&mut self) -> Result<(), E> {
        self.write_command(Command::Scroll);
        for data in [0x00, 0x00, 0xF0, 0x80, 0x25, 0xF0, 0x00, 0x4B] {
            self.write_data(data);
        }

        self.write_command(Command::HdotScr);
        self.write_data(0x00);

        self.write_command(Command::Ovlay);
        self.write_data(0x01);

        self.write_command(Command::DisplayOff);
        // self.write_data(0x56);
        self.write_data(0b00001101);

        self.write_command(Command::Csrw);
        self.write_data(0x00);
        self.write_data(0x00);

        self.write_command(Command::CsrForm);
        self.write_data(0x04);
        self.write_data(0x86);

        self.write_command(Command::DisplayOn);

        self.write_command(Command::CsrDirRight);

        self.write_command(Command::Mwrite);
        for &data in "hell world".as_bytes() {
            self.write_data(data);
        }
        Ok(())
    }

    fn write_command(&mut self, cmd: Command) -> Result<(), E> {
        self.a0.set_high();
        self.data.write(cmd as u8);
        self.delay.delay_ns(10);
        self.wr.set_low();
        self.delay.delay_ns(160);
        self.wr.set_high();
        Ok(())
    }

    fn write_data(&mut self, data: u8) -> Result<(), E> {
        self.a0.set_low();
        self.data.write(data);
        self.delay.delay_ns(10);
        self.wr.set_low();
        self.delay.delay_ns(160);
        self.wr.set_high();
        Ok(())
    }

    fn read_data(&mut self) -> Result<u8, E> {
        todo!();
    }

    fn set_cursor_address(&mut self, address: u16) -> Result<(), E> {
        todo!();
        // self.write_command(Command::Csrw)?;
        // self.write_data((address & 0xFF) as u8)?;
        // self.write_data((address >> 8) as u8)
    }
}

pub trait ParallelBus {
    type Error;

    fn write(&mut self, value: u8) -> ();
    fn read(&mut self) -> Result<u8, Self::Error>;
    fn set_input(&mut self) -> Result<(), Self::Error>;
    fn set_output(&mut self) -> Result<(), Self::Error>;
}
