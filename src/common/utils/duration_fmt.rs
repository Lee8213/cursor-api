//! # Duration Formatter
//!
//! A high-performance library for formatting time durations in various human-readable formats.
//!
//! This module provides flexible and localized duration formatting with multiple output styles
//! and language options, optimized for minimal allocations.
//!
//! ## Features
//!
//! - Multiple formatting styles (compact, standard, detailed, ISO8601, etc.)
//! - Multi-language support (English, Chinese, Japanese, Spanish, German)
//! - Automatic format selection based on duration size
//! - High performance with minimal allocations
//!
//! ## Examples
//!
//! ```
//! use std::time::Duration;
//! use duration_fmt::{human, DurationFormat, Language};
//!
//! // Basic usage
//! let duration = Duration::from_secs(3662); // 1h 1m 2s
//! println!("{}", human(duration)); // Uses default format and language
//!
//! // With custom format and language
//! println!("{}", human(duration)
//!     .format(DurationFormat::Detailed)
//!     .language(Language::English));
//! ```

use rand::Rng as _;
use std::{fmt, time::Duration};

use super::string_builder::StringBuilder;

/// Defines the display format for duration formatting.
///
/// Each format option represents a different style of presenting time durations,
/// from compact representations to detailed human-readable formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DurationFormat {
    /// Automatically selects the most appropriate format based on duration size.
    ///
    /// - For durations with days: uses `Detailed` format
    /// - For durations with hours or minutes: uses `Compact` format
    /// - For durations with seconds: displays seconds with millisecond precision
    /// - For smaller durations: uses appropriate millisecond or microsecond units
    Auto,

    /// Compact format without spaces: `1h2m3s`
    ///
    /// Useful for space-constrained displays or technical outputs.
    Compact,

    /// Standard format with spaces: `1 hour 2 minutes 3 seconds`
    ///
    /// A balanced format for general purpose human-readable output.
    Standard,

    /// Detailed format with commas: `1 hour, 2 minutes, 3 seconds`
    ///
    /// Provides the most formal and complete representation.
    Detailed,

    /// ISO 8601 duration format: `PT1H2M3S`
    ///
    /// Follows the international standard for representing time durations.
    ISO8601,

    /// Fuzzy, human-friendly format: `about 5 minutes`
    ///
    /// Rounds to the most significant unit for casual time indications.
    Fuzzy,

    /// Clock-like numeric format: `01:02:03.456`
    ///
    /// Displays time in a familiar digital clock format.
    Numeric,

    /// Verbose format for debugging: `D:1 H:2 M:3 S:4 MS:567`
    ///
    /// Shows all time components with labels, useful for debugging.
    Verbose,

    /// Randomly selects a format for each display.
    ///
    /// Adds an element of surprise to your time displays!
    Random,
}

/// Language setting for duration formatting.
///
/// Controls the language used for unit names and other localized elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    /// English language: "hour", "minute", "second"
    English,

    /// Chinese language: "小时", "分钟", "秒"
    Chinese,

    /// Japanese language: "時間", "分", "秒"
    Japanese,

    /// Spanish language: "hora", "minuto", "segundo"
    Spanish,

    /// German language: "Stunde", "Minute", "Sekunde"
    German,

    /// Randomly selects a language for each display
    Random,
}

// 使用宏定义常量
crate::define_typed_constants! {
    u64 => {
        SECONDS_PER_MINUTE = 60,
        SECONDS_PER_HOUR = 3600,
        SECONDS_PER_DAY = 86400,
        // MILLIS_PER_SECOND = 1000,
    }
    u32 => {
        NANOS_PER_MILLI = 1_000_000,
        NANOS_PER_MICRO = 1_000,
    }
}

/// Time unit used in duration formatting.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TimeUnit {
    /// Days (86400 seconds)
    Day,

    /// Hours (3600 seconds)
    Hour,

    /// Minutes (60 seconds)
    Minute,

    /// Seconds
    Second,

    /// Milliseconds (1/1000 of a second)
    Millisecond,

    /// Microseconds (1/1000000 of a second)
    Microsecond,
}

/// Localization information for a time unit.
///
/// Contains singular and plural forms, abbreviations, and
/// localized fuzzy prefixes for each time unit.
struct UnitLocale {
    /// Singular form of the unit name (e.g., "hour")
    singular: &'static str,

    /// Plural form of the unit name (e.g., "hours")
    plural: &'static str,

    /// Abbreviated form of the unit (e.g., "h")
    short: &'static str,

    /// Prefix used in fuzzy format (e.g., "about ")
    fuzzy_prefix: &'static str,
}

impl UnitLocale {
    /// Creates a new UnitLocale with different singular and plural forms.
    #[inline(always)]
    const fn new(
        singular: &'static str,
        plural: &'static str,
        short: &'static str,
        fuzzy_prefix: &'static str,
    ) -> Self {
        Self {
            singular,
            plural,
            short,
            fuzzy_prefix,
        }
    }

    /// Creates a UnitLocale with the same form for singular and plural.
    ///
    /// Useful for languages like Chinese where the unit doesn't change for plural.
    #[inline(always)]
    const fn same(word: &'static str, short: &'static str, fuzzy_prefix: &'static str) -> Self {
        Self {
            singular: word,
            plural: word,
            short,
            fuzzy_prefix,
        }
    }
}

// 定义本地化字符串
crate::define_typed_constants! {
    &'static str => {
        // Fuzzy prefixes
        FUZZY_EN = "about ",
        FUZZY_ZH = "大约",
        FUZZY_JA = "約",
        FUZZY_ES = "alrededor de ",
        FUZZY_DE = "etwa ",
        FUZZY_EMPTY = "",

        // Time unit abbreviations
        ABBR_DAY = "d",
        ABBR_HOUR_EN = "h",
        ABBR_HOUR_ZH = "时",
        ABBR_HOUR_JA = "時",
        ABBR_HOUR_DE = "Std",
        ABBR_MINUTE_EN = "m",
        ABBR_MINUTE_ZH = "分",
        ABBR_MINUTE_JA = "分",
        ABBR_MINUTE_DE = "Min",
        ABBR_SECOND = "s",
        ABBR_MILLISECOND = "ms",
        ABBR_MICROSECOND_EN = "μs",
        ABBR_MICROSECOND_JA = "μ秒",

        // Chinese units
        UNIT_DAY_ZH = "天",
        UNIT_HOUR_ZH = "小时",
        UNIT_MINUTE_ZH = "分钟",
        UNIT_SECOND_ZH = "秒",
        UNIT_MILLISECOND_ZH = "毫秒",
        UNIT_MICROSECOND_ZH = "微秒",

        // Japanese units
        UNIT_DAY_JA = "日",
        UNIT_HOUR_JA = "時間",
        UNIT_MINUTE_JA = "分",
        UNIT_SECOND_JA = "秒",
        UNIT_MILLISECOND_JA = "ミリ秒",
        UNIT_MICROSECOND_JA = "マイクロ秒",
    }

    UnitLocale => {
        // English
        EN_DAY = UnitLocale::new("day", "days", ABBR_DAY, FUZZY_EN),
        EN_HOUR = UnitLocale::new("hour", "hours", ABBR_HOUR_EN, FUZZY_EN),
        EN_MINUTE = UnitLocale::new("minute", "minutes", ABBR_MINUTE_EN, FUZZY_EN),
        EN_SECOND = UnitLocale::new("second", "seconds", ABBR_SECOND, FUZZY_EN),
        EN_MILLISECOND = UnitLocale::new("millisecond", "milliseconds", ABBR_MILLISECOND, FUZZY_EMPTY),
        EN_MICROSECOND = UnitLocale::new("microsecond", "microseconds", ABBR_MICROSECOND_EN, FUZZY_EMPTY),

        // Chinese
        ZH_DAY = UnitLocale::same(UNIT_DAY_ZH, UNIT_DAY_ZH, FUZZY_ZH),
        ZH_HOUR = UnitLocale::same(UNIT_HOUR_ZH, ABBR_HOUR_ZH, FUZZY_ZH),
        ZH_MINUTE = UnitLocale::same(UNIT_MINUTE_ZH, ABBR_MINUTE_ZH, FUZZY_ZH),
        ZH_SECOND = UnitLocale::same(UNIT_SECOND_ZH, UNIT_SECOND_ZH, FUZZY_ZH),
        ZH_MILLISECOND = UnitLocale::same(UNIT_MILLISECOND_ZH, UNIT_MILLISECOND_ZH, FUZZY_EMPTY),
        ZH_MICROSECOND = UnitLocale::same(UNIT_MICROSECOND_ZH, UNIT_MICROSECOND_ZH, FUZZY_EMPTY),

        // Japanese
        JA_DAY = UnitLocale::same(UNIT_DAY_JA, UNIT_DAY_JA, FUZZY_JA),
        JA_HOUR = UnitLocale::same(UNIT_HOUR_JA, ABBR_HOUR_JA, FUZZY_JA),
        JA_MINUTE = UnitLocale::same(UNIT_MINUTE_JA, ABBR_MINUTE_JA, FUZZY_JA),
        JA_SECOND = UnitLocale::same(UNIT_SECOND_JA, UNIT_SECOND_JA, FUZZY_JA),
        JA_MILLISECOND = UnitLocale::same(UNIT_MILLISECOND_JA, UNIT_MILLISECOND_JA, FUZZY_EMPTY),
        JA_MICROSECOND = UnitLocale::same(UNIT_MICROSECOND_JA, ABBR_MICROSECOND_JA, FUZZY_EMPTY),

        // Spanish
        ES_DAY = UnitLocale::new("día", "días", ABBR_DAY, FUZZY_ES),
        ES_HOUR = UnitLocale::new("hora", "horas", ABBR_HOUR_EN, FUZZY_ES),
        ES_MINUTE = UnitLocale::new("minuto", "minutos", ABBR_MINUTE_EN, FUZZY_ES),
        ES_SECOND = UnitLocale::new("segundo", "segundos", ABBR_SECOND, FUZZY_ES),
        ES_MILLISECOND = UnitLocale::new("milisegundo", "milisegundos", ABBR_MILLISECOND, FUZZY_EMPTY),
        ES_MICROSECOND = UnitLocale::new("microsegundo", "microsegundos", ABBR_MICROSECOND_EN, FUZZY_EMPTY),

        // German (Deutsch)
        DE_DAY = UnitLocale::new("Tag", "Tage", "T", FUZZY_DE),
        DE_HOUR = UnitLocale::new("Stunde", "Stunden", ABBR_HOUR_DE, FUZZY_DE),
        DE_MINUTE = UnitLocale::new("Minute", "Minuten", ABBR_MINUTE_DE, FUZZY_DE),
        DE_SECOND = UnitLocale::new("Sekunde", "Sekunden", ABBR_SECOND, FUZZY_DE),
        DE_MILLISECOND = UnitLocale::new("Millisekunde", "Millisekunden", ABBR_MILLISECOND, FUZZY_EMPTY),
        DE_MICROSECOND = UnitLocale::new("Mikrosekunde", "Mikrosekunden", ABBR_MICROSECOND_EN, FUZZY_EMPTY),
    }
}

/// A wrapper for Duration that provides human-readable formatting options.
///
/// `HumanDuration` allows a standard `Duration` to be formatted in various styles
/// and languages. It uses a builder pattern for configuring format and language options.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use duration_fmt::{human, DurationFormat, Language};
///
/// let duration = Duration::from_secs(3662); // 1h 1m 2s
///
/// // Basic usage with default settings
/// println!("{}", human(duration));
///
/// // With custom format and language
/// println!("{}", human(duration)
///     .format(DurationFormat::Detailed)
///     .language(Language::English));
/// ```
pub struct HumanDuration {
    /// The wrapped Duration to be formatted
    duration: Duration,

    /// The selected format style
    format: DurationFormat,

    /// The selected language
    language: Language,
}

impl HumanDuration {
    /// Creates a new `HumanDuration` with default formatting options.
    ///
    /// By default, it uses `DurationFormat::Auto` and `Language::Chinese`.
    ///
    /// # Arguments
    ///
    /// * `duration` - The `Duration` to be formatted
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use duration_fmt::HumanDuration;
    ///
    /// let duration = Duration::from_secs(65);
    /// let human_duration = HumanDuration::new(duration);
    /// ```
    #[inline]
    pub const fn new(duration: Duration) -> Self {
        Self {
            duration,
            format: DurationFormat::Auto,
            language: Language::Chinese,
        }
    }

    /// Sets the display format for this duration.
    ///
    /// # Arguments
    ///
    /// * `format` - The `DurationFormat` to use when formatting
    ///
    /// # Returns
    ///
    /// The modified `HumanDuration` for method chaining
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use duration_fmt::{human, DurationFormat};
    ///
    /// let formatted = human(Duration::from_secs(65))
    ///     .format(DurationFormat::Compact);
    /// assert_eq!(formatted.to_string(), "1m5s"); // In compact format
    /// ```
    #[inline]
    pub const fn format(mut self, format: DurationFormat) -> Self {
        self.format = format;
        self
    }

    /// Sets the language for this duration's output.
    ///
    /// # Arguments
    ///
    /// * `language` - The `Language` to use for formatting
    ///
    /// # Returns
    ///
    /// The modified `HumanDuration` for method chaining
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use duration_fmt::{human, Language, DurationFormat};
    ///
    /// let formatted = human(Duration::from_secs(60))
    ///     .format(DurationFormat::Standard)
    ///     .language(Language::English);
    /// assert_eq!(formatted.to_string(), "1 minute");
    /// ```
    #[inline]
    pub const fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    /// Extracts the component parts from the duration for formatting.
    ///
    /// Breaks the duration into days, hours, minutes, seconds, milliseconds, etc.
    #[inline]
    const fn get_parts(&self) -> TimeParts {
        let total_secs = self.duration.as_secs();
        let days = total_secs / SECONDS_PER_DAY;
        let hours = (total_secs % SECONDS_PER_DAY) / SECONDS_PER_HOUR;
        let minutes = (total_secs % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
        let seconds = total_secs % SECONDS_PER_MINUTE;
        let nanos = self.duration.subsec_nanos();
        let millis = nanos / NANOS_PER_MILLI;
        let micros = (nanos % NANOS_PER_MILLI) / NANOS_PER_MICRO;

        TimeParts {
            days,
            hours,
            minutes,
            seconds,
            millis,
            micros,
            nanos,
        }
    }

    /// Randomly selects a language for variety.
    ///
    /// Used when `Language::Random` is specified.
    #[inline(never)] // 随机函数不应该内联，它们通常不在热路径上
    fn random_language() -> Language {
        const LANGUAGES: &[Language] = &[
            Language::English,
            Language::Chinese,
            Language::Japanese,
            Language::Spanish,
            Language::German,
        ];

        unsafe { *LANGUAGES.get_unchecked(rand::rng().random_range(0..LANGUAGES.len())) }
    }

    /// Resolves the actual language to use, handling the Random case.
    #[inline]
    fn get_actual_language(&self) -> Language {
        match self.language {
            Language::Random => Self::random_language(),
            other => other,
        }
    }

    /// Gets the localization information for a time unit.
    ///
    /// Provides the appropriate strings for unit names based on the current language.
    #[inline(always)] // 这是一个简单的查找表，应该总是内联
    fn get_unit_locale(&self, unit: TimeUnit) -> &'static UnitLocale {
        use Language::*;
        use TimeUnit::*;

        let language = self.get_actual_language();

        match (language, unit) {
            // English
            (English, Day) => &EN_DAY,
            (English, Hour) => &EN_HOUR,
            (English, Minute) => &EN_MINUTE,
            (English, Second) => &EN_SECOND,
            (English, Millisecond) => &EN_MILLISECOND,
            (English, Microsecond) => &EN_MICROSECOND,

            // Chinese
            (Chinese, Day) => &ZH_DAY,
            (Chinese, Hour) => &ZH_HOUR,
            (Chinese, Minute) => &ZH_MINUTE,
            (Chinese, Second) => &ZH_SECOND,
            (Chinese, Millisecond) => &ZH_MILLISECOND,
            (Chinese, Microsecond) => &ZH_MICROSECOND,

            // Japanese
            (Japanese, Day) => &JA_DAY,
            (Japanese, Hour) => &JA_HOUR,
            (Japanese, Minute) => &JA_MINUTE,
            (Japanese, Second) => &JA_SECOND,
            (Japanese, Millisecond) => &JA_MILLISECOND,
            (Japanese, Microsecond) => &JA_MICROSECOND,

            // Spanish
            (Spanish, Day) => &ES_DAY,
            (Spanish, Hour) => &ES_HOUR,
            (Spanish, Minute) => &ES_MINUTE,
            (Spanish, Second) => &ES_SECOND,
            (Spanish, Millisecond) => &ES_MILLISECOND,
            (Spanish, Microsecond) => &ES_MICROSECOND,

            // German
            (German, Day) => &DE_DAY,
            (German, Hour) => &DE_HOUR,
            (German, Minute) => &DE_MINUTE,
            (German, Second) => &DE_SECOND,
            (German, Millisecond) => &DE_MILLISECOND,
            (German, Microsecond) => &DE_MICROSECOND,

            // Random should have been resolved by get_actual_language
            (Random, _) => __unreachable!(),
        }
    }

    /// Gets the appropriate unit string based on count and format.
    ///
    /// Returns singular, plural, or abbreviated form as appropriate.
    #[inline(always)] // 简单的条件判断，应该总是内联
    fn get_unit_str(&self, unit: TimeUnit, count: u64, short: bool) -> &'static str {
        let locale = self.get_unit_locale(unit);
        if short {
            locale.short
        } else if count > 1 {
            locale.plural
        } else {
            locale.singular
        }
    }
}

/// Container for the time components extracted from a Duration.
///
/// Holds the days, hours, minutes, seconds, milliseconds, etc. broken out
/// from a Duration for easier formatting.
struct TimeParts {
    /// Number of whole days
    days: u64,

    /// Number of hours (0-23)
    hours: u64,

    /// Number of minutes (0-59)
    minutes: u64,

    /// Number of seconds (0-59)
    seconds: u64,

    /// Number of milliseconds (0-999)
    millis: u32,

    /// Number of microseconds (0-999)
    micros: u32,

    /// Total nanoseconds part (0-999,999,999)
    nanos: u32,
}

/// Display implementation for HumanDuration
///
/// This allows a HumanDuration to be formatted with `format!` or `to_string()`.
impl fmt::Display for HumanDuration {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DurationFormat::*;

        // 如果是随机格式或随机语言，需要创建新的实例
        if self.format == Random || self.language == Language::Random {
            let actual_format = if self.format == Random {
                Self::random_format()
            } else {
                self.format
            };

            let actual_language = if self.language == Language::Random {
                Self::random_language()
            } else {
                self.language
            };

            return HumanDuration {
                duration: self.duration,
                format: actual_format,
                language: actual_language,
            }
            .fmt(f);
        }

        match self.format {
            Auto => self.fmt_auto(f),
            Compact => self.fmt_compact(f),
            Standard => self.fmt_standard(f),
            Detailed => self.fmt_detailed(f),
            ISO8601 => self.fmt_iso8601(f),
            Fuzzy => self.fmt_fuzzy(f),
            Numeric => self.fmt_numeric(f),
            Verbose => self.fmt_verbose(f),
            Random => __unreachable!(),
        }
    }
}

// 格式化实现
impl HumanDuration {
    /// Randomly selects a format for variety.
    ///
    /// Used when `DurationFormat::Random` is specified.
    #[inline(never)] // 随机函数不应该内联
    fn random_format() -> DurationFormat {
        const CANDIDATES: &[DurationFormat] = &[
            DurationFormat::Compact,
            DurationFormat::Standard,
            DurationFormat::Detailed,
            DurationFormat::ISO8601,
            DurationFormat::Fuzzy,
            DurationFormat::Numeric,
            DurationFormat::Verbose,
        ];

        unsafe { *CANDIDATES.get_unchecked(rand::rng().random_range(0..CANDIDATES.len())) }
    }

    /// Formats the duration using the Auto format.
    ///
    /// Auto selects an appropriate format based on the duration's magnitude:
    /// - For durations with days: uses the Detailed format
    /// - For durations with hours/minutes: uses the Compact format
    /// - For seconds: shows seconds with millisecond precision
    /// - For smaller durations: uses appropriate millisecond or microsecond units
    #[inline(never)] // 复杂的格式化函数不应该内联，会增加代码大小
    fn fmt_auto(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.get_parts();

        if parts.days > 0 {
            self.fmt_detailed(f)
        } else if parts.hours > 0 || parts.minutes > 0 {
            self.fmt_compact(f)
        } else if parts.seconds > 0 {
            write!(
                f,
                "{}.{:03}{}",
                parts.seconds,
                parts.millis,
                self.get_unit_str(TimeUnit::Second, parts.seconds, false)
            )
        } else if parts.millis > 0 {
            write!(
                f,
                "{}{}",
                parts.millis,
                self.get_unit_str(TimeUnit::Millisecond, parts.millis as u64, false)
            )
        } else if parts.micros > 0 {
            write!(
                f,
                "{}{}",
                parts.micros,
                self.get_unit_str(TimeUnit::Microsecond, parts.micros as u64, false)
            )
        } else {
            write!(f, "0{}", self.get_unit_str(TimeUnit::Second, 0, false))
        }
    }

    /// Formats the duration in Compact style: `1h2m3s`
    ///
    /// This format shows each non-zero time component with its abbreviated
    /// unit indicator, without spaces between components.
    #[inline(never)] // 复杂的格式化函数
    fn fmt_compact(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.get_parts();

        // 计算最大可能的append调用次数：
        // 每个组件最多有2次append (数字 + 单位)
        // 4个可能的组件 (天, 小时, 分钟, 秒)
        // 总共: 4 * 2 = 8次append
        let mut builder = StringBuilder::with_capacity(8);

        if parts.days > 0 {
            let unit = self.get_unit_str(TimeUnit::Day, parts.days, true);
            builder.append_mut(parts.days.to_string()).append_mut(unit);
        }
        if parts.hours > 0 {
            let unit = self.get_unit_str(TimeUnit::Hour, parts.hours, true);
            builder.append_mut(parts.hours.to_string()).append_mut(unit);
        }
        if parts.minutes > 0 {
            let unit = self.get_unit_str(TimeUnit::Minute, parts.minutes, true);
            builder
                .append_mut(parts.minutes.to_string())
                .append_mut(unit);
        }
        if parts.seconds > 0 || builder.is_empty() {
            let unit = self.get_unit_str(TimeUnit::Second, parts.seconds, true);
            builder
                .append_mut(parts.seconds.to_string())
                .append_mut(unit);
        }

        f.write_str(&builder.build())
    }

    /// Formats the duration in Standard style: `1 hour 2 minutes 3 seconds`
    ///
    /// This format shows each non-zero time component with its full unit name,
    /// separated by spaces.
    #[inline(never)] // 复杂的格式化函数
    fn fmt_standard(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.get_parts();

        // 计算最大可能的append调用次数：
        // 每个组件最多有3次append (数字, 空格, 单位)
        // 4个可能的组件 (天, 小时, 分钟, 秒)
        // 组件之间的空格最多3次
        // 总共: 4*3 + 3 = 15次append
        let mut builder = StringBuilder::with_capacity(15);

        if parts.days > 0 {
            let unit = self.get_unit_str(TimeUnit::Day, parts.days, false);
            builder
                .append_mut(parts.days.to_string())
                .append_mut(" ")
                .append_mut(unit);
        }
        if parts.hours > 0 {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            let unit = self.get_unit_str(TimeUnit::Hour, parts.hours, false);
            builder
                .append_mut(parts.hours.to_string())
                .append_mut(" ")
                .append_mut(unit);
        }
        if parts.minutes > 0 {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            let unit = self.get_unit_str(TimeUnit::Minute, parts.minutes, false);
            builder
                .append_mut(parts.minutes.to_string())
                .append_mut(" ")
                .append_mut(unit);
        }
        if parts.seconds > 0 || builder.is_empty() {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            let unit = self.get_unit_str(TimeUnit::Second, parts.seconds, false);
            builder
                .append_mut(parts.seconds.to_string())
                .append_mut(" ")
                .append_mut(unit);
        }

        f.write_str(&builder.build())
    }

    /// Formats the duration in Detailed style with separators between components
    ///
    /// This format is similar to Standard but adds appropriate separators
    /// between components based on the selected language.
    #[inline(never)] // 复杂的格式化函数
    fn fmt_detailed(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let separator = match self.get_actual_language() {
            Language::Chinese | Language::Japanese => "、",
            _ => ", ",
        };

        let parts = self.get_parts();

        // 计算最大可能的append调用次数：
        // 每个组件最多有3次append (数字, 空格, 单位)
        // 4个可能的组件 (天, 小时, 分钟, 秒+毫秒)
        // 组件之间的分隔符最多3次
        // 总共: 4*3 + 3 = 15次append
        let mut builder = StringBuilder::with_capacity(15);

        if parts.days > 0 {
            let unit = self.get_unit_str(TimeUnit::Day, parts.days, false);
            builder
                .append_mut(parts.days.to_string())
                .append_mut(" ")
                .append_mut(unit);
        }
        if parts.hours > 0 {
            if !builder.is_empty() {
                builder.append_mut(separator);
            }
            let unit = self.get_unit_str(TimeUnit::Hour, parts.hours, false);
            builder
                .append_mut(parts.hours.to_string())
                .append_mut(" ")
                .append_mut(unit);
        }
        if parts.minutes > 0 {
            if !builder.is_empty() {
                builder.append_mut(separator);
            }
            let unit = self.get_unit_str(TimeUnit::Minute, parts.minutes, false);
            builder
                .append_mut(parts.minutes.to_string())
                .append_mut(" ")
                .append_mut(unit);
        }

        // 格式化秒和毫秒
        if !builder.is_empty() {
            builder.append_mut(separator);
        }

        // 直接格式化秒和毫秒部分到StringBuilder
        let seconds_str = format!("{}.{:03}", parts.seconds, parts.millis);
        let unit = self.get_unit_str(TimeUnit::Second, parts.seconds, false);
        builder
            .append_mut(seconds_str)
            .append_mut(" ")
            .append_mut(unit);

        f.write_str(&builder.build())
    }

    /// Formats the duration in ISO 8601 style: `PT1H2M3S`
    ///
    /// Follows the international standard format for durations.
    #[inline] // ISO8601相对简单，可以内联
    fn fmt_iso8601(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.get_parts();

        // 计算最大可能的append调用次数：
        // "P" + 可能的天 (数字+"D") + 可能的"T" +
        // 可能的小时 (数字+"H") + 可能的分钟 (数字+"M") + 可能的秒 (数字+"S")
        // 总共: 1 + 2 + 1 + 2 + 2 + 2 = 10次append
        let mut builder = StringBuilder::with_capacity(10);

        builder.append_mut("P");

        if parts.days > 0 {
            builder.append_mut(parts.days.to_string()).append_mut("D");
        }

        if parts.hours > 0 || parts.minutes > 0 || parts.seconds > 0 {
            builder.append_mut("T");
            if parts.hours > 0 {
                builder.append_mut(parts.hours.to_string()).append_mut("H");
            }
            if parts.minutes > 0 {
                builder
                    .append_mut(parts.minutes.to_string())
                    .append_mut("M");
            }
            if parts.seconds > 0 {
                builder
                    .append_mut(parts.seconds.to_string())
                    .append_mut("S");
            }
        }

        f.write_str(&builder.build())
    }

    /// Formats the duration in a fuzzy, human-friendly style: `about 5 minutes`
    ///
    /// Rounds to the most significant unit for a rough indication of time.
    #[inline(never)] // 复杂的格式化函数
    fn fmt_fuzzy(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_secs = self.duration.as_secs();
        let locale_prefix = self.get_unit_locale(TimeUnit::Second).fuzzy_prefix;

        // 计算最大可能的append调用次数：
        // 前缀 + 数字 + 空格 + 单位
        // 总共: 4次append
        let mut builder = StringBuilder::with_capacity(4);

        if total_secs < 2 {
            let unit = self.get_unit_str(TimeUnit::Second, 1, false);
            builder
                .append_mut(locale_prefix)
                .append_mut("1 ")
                .append_mut(unit);
        } else if total_secs < 60 {
            let unit = self.get_unit_str(TimeUnit::Second, total_secs, false);
            builder
                .append_mut(locale_prefix)
                .append_mut(total_secs.to_string())
                .append_mut(" ")
                .append_mut(unit);
        } else if total_secs < 120 {
            let unit = self.get_unit_str(TimeUnit::Minute, 1, false);
            builder
                .append_mut(locale_prefix)
                .append_mut("1 ")
                .append_mut(unit);
        } else if total_secs < SECONDS_PER_HOUR {
            let minutes = (total_secs + 30) / 60; // 四舍五入到分钟
            let unit = self.get_unit_str(TimeUnit::Minute, minutes, false);
            builder
                .append_mut(locale_prefix)
                .append_mut(minutes.to_string())
                .append_mut(" ")
                .append_mut(unit);
        } else if total_secs < SECONDS_PER_HOUR * 2 {
            let unit = self.get_unit_str(TimeUnit::Hour, 1, false);
            builder
                .append_mut(locale_prefix)
                .append_mut("1 ")
                .append_mut(unit);
        } else if total_secs < SECONDS_PER_DAY {
            let hours = (total_secs + 1800) / 3600; // 四舍五入到小时
            let unit = self.get_unit_str(TimeUnit::Hour, hours, false);
            builder
                .append_mut(locale_prefix)
                .append_mut(hours.to_string())
                .append_mut(" ")
                .append_mut(unit);
        } else {
            let days = (total_secs + 43200) / 86400; // 四舍五入到天
            let unit = self.get_unit_str(TimeUnit::Day, days, false);
            builder
                .append_mut(locale_prefix)
                .append_mut(days.to_string())
                .append_mut(" ")
                .append_mut(unit);
        }

        f.write_str(&builder.build())
    }

    /// Formats the duration in a numeric, clock-like style: `01:02:03.456`
    ///
    /// Shows the time components in a familiar digital clock format.
    #[inline] // 相对简单的格式化函数，可以内联
    fn fmt_numeric(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.get_parts();

        // 这个函数使用write!直接格式化到f更高效，保持原样
        if parts.days > 0 {
            write!(
                f,
                "{}:{:02}:{:02}:{:02}.{:03}",
                parts.days, parts.hours, parts.minutes, parts.seconds, parts.millis
            )
        } else {
            write!(
                f,
                "{:02}:{:02}:{:02}.{:03}",
                parts.hours, parts.minutes, parts.seconds, parts.millis
            )
        }
    }

    /// Formats the duration in verbose style for debugging: `D:1 H:2 M:3 S:4 MS:567`
    ///
    /// Shows all time components with explicit labels, useful for debugging.
    #[inline(never)] // 涉及Vec分配的复杂函数
    fn fmt_verbose(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.get_parts();

        // 计算最大可能的append调用次数：
        // 每个组件最多有 (标签 + 数字 + 可能的空格)
        // 7个可能的组件 (天, 小时, 分钟, 秒, 毫秒, 微秒, 纳秒)
        // 总共: 7 * 3 = 21次append
        let mut builder = StringBuilder::with_capacity(21);

        if parts.days > 0 {
            builder.append_mut("D:").append_mut(parts.days.to_string());
        }
        if parts.hours > 0 {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            builder.append_mut("H:").append_mut(parts.hours.to_string());
        }
        if parts.minutes > 0 {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            builder
                .append_mut("M:")
                .append_mut(parts.minutes.to_string());
        }
        if parts.seconds > 0 {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            builder
                .append_mut("S:")
                .append_mut(parts.seconds.to_string());
        }
        if parts.millis > 0 {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            builder
                .append_mut("ms:")
                .append_mut(parts.millis.to_string());
        }
        if parts.micros > 0 {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            builder
                .append_mut("μs:")
                .append_mut(parts.micros.to_string());
        }
        if parts.nanos > 0 {
            if !builder.is_empty() {
                builder.append_mut(" ");
            }
            builder
                .append_mut("ns:")
                .append_mut(parts.nanos.to_string());
        }

        if builder.is_empty() {
            f.write_str("0s")
        } else {
            f.write_str(&builder.build())
        }
    }
}

/// Creates a new HumanDuration from a Duration.
///
/// This is a convenience function for creating a HumanDuration with default settings.
///
/// # Arguments
///
/// * `duration` - The Duration to format
///
/// # Returns
///
/// A new HumanDuration with default settings
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use duration_fmt::{human, DurationFormat, Language};
///
/// let duration = Duration::from_secs(3662); // 1h 1m 2s
///
/// // Basic usage
/// println!("{}", human(duration));
///
/// // With additional options
/// println!("{}", human(duration)
///     .format(DurationFormat::Compact)
///     .language(Language::English));
/// ```
#[inline(always)] // 简单的包装函数，应该总是内联
pub const fn human(duration: Duration) -> HumanDuration { HumanDuration::new(duration) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_compact_format() {
        let duration = Duration::from_secs(3662); // 1h 1m 2s
        let formatted = human(duration)
            .format(DurationFormat::Compact)
            .language(Language::English);
        assert_eq!(formatted.to_string(), "1h1m2s");
    }

    #[test]
    fn test_standard_format() {
        let duration = Duration::from_secs(3662); // 1h 1m 2s
        let formatted = human(duration)
            .format(DurationFormat::Standard)
            .language(Language::English);
        assert_eq!(formatted.to_string(), "1 hour 1 minute 2 seconds");
    }

    #[test]
    fn test_detailed_format() {
        let duration = Duration::from_secs(3662); // 1h 1m 2s
        let formatted = human(duration)
            .format(DurationFormat::Detailed)
            .language(Language::English);
        assert_eq!(formatted.to_string(), "1 hour, 1 minute, 2.000 seconds");
    }

    #[test]
    fn test_iso8601_format() {
        let duration = Duration::from_secs(3662); // 1h 1m 2s
        let formatted = human(duration).format(DurationFormat::ISO8601);
        assert_eq!(formatted.to_string(), "PT1H1M2S");
    }

    #[test]
    fn test_chinese_language() {
        let duration = Duration::from_secs(65); // 1m 5s
        let formatted = human(duration)
            .format(DurationFormat::Standard)
            .language(Language::Chinese);
        assert_eq!(formatted.to_string(), "1 分钟 5 秒");
    }

    #[test]
    fn test_fuzzy_format() {
        let duration = Duration::from_secs(50); // 50s
        let formatted = human(duration)
            .format(DurationFormat::Fuzzy)
            .language(Language::English);
        assert_eq!(formatted.to_string(), "about 50 seconds");

        let duration = Duration::from_secs(3600); // 1h
        let formatted = human(duration)
            .format(DurationFormat::Fuzzy)
            .language(Language::English);
        assert_eq!(formatted.to_string(), "about 1 hour");
    }
}
