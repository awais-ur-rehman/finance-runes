//! Exact amounts of money.

use crate::error::MoneyError;
use crate::money::currency::Currency;
use crate::money::rounding::Rounding;

/// An exact amount of money: a count of minor units in one currency.
///
/// The amount is never a floating-point number, so repeated addition, splitting
/// and conversion cannot drift. See ADR-0003.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Money {
    minor: i64,
    currency: Currency,
}

impl Money {
    /// Builds an amount from a count of minor units.
    pub const fn from_minor(minor: i64, currency: Currency) -> Self {
        Self { minor, currency }
    }

    /// Zero in the given currency.
    pub const fn zero(currency: Currency) -> Self {
        Self::from_minor(0, currency)
    }

    /// The amount in minor units.
    pub const fn minor(self) -> i64 {
        self.minor
    }

    /// The currency.
    pub const fn currency(self) -> Currency {
        self.currency
    }

    /// True when the amount is exactly zero.
    pub const fn is_zero(self) -> bool {
        self.minor == 0
    }

    /// True when the amount is negative.
    pub const fn is_negative(self) -> bool {
        self.minor < 0
    }

    /// True when the amount is strictly positive.
    pub const fn is_positive(self) -> bool {
        self.minor > 0
    }

    /// Adds two amounts of the same currency.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::CurrencyMismatch`] if the currencies differ and
    /// [`MoneyError::Overflow`] if the result does not fit.
    pub fn checked_add(self, rhs: Self) -> Result<Self, MoneyError> {
        self.ensure_same(rhs.currency)?;
        let minor = self
            .minor
            .checked_add(rhs.minor)
            .ok_or(MoneyError::Overflow)?;
        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    /// Subtracts `rhs` from `self`.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::CurrencyMismatch`] if the currencies differ and
    /// [`MoneyError::Overflow`] if the result does not fit.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, MoneyError> {
        self.ensure_same(rhs.currency)?;
        let minor = self
            .minor
            .checked_sub(rhs.minor)
            .ok_or(MoneyError::Overflow)?;
        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    /// Negates the amount.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::Overflow`] for the smallest representable amount.
    pub fn checked_neg(self) -> Result<Self, MoneyError> {
        let minor = self.minor.checked_neg().ok_or(MoneyError::Overflow)?;
        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    /// The absolute value of the amount.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::Overflow`] for the smallest representable amount.
    pub fn checked_abs(self) -> Result<Self, MoneyError> {
        let minor = self.minor.checked_abs().ok_or(MoneyError::Overflow)?;
        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    /// Multiplies by an integer factor.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::Overflow`] if the result does not fit.
    pub fn checked_mul(self, factor: i64) -> Result<Self, MoneyError> {
        let minor = self.minor.checked_mul(factor).ok_or(MoneyError::Overflow)?;
        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    /// Divides by an integer, rounding with the given policy.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::DivisionByZero`] when `divisor` is zero and
    /// [`MoneyError::Overflow`] if the result does not fit.
    pub fn div(self, divisor: i64, rounding: Rounding) -> Result<Self, MoneyError> {
        if divisor == 0 {
            return Err(MoneyError::DivisionByZero);
        }
        let minor = rounding.apply(i128::from(self.minor), i128::from(divisor));
        let minor = i64::try_from(minor).map_err(|_| MoneyError::Overflow)?;
        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    /// Splits the amount across `weights` so that the parts sum exactly to the
    /// original, using largest remainders.
    ///
    /// This is how a split or a budget allocation stays exact: the parts can
    /// never lose or invent a minor unit.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::EmptyAllocation`] when `weights` is empty or all
    /// zero, and [`MoneyError::Overflow`] if a part does not fit.
    pub fn allocate(self, weights: &[u32]) -> Result<Vec<Self>, MoneyError> {
        if weights.is_empty() {
            return Err(MoneyError::EmptyAllocation);
        }
        let total: u64 = weights.iter().map(|weight| u64::from(*weight)).sum();
        if total == 0 {
            return Err(MoneyError::EmptyAllocation);
        }

        let negative = self.minor < 0;
        let magnitude = i128::from(self.minor).abs();
        let total = i128::from(total);

        let mut parts = Vec::with_capacity(weights.len());
        let mut remainders = Vec::with_capacity(weights.len());
        let mut assigned: i128 = 0;

        for (index, weight) in weights.iter().enumerate() {
            let exact = magnitude * i128::from(*weight);
            let base = exact / total;
            parts.push(base);
            remainders.push((exact % total, index));
            assigned += base;
        }

        let leftover = magnitude - assigned;
        remainders.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
        for (_, index) in remainders.into_iter().take(leftover as usize) {
            parts[index] += 1;
        }

        parts
            .into_iter()
            .map(|part| {
                let signed = if negative { -part } else { part };
                let minor = i64::try_from(signed).map_err(|_| MoneyError::Overflow)?;
                Ok(Self {
                    minor,
                    currency: self.currency,
                })
            })
            .collect()
    }

    /// Parses a decimal string such as `-1234.50`, using the currency's exponent.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::InvalidAmount`] for anything that is not a decimal
    /// number, [`MoneyError::TooManyDecimals`] when the input has more decimal
    /// places than the currency allows, and [`MoneyError::Overflow`] when the
    /// value does not fit.
    pub fn parse(input: &str, currency: Currency) -> Result<Self, MoneyError> {
        let invalid = || MoneyError::InvalidAmount {
            input: input.to_owned(),
        };

        let text = input.trim();
        if text.is_empty() {
            return Err(invalid());
        }

        let (negative, body) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        if body.is_empty() {
            return Err(invalid());
        }

        let (whole, fraction) = match body.split_once('.') {
            Some((whole, fraction)) => (whole, fraction),
            None => (body, ""),
        };
        if fraction.contains('.') {
            return Err(invalid());
        }

        let whole = if whole.is_empty() { "0" } else { whole };
        if !whole.bytes().all(|byte| byte.is_ascii_digit())
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(invalid());
        }

        let exponent = usize::from(currency.exponent());
        if fraction.len() > exponent {
            return Err(MoneyError::TooManyDecimals {
                currency,
                max: currency.exponent(),
                found: fraction.len(),
            });
        }

        let scale = i128::from(currency.minor_units_per_major());
        let whole_value: i128 = whole.parse().map_err(|_| invalid())?;
        let mut minor = whole_value.checked_mul(scale).ok_or(MoneyError::Overflow)?;

        if !fraction.is_empty() {
            let mut fraction_value: i128 = fraction.parse().map_err(|_| invalid())?;
            for _ in fraction.len()..exponent {
                fraction_value *= 10;
            }
            minor = minor
                .checked_add(fraction_value)
                .ok_or(MoneyError::Overflow)?;
        }

        if negative {
            minor = -minor;
        }
        let minor = i64::try_from(minor).map_err(|_| MoneyError::Overflow)?;
        Ok(Self { minor, currency })
    }

    /// Formats the amount without grouping, e.g. `-1234.50`.
    ///
    /// Grouping is a display concern and belongs to the UI, which follows the
    /// user's locale (NFR-050).
    #[must_use]
    pub fn to_plain_string(self) -> String {
        let exponent = usize::from(self.currency.exponent());
        let magnitude = i128::from(self.minor).abs();
        let scale = i128::from(self.currency.minor_units_per_major());
        let whole = magnitude / scale;
        let fraction = magnitude % scale;
        let sign = if self.minor < 0 { "-" } else { "" };

        if exponent == 0 {
            format!("{sign}{whole}")
        } else {
            format!("{sign}{whole}.{fraction:0width$}", width = exponent)
        }
    }

    fn ensure_same(self, other: Currency) -> Result<(), MoneyError> {
        if self.currency == other {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch {
                left: self.currency,
                right: other,
            })
        }
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.to_plain_string(), self.currency)
    }
}
