use bootloader::boot_info::{FrameBufferInfo, PixelFormat};
use conquer_once::spin::OnceCell;
use core::{
    fmt::{self, Write, Arguments},
    ptr,
};
use noto_sans_mono_bitmap::{get_bitmap, get_bitmap_width, BitmapChar, BitmapHeight, FontWeight};
use spinning_top::Spinlock;

/// The global logger instance used for the `log` crate.
pub static WRITER: OnceCell<LockedWriter> = OnceCell::uninit();

/// A [`Writer`] instance protected by a spinlock.
pub struct LockedWriter(Spinlock<Writer>);

/// Additional vertical space between lines
const LINE_SPACING: usize = 0;
/// Additional vertical space between separate log messages
const LOG_SPACING: usize = 2;

impl LockedWriter {
    /// Create a new instance that logs to the given framebuffer.
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        LockedWriter(Spinlock::new(Writer::new(framebuffer, info)))
    }

    pub fn write_fmt(&self, args: Arguments) -> fmt::Result {
        self.0.lock().write_fmt(args)
    }

    /// Force-unlocks the logger to prevent a deadlock.
    ///
    /// This method is not memory safe and should be only used when absolutely necessary.
    pub unsafe fn force_unlock(&self) {
        unsafe { self.0.force_unlock() };
    }
}

/// Allows logging text to a pixel-based framebuffer.
pub struct Writer {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x_pos: usize,
    y_pos: usize,
}

impl Writer {
    /// Creates a new logger that uses the given framebuffer.
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut writer = Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
        };
        writer.clear();
        writer
    }

    fn newline(&mut self) {
        self.y_pos += 14 + LINE_SPACING;
        self.carriage_return()
    }

    fn add_vspace(&mut self, space: usize) {
        self.y_pos += space;
    }

    fn carriage_return(&mut self) {
        self.x_pos = 0;
    }

    /// Erases all text on the screen.
    pub fn clear(&mut self) {
        self.x_pos = 0;
        self.y_pos = 0;
        self.framebuffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.horizontal_resolution
    }

    fn height(&self) -> usize {
        self.info.vertical_resolution
    }

    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                if self.x_pos >= self.width() {
                    self.newline();
                }
                const BITMAP_LETTER_WIDTH: usize =
                    get_bitmap_width(FontWeight::Regular, BitmapHeight::Size14);
                if self.y_pos >= (self.height() - BITMAP_LETTER_WIDTH) {
                    self.clear();
                }
                let bitmap_char = get_bitmap(c, FontWeight::Regular, BitmapHeight::Size14).unwrap();
                self.write_rendered_char(bitmap_char);
            }
        }
    }

    fn write_rendered_char(&mut self, rendered_char: BitmapChar) {
        for (y, row) in rendered_char.bitmap().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += rendered_char.width();
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.info.stride + x;
        let color = match self.info.pixel_format {
            PixelFormat::RGB => [intensity, intensity, intensity / 2, 0],
            PixelFormat::BGR => [intensity / 2, intensity, intensity, 0],
            PixelFormat::U8 => [if intensity > 200 { 0xf } else { 0 }, 0, 0, 0],
            _ => panic!("unsupported pixel format"),
        };
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }
}

unsafe impl Send for Writer {}
unsafe impl Sync for Writer {}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}

///printf macro
#[macro_export]
macro_rules! printf {
    ($($arg:tt)*) => ($crate::terminal::WRITER.get().unwrap().write_fmt(format_args!($($arg)*)).unwrap());
}
