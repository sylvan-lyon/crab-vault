use std::fmt::Display;

use crate::bitmap::Bitmap;

pub const RESET: &str = "\x1B[0m";
pub const ESCAPE_BEGIN: &str = "\x1B[";
pub const ESCAPE_OVER: &str = "m";

pub const BOLD: usize = 1;
pub const DIMMED: usize = 2;
pub const ITALIC: usize = 3;
pub const UNDERLINE: usize = 4;
pub const BLINK_SLOWLY: usize = 5;
pub const BLINK_RAPIDLY: usize = 6;
pub const REVERSE: usize = 7;
pub const HIDDEN: usize = 8;
pub const STRIKE_THROUGH: usize = 9;

#[derive(Clone, Copy, Default)]
pub struct FontStyle {
    pub options: Bitmap<u16>,
}

#[derive(Clone, Copy)]
pub enum AnsiColor {
    Black = 30,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack = 90,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

#[derive(Clone, Copy, Default)]
pub struct AnsiStyle {
    fore: Option<AnsiColor>,
    back: Option<AnsiColor>,
    font: FontStyle,
}

#[derive(Clone, Copy)]
pub struct AnsiString<'a> {
    style: AnsiStyle,
    is_vanilla: bool,
    content: &'a str,
}

impl Display for AnsiStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_vanilla() {
            Ok(())
        } else {
            f.write_str(ESCAPE_BEGIN)?;

            for code in self.font.options.iter_ones() {
                f.write_fmt(format_args!(";{code}"))?;
            }

            if self.fore.is_some() {
                f.write_fmt(format_args!(";{}", self.fore.unwrap().into_fore()))?;
            }

            if self.back.is_some() {
                f.write_fmt(format_args!(";{}", self.back.unwrap().into_back()))?;
            }

            f.write_str(ESCAPE_OVER)
        }
    }
}

impl<'a> Display for AnsiString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 与上面的 AnsiStyle 的 is_vanilla 方法不同
        // AnsiString 的 is_vanilla 完全控制了是否输出转义序列
        if self.is_vanilla {
            f.pad(self.content)
        } else {
            f.write_fmt(format_args!("{}", self.style))?;
            f.pad(self.content)?;
            f.write_str(RESET)
        }
    }
}

impl FontStyle {
    #[inline]
    pub fn new() -> Self {
        Self {
            options: Bitmap::new(),
        }
    }
}

impl AnsiColor {
    #[inline(always)]
    pub fn into_fore(self) -> u8 {
        self as u8
    }

    #[inline(always)]
    pub fn into_back(self) -> u8 {
        self as u8 + 10
    }
}

impl FontStyle {
    #[inline]
    pub fn bold(mut self, enabled: bool) -> Self {
        self.options.set(BOLD, enabled);
        self
    }

    #[inline]
    pub fn dimmed(mut self, enabled: bool) -> Self {
        self.options.set(DIMMED, enabled);
        self
    }

    #[inline]
    pub fn italic(mut self, enabled: bool) -> Self {
        self.options.set(ITALIC, enabled);
        self
    }

    #[inline]
    pub fn underline(mut self, enabled: bool) -> Self {
        self.options.set(UNDERLINE, enabled);
        self
    }

    #[inline]
    pub fn blink_slowly(mut self, enabled: bool) -> Self {
        self.options.set(BLINK_SLOWLY, enabled);
        self
    }

    #[inline]
    pub fn blink_rapidly(mut self, enabled: bool) -> Self {
        self.options.set(BLINK_RAPIDLY, enabled);
        self
    }

    #[inline]
    pub fn reverse(mut self, enabled: bool) -> Self {
        self.options.set(REVERSE, enabled);
        self
    }

    #[inline]
    pub fn hidden(mut self, enabled: bool) -> Self {
        self.options.set(HIDDEN, enabled);
        self
    }

    #[inline]
    pub fn strike_through(mut self, enabled: bool) -> Self {
        self.options.set(STRIKE_THROUGH, enabled);
        self
    }
}

impl AnsiStyle {
    #[inline]
    pub fn new() -> Self {
        Self {
            fore: None,
            back: None,
            font: FontStyle::new(),
        }
    }

    #[inline(always)]
    pub fn new_vanilla() -> Self {
        Self::new()
    }

    #[inline]
    pub fn with_font(mut self, other: FontStyle) -> Self {
        self.font.options |= other.options;
        self
    }

    #[inline]
    pub const fn with_fore(mut self, fore: AnsiColor) -> Self {
        self.fore = Some(fore);
        self
    }

    #[inline]
    pub const fn with_back(mut self, back: AnsiColor) -> Self {
        self.back = Some(back);
        self
    }

    #[inline]
    pub fn with_font_option(mut self, font: Option<FontStyle>) -> Self {
        match font {
            Some(other) => self.font.options |= other.options,
            None => {}
        }
        self
    }

    #[inline]
    pub const fn with_fore_option(mut self, color: Option<AnsiColor>) -> Self {
        self.fore = color;
        self
    }

    #[inline]
    pub const fn with_back_option(mut self, color: Option<AnsiColor>) -> Self {
        self.back = color;
        self
    }

    #[inline]
    pub const fn is_vanilla(self) -> bool {
        self.fore.is_none() && self.back.is_none()
    }

    #[inline]
    pub const fn decorate<'a>(self, content: &'a str) -> AnsiString<'a> {
        AnsiString {
            style: self,
            is_vanilla: self.is_vanilla(),
            content,
        }
    }
}

impl<'a> AnsiString<'a> {
    /// 这将创建一个 [`AnsiString`]
    ///
    /// 但是通过这种方式创建的 [`AnsiString`] **始终不会**带有任何的装饰，因为 AnsiString 这个结构**不提供任何装饰内容的 API**
    ///
    /// 你应该当使用 [`AnsiStyle::decorate`] 方法产生一个 [`AnsiString`]，这样产生的 [`AnsiString`] 就有了颜色信息、样式信息等
    pub fn new_vanilla(content: &'a str) -> Self {
        Self {
            style: AnsiStyle::new(),
            is_vanilla: true,
            content,
        }
    }

    pub fn reset(mut self) -> Self {
        self.is_vanilla = true;
        self
    }

    pub fn get_content(self) -> &'a str {
        self.content
    }
}
