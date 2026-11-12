//! ISO 4217 currencies and their minor-unit exponent.

use crate::error::MoneyError;

/// A currency, with the number of decimal places its minor unit uses.
///
/// The exponent matters: PKR has 2 decimal places, JPY has 0 and KWD has 3.
/// Storing amounts as integers in minor units means the exponent is the only
/// place that difference lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Currency {
    code: [u8; 3],
    exponent: u8,
}

impl Currency {
    /// Pakistani rupee, two decimal places.
    pub const PKR: Self = Self::from_static(*b"PKR", 2);
    /// United States dollar, two decimal places.
    pub const USD: Self = Self::from_static(*b"USD", 2);
    /// Euro, two decimal places.
    pub const EUR: Self = Self::from_static(*b"EUR", 2);
    /// Pound sterling, two decimal places.
    pub const GBP: Self = Self::from_static(*b"GBP", 2);
    /// United Arab Emirates dirham, two decimal places.
    pub const AED: Self = Self::from_static(*b"AED", 2);
    /// Saudi riyal, two decimal places.
    pub const SAR: Self = Self::from_static(*b"SAR", 2);
    /// Japanese yen, no decimal places.
    pub const JPY: Self = Self::from_static(*b"JPY", 0);
    /// Kuwaiti dinar, three decimal places.
    pub const KWD: Self = Self::from_static(*b"KWD", 3);

    const fn from_static(code: [u8; 3], exponent: u8) -> Self {
        Self { code, exponent }
    }

    /// Builds a currency from a three-letter uppercase code and an exponent.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::InvalidCurrency`] when the code is not three
    /// uppercase ASCII letters, and [`MoneyError::InvalidExponent`] when the
    /// exponent is greater than four.
    pub fn new(code: &str, exponent: u8) -> Result<Self, MoneyError> {
        let bytes = code.as_bytes();
        if bytes.len() != 3 || !bytes.iter().all(u8::is_ascii_uppercase) {
            return Err(MoneyError::InvalidCurrency {
                code: code.to_owned(),
            });
        }
        if exponent > 4 {
            return Err(MoneyError::InvalidExponent { exponent });
        }
        Ok(Self {
            code: [bytes[0], bytes[1], bytes[2]],
            exponent,
        })
    }

    /// Looks up a currency from the small table of codes the app knows about.
    #[must_use]
    pub fn from_iso_code(code: &str) -> Option<Self> {
        match code {
            "PKR" => Some(Self::PKR),
            "USD" => Some(Self::USD),
            "EUR" => Some(Self::EUR),
            "GBP" => Some(Self::GBP),
            "AED" => Some(Self::AED),
            "SAR" => Some(Self::SAR),
            "JPY" => Some(Self::JPY),
            "KWD" => Some(Self::KWD),
            _ => None,
        }
    }

    /// The three-letter ISO code.
    #[must_use]
    pub fn code(&self) -> &str {
        std::str::from_utf8(&self.code).unwrap_or("???")
    }

    /// Decimal places in the minor unit.
    #[must_use]
    pub const fn exponent(self) -> u8 {
        self.exponent
    }

    /// How many minor units make one major unit: ten to the power of the exponent.
    #[must_use]
    pub const fn minor_units_per_major(self) -> i64 {
        const SCALES: [i64; 5] = [1, 10, 100, 1_000, 10_000];
        let index = self.exponent as usize;
        if index < SCALES.len() {
            SCALES[index]
        } else {
            1
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
