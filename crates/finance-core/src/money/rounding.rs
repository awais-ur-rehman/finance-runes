//! Rounding policy for divisions that do not land on a whole minor unit.

/// How to round when a division leaves a remainder.
///
/// Every division in the domain names its policy, so rounding is never an
/// accident of the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    /// Toward zero.
    Down,
    /// Away from zero.
    Up,
    /// To the nearest minor unit, with ties away from zero.
    HalfUp,
    /// To the nearest minor unit, with ties to the even minor unit.
    HalfEven,
}

impl Rounding {
    /// Rounds `numerator / denominator` to an integer under this policy.
    pub(crate) fn apply(self, numerator: i128, denominator: i128) -> i128 {
        debug_assert!(denominator != 0, "caller must reject a zero divisor");
        let quotient = numerator / denominator;
        let remainder = numerator % denominator;
        if remainder == 0 {
            return quotient;
        }

        let same_sign = (numerator < 0) == (denominator < 0);
        let direction = if same_sign { 1 } else { -1 };
        let remainder = remainder.abs();
        let denominator = denominator.abs();

        let bump = match self {
            Self::Down => false,
            Self::Up => true,
            Self::HalfUp => remainder * 2 >= denominator,
            Self::HalfEven => match (remainder * 2).cmp(&denominator) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => quotient % 2 != 0,
            },
        };

        if bump { quotient + direction } else { quotient }
    }
}
