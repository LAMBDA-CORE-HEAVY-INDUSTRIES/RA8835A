#![no_std]
#![deny(unsafe_code)]

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

#[repr(u8)]
pub enum Command {
    /// Initialize device and display.
    SystemSet = 0x40,
    /// Enter standby mode.
    SleepIn = 0x53,
    /// Display off.
    DisplayOff = 0x58,
    /// Display on.
    DisplayOn = 0x59,
    /// Set display start address and display memory regions.
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
    pub text_layer_start: u16,
    pub graphics_layer_start: u16,
}


impl Config {
    pub fn new(font_width: u8, font_height: u8, screen_width: u16, screen_height: u16) -> Result<Self, &'static str> {
        let chars_per_line = screen_width / font_width as u16;
        let bytes_per_char = ((font_width + 7) / 8) as u16;
        let cr = chars_per_line * bytes_per_char;
        if cr > 239 { return Err("CR exceeds maximum 239"); }

        let lines = screen_height / font_height as u16;
        let text_layer_size = chars_per_line * lines;
        let text_layer_start = 0x0000;
        let graphics_layer_start = text_layer_start + text_layer_size;
        Ok(Self {
            font_width, font_height,
            screen_width, screen_height,
            text_layer_start, graphics_layer_start,
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
    pub config: Config,
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
        display.configure_layers()?;
        display.clear_display()?;
        display.enable_display()?;
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
        self.write_command(Command::SystemSet)?;
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
            self.write_data(param)?;
        }
        Ok(())
    }

    /// Configure layer 1 (text), and layer 2 (graphics).
    fn configure_layers(&mut self) -> Result<(), E> {
        self.write_command(Command::Scroll)?;
        let params = [
            (self.config.text_layer_start & 0xFF) as u8,
            (self.config.text_layer_start >> 8) as u8,
            (self.config.screen_height) as u8,
            (self.config.graphics_layer_start & 0xFF) as u8,
            (self.config.graphics_layer_start >> 8) as u8,
            (self.config.screen_height) as u8,
            // 0x00, 0x00,
        ];
        for param in params {
            self.write_data(param)?;
        }
        Ok(())
    }

    pub fn clear_display(&mut self) -> Result<(), E> {
        let total_bytes = (self.config.screen_width / 8) * self.config.screen_height;
        self.set_cursor_address(0x00)?;
        self.write_command(Command::CsrDirRight)?;
        self.write_command(Command::Mwrite)?;
        for _ in 0..total_bytes*3 {
            self.write_data(0x00)?;
        }
        Ok(())
    }

    pub fn write_text(&mut self, text: &str) -> Result<(), E> {
        self.write_command(Command::Mwrite)?;
        for &char in text.as_bytes() {
            self.write_data(char)?;
        }
        Ok(())
    }

    pub fn write_text_at(&mut self, text: &str, x: u16, y: u16) -> Result<(), E> {
        let chars_per_line = self.config.screen_width / self.config.font_width as u16;
        let char_x = x / self.config.font_width as u16;
        let char_y = y / self.config.font_height as u16;
        let byte_addr = self.config.text_layer_start + (char_y * chars_per_line) + char_x;
        self.set_cursor_address(byte_addr)?;
        self.write_command(Command::Mwrite)?;
        for &char in text.as_bytes() {
            self.write_data(char)?;
        }
        Ok(())
    }

    /// Turn the display off and on to enable layers defined in `configure_layers()`.
    fn enable_display(&mut self) -> Result<(), E> {
        self.write_command(Command::HdotScr)?;
        self.write_data(0x00)?;

        self.write_command(Command::DisplayOff)?;
        // First and second layer enabled. Flash cursor at ~16hz.
        self.write_data(0b00111111)?;

        self.write_command(Command::Csrw)?;
        self.write_data(0x00)?;
        self.write_data(0x00)?;
        self.write_command(Command::CsrForm)?;
        self.write_data(0x05)?;
        self.write_data((1 << 7) + self.config.font_height)?; // Block cursor.
        self.write_command(Command::Ovlay)?;
        self.write_data(0x00)?;
        self.write_command(Command::DisplayOn)?;
        Ok(())
    }

    /// Draw pixel at xy. `color` determines if pixel will be drawn or erased.
    pub fn set_pixel(&mut self, x: u16, y: u16, color: bool) -> Result<(), E> {
        let bit_mask = 1 << 0x07 - (x % self.config.font_width as u16);
        let bytes_per_line = self.config.screen_width / self.config.font_width as u16;
        let byte_addr = self.config.graphics_layer_start + (y * bytes_per_line) + (x / self.config.font_width as u16);
        self.set_cursor_address(byte_addr)?;
        self.write_command(Command::Mread)?;
        let current = self.read_data().unwrap_or(0);
        let new_data = match color {
            true => current | bit_mask,
            false => current & !bit_mask
        };
        self.set_cursor_address(byte_addr)?;
        self.write_command(Command::Mwrite)?;
        self.write_data(new_data)?;
        Ok(())
    }

    pub fn write_command(&mut self, cmd: Command) -> Result<(), E> {
        self.a0.set_high();
        self.data.write(cmd as u8);
        self.delay.delay_ns(10);
        self.wr.set_low();
        self.delay.delay_ns(150);
        self.wr.set_high();
        Ok(())
    }

    pub fn write_data(&mut self, data: u8) -> Result<(), E> {
        self.a0.set_low();
        self.data.write(data);
        self.delay.delay_ns(10);
        self.wr.set_low();
        self.delay.delay_ns(150);
        self.wr.set_high();
        Ok(())
    }

    pub fn read_data(&mut self) -> Result<u8, E> {
        self.data.set_input();
        self.a0.set_high();
        self.rd.set_low();
        self.delay.delay_ns(30);
        let result = self.data.read()?;
        self.delay.delay_ns(30);
        self.rd.set_high();
        self.data.set_output();
        Ok(result)
    }

    pub fn set_cursor_address(&mut self, address: u16) -> Result<(), E> {
        self.write_command(Command::Csrw)?;
        self.write_data((address & 0xFF) as u8)?;
        self.write_data((address >> 8) as u8)?;
        Ok(())
    }
}

pub trait ParallelBus {
    type Error;

    fn write(&mut self, value: u8) -> ();
    fn read(&mut self) -> Result<u8, Self::Error>;
    fn set_input(&mut self) -> ();
    fn set_output(&mut self) -> ();
}
