use crate::currency::FormattableCurrency;
use crate::format::{Formatter, Params, Position};
use crate::locale::LocalFormat;
use crate::MoneyError;
use rust_decimal::Decimal;
use rust_decimal_macros::*;
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use std::str::FromStr;

/// Represents an amount of a given currency.
///
/// Money represents financial amounts through a Decimal (owned) and a Currency (reference).
/// Operations on Money objects always create new instances of Money, with the exception
/// of `round()`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Money<'a, T: FormattableCurrency> {
    amount: Decimal,
    currency: &'a T,
}

impl<'a, T: FormattableCurrency> Add for Money<'a, T> {
    type Output = Money<'a, T>;
    fn add(self, other: Money<'a, T>) -> Money<'a, T> {
        if self.currency != other.currency {
            panic!();
        }
        Money::from_decimal(self.amount + other.amount, self.currency)
    }
}

impl<'a, T: FormattableCurrency> AddAssign for Money<'a, T> {
    fn add_assign(&mut self, other: Self) {
        if self.currency != other.currency {
            panic!();
        }
        *self = Self {
            amount: self.amount + other.amount,
            currency: self.currency,
        };
    }
}

impl<'a, T: FormattableCurrency> Sub for Money<'a, T> {
    type Output = Money<'a, T>;
    fn sub(self, other: Money<'a, T>) -> Money<'a, T> {
        if self.currency != other.currency {
            panic!();
        }
        Money::from_decimal(self.amount - other.amount, self.currency)
    }
}

impl<'a, T: FormattableCurrency> SubAssign for Money<'a, T> {
    fn sub_assign(&mut self, other: Self) {
        if self.currency != other.currency {
            panic!();
        }

        *self = Self {
            amount: self.amount - other.amount,
            currency: self.currency,
        };
    }
}

macro_rules! impl_mul_div {
    ($type:ty) => {
        impl<'a, T: FormattableCurrency> Mul<$type> for Money<'a, T> {
            type Output = Money<'a, T>;

            fn mul(self, rhs: $type) -> Money<'a, T> {
                let rhs = Decimal::from_str(&rhs.to_string()).unwrap();
                Money::from_decimal(self.amount * rhs, self.currency)
            }
        }

        impl<'a, T: FormattableCurrency> Mul<Money<'a, T>> for $type {
            type Output = Money<'a, T>;

            fn mul(self, rhs: Money<'a, T>) -> Money<'a, T> {
                let lhs = Decimal::from_str(&self.to_string()).unwrap();
                Money::from_decimal(rhs.amount * lhs, rhs.currency)
            }
        }

        impl<'a, T: FormattableCurrency> MulAssign<$type> for Money<'a, T> {
            fn mul_assign(&mut self, rhs: $type) {
                *self = Self {
                    amount: self.amount * Decimal::from(rhs),
                    currency: self.currency,
                };
            }
        }

        impl<'a, T: FormattableCurrency> Div<$type> for Money<'a, T> {
            type Output = Money<'a, T>;

            fn div(self, rhs: $type) -> Money<'a, T> {
                let rhs = Decimal::from_str(&rhs.to_string()).unwrap();
                Money::from_decimal(self.amount / rhs, self.currency)
            }
        }

        impl<'a, T: FormattableCurrency> Div<Money<'a, T>> for $type {
            type Output = Money<'a, T>;

            fn div(self, rhs: Money<'a, T>) -> Money<'a, T> {
                let lhs = Decimal::from_str(&self.to_string()).unwrap();
                Money::from_decimal(lhs / rhs.amount, rhs.currency)
            }
        }

        impl<'a, T: FormattableCurrency> DivAssign<$type> for Money<'a, T> {
            fn div_assign(&mut self, rhs: $type) {
                *self = Self {
                    amount: self.amount / Decimal::from(rhs),
                    currency: self.currency,
                };
            }
        }
    };
}

impl_mul_div!(isize);
impl_mul_div!(i8);
impl_mul_div!(i16);
impl_mul_div!(i32);
impl_mul_div!(i64);
impl_mul_div!(usize);
impl_mul_div!(u8);
impl_mul_div!(u16);
impl_mul_div!(u32);
impl_mul_div!(u64);
impl_mul_div!(Decimal);

impl<'a, T: FormattableCurrency> PartialOrd for Money<'a, T> {
    fn partial_cmp(&self, other: &Money<'a, T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, T: FormattableCurrency> Ord for Money<'a, T> {
    fn cmp(&self, other: &Money<'a, T>) -> Ordering {
        if self.currency != other.currency {
            panic!();
        }
        self.amount.cmp(&other.amount)
    }
}

impl<'a, T: FormattableCurrency> Money<'a, T> {
    /// Creates a Money object given an amount string and a currency str.
    ///
    /// Supports fuzzy amount strings like "100", "100.00" and "-100.00"
    pub fn from_str(amount: &str, currency: &'a T) -> Result<Money<'a, T>, MoneyError> {
        let format = LocalFormat::from_locale(currency.locale());
        let amount_parts: Vec<&str> = amount.split(format.exponent_separator).collect();

        let mut split_decimal: Vec<&str> = amount_parts[0].split(format.digit_separator).collect();
        let mut parsed_decimal = split_decimal.concat();

        // Sanity check whether the decimal part can be parsed as an i32
        i32::from_str(&parsed_decimal)?;

        // Sanity check the decimal seperation
        for &num in format.digit_separator_pattern().iter() {
            if split_decimal.len() <= 1 {
                break;
            }
            let current = split_decimal.pop().unwrap();
            if current.len() != num {
                return Err(MoneyError::InvalidAmount);
            }
        }

        if amount_parts.len() == 1 {
            parsed_decimal += ".";
            for _ in 0..currency.exponent() {
                parsed_decimal += "0";
            }
        } else if amount_parts.len() == 2 {
            i32::from_str(&amount_parts[1])?;
            parsed_decimal = parsed_decimal + "." + amount_parts[1];
        } else {
            return Err(MoneyError::InvalidAmount);
        }

        let decimal = Decimal::from_str(&parsed_decimal).unwrap();
        Ok(Money::from_decimal(decimal, &currency))
    }

    /// Creates a Money object given an integer and a currency reference.
    ///
    /// The integer represents minor units of the currency (e.g. 1000 -> 10.00 in USD )
    pub fn from_minor(amount: i64, currency: &'a T) -> Money<'a, T> {
        let amount = Decimal::new(amount, currency.exponent());
        Money { amount, currency }
    }

    /// Creates a Money object given an integer and a currency reference.
    ///
    /// The integer represents major units of the currency (e.g. 1000 -> 1,000 in USD )
    pub fn from_major(amount: i64, currency: &'a T) -> Money<'a, T> {
        let amount = Decimal::new(amount, 0);
        Money { amount, currency }
    }

    /// Creates a Money object given a major and minor amount and a currency reference.
    ///
    /// The major amount represents major units of the currency (e.g. 1000 -> 1,000 in USD)
    /// The integer represents minor units of the currency (e.g. 1000 -> 10.00 in USD )
    pub fn from_major_and_minor(major: i64, minor: i64, currency: &'a T) -> Money<'a, T> {
        Money::from_major(major, currency) + Money::from_minor(minor, currency)
    }

    /// Creates a Money object given a decimal amount and a currency reference.
    pub fn from_decimal(amount: Decimal, currency: &'a T) -> Money<'a, T> {
        Money { amount, currency }
    }

    /// Returns a reference to the Decimal amount.
    pub fn amount(&self) -> &Decimal {
        &self.amount
    }

    /// Returns the minor amount required to represent the current value.
    ///
    /// For currencies that have no minor units, this is simply the major
    /// amount (e.g. ￥500 will return `Some(500)`);
    ///
    /// On overflow `None` is returned.
    ///
    /// Requires the `maths` feature to be enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rusty_money::{define_currency_set, Money};
    ///
    /// define_currency_set!(
    ///     test {
    ///         USD: {
    ///             code: "USD",
    ///             exponent: 2,
    ///             locale: EnUs,
    ///             minor_units: 100,
    ///             name: "USD",
    ///             symbol: "$",
    ///             symbol_first: true,
    ///         }
    ///     }
    /// );
    ///
    /// let money = Money::from_minor(350, test::USD);
    /// let minor_amount = money.minor_amount().unwrap();
    /// assert_eq!(350, minor_amount);
    ///
    /// let money = Money::from_major(120, test::USD);
    /// let minor_amount = money.minor_amount().unwrap();
    /// assert_eq!(12_000, minor_amount); // $120 is 12,000¢
    ///
    /// let m1 = Money::from_major(5, test::USD);
    /// let m2 = Money::from_minor(350, test::USD);
    /// let money = m1 + m2;
    /// let minor_amount = money.minor_amount().unwrap();
    /// assert_eq!(850, minor_amount); // $5 + 350¢ = $8.50
    /// ```
    #[cfg(feature = "maths")]
    pub fn minor_amount(&self) -> Option<i64> {
        use rust_decimal::prelude::{MathematicalOps, ToPrimitive};

        let amount = self.amount.clone();
        let ten = Decimal::TEN;

        ten.checked_powi(self.currency.exponent() as i64)
            .and_then(|multiplier| (amount * multiplier).to_i64())
    }

    /// Returns the Currency type.
    pub fn currency(&self) -> &'a T {
        self.currency
    }

    /// Returns true if amount == 0.
    pub fn is_zero(&self) -> bool {
        self.amount == dec!(0.0)
    }

    /// Returns true if amount > 0.
    pub fn is_positive(&self) -> bool {
        self.amount.is_sign_positive() && self.amount != dec!(0.0)
    }

    /// Returns true if amount < 0.
    pub fn is_negative(&self) -> bool {
        self.amount.is_sign_negative() && self.amount != dec!(0.0)
    }

    /// Divides money equally into n shares.
    ///
    /// If the divison cannot be applied perfectly, it allocates the remainder
    /// to some of the shares.
    pub fn allocate_to(&self, number: i32) -> Result<Vec<Money<'a, T>>, MoneyError> {
        let ratios: Vec<i32> = (0..number).map(|_| 1).collect();
        self.allocate(ratios)
    }

    /// Divides money into n shares according to a particular ratio.
    ///  
    /// If the divison cannot be applied perfectly, it allocates the remainder
    /// to some of the shares.
    pub fn allocate(&self, ratios: Vec<i32>) -> Result<Vec<Money<'a, T>>, MoneyError> {
        if ratios.is_empty() {
            return Err(MoneyError::InvalidRatio);
        }

        let ratios: Vec<Decimal> = ratios
            .iter()
            .map(|x| Decimal::from_str(&x.to_string()).unwrap())
            .collect();

        let mut remainder = self.amount;
        let ratio_total: Decimal = ratios.iter().fold(dec!(0.0), |acc, x| acc + x);

        let mut allocations: Vec<Money<'a, T>> = Vec::new();

        for ratio in ratios {
            if ratio <= dec!(0.0) {
                return Err(MoneyError::InvalidRatio);
            }

            let share = (self.amount * ratio / ratio_total).floor();

            allocations.push(Money::from_decimal(share, self.currency));
            remainder -= share;
        }

        if remainder < dec!(0.0) {
            panic!("Remainder was negative, should be 0 or positive");
        }

        if remainder - remainder.floor() != dec!(0.0) {
            panic!("Remainder is not an integer, should be an integer");
        }

        let mut i: usize = 0;
        while remainder > dec!(0.0) {
            allocations[i].amount += dec!(1.0);
            remainder -= dec!(1.0);
            i += 1;
        }
        Ok(allocations)
    }

    /// Returns a `Money` rounded to the specified number of minor units using the rounding strategy.
    pub fn round(&self, digits: u32, strategy: Round) -> Money<'a, T> {
        let mut money = self.clone();

        money.amount = match strategy {
            Round::HalfDown => money
                .amount
                .round_dp_with_strategy(digits, rust_decimal::RoundingStrategy::MidpointTowardZero),
            Round::HalfUp => money.amount.round_dp_with_strategy(
                digits,
                rust_decimal::RoundingStrategy::MidpointAwayFromZero,
            ),
            Round::HalfEven => money.amount.round_dp_with_strategy(
                digits,
                rust_decimal::RoundingStrategy::MidpointNearestEven,
            ),
        };

        money
    }
}

/// Strategies that can be used to round Money.
///
/// For more details, see (rust_decimal::Rounding_Strategy)[https://docs.rs/rust_decimal/1.1.0/rust_decimal/enum.RoundingStrategy.html]
pub enum Round {
    HalfUp,
    HalfDown,
    HalfEven,
}

impl<'a, T: FormattableCurrency + FormattableCurrency> fmt::Display for Money<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let currency = self.currency;
        let format = LocalFormat::from_locale(currency.locale());

        let mut format_params = Params {
            digit_separator: format.digit_separator,
            exponent_separator: format.exponent_separator,
            separator_pattern: format.digit_separator_pattern(),
            rounding: Some(currency.exponent()),
            symbol: Some(currency.symbol()),
            code: Some(currency.code()),
            ..Default::default()
        };

        if currency.symbol_first() {
            format_params.positions = vec![Position::Sign, Position::Symbol, Position::Amount];
            write!(f, "{}", Formatter::money(self, format_params))
        } else {
            format_params.positions = vec![Position::Sign, Position::Amount, Position::Symbol];
            write!(f, "{}", Formatter::money(self, format_params))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_currency_set;

    define_currency_set!(
        test {
            USD: {
                code: "USD",
                exponent: 2,
                locale: EnUs,
                minor_units: 100,
                name: "USD",
                symbol: "$",
                symbol_first: true,
            },
            GBP : {
                code: "GBP",
                exponent: 2,
                locale: EnUs,
                minor_units: 1,
                name: "British Pound",
                symbol: "£",
                symbol_first: true,
            },
            EUR : {
                code: "EUR",
                exponent: 2,
                locale: EnEu,
                minor_units: 1,
                name: "Euro",
                symbol: "€",
                symbol_first: true,
            },
            INR : {
                code: "INR",
                exponent: 2,
                locale: EnIn,
                minor_units: 50,
                name: "Indian Rupee",
                symbol: "₹",
                symbol_first: true,
            },
            BHD : {
                code: "BHD",
                exponent: 3,
                locale: EnUs,
                minor_units: 5,
                name: "Bahraini Dinar",
                symbol: "ب.د",
                symbol_first: true,
            },
            AED : {
                code: "AED",
                exponent: 2,
                locale: EnUs,
                minor_units: 25,
                name: "United Arab Emirates Dirham",
                symbol: "د.إ",
                symbol_first: false,
            }
        }
    );

    #[test]
    fn money_major_minor() {
        let _usd = test::find("USD"); // Prevents unused code warnings from the defined module.
        let major_usd = Money::from_major(10, test::USD);
        let minor_usd = Money::from_minor(1000, test::USD);
        assert_eq!(major_usd, minor_usd);
    }

    #[test]
    fn money_from_string_parses_correctly() {
        let expected_money = Money::from_minor(2999, test::GBP);
        let money = Money::from_str("29.99", test::GBP).unwrap();
        assert_eq!(money, expected_money);
    }

    #[test]
    fn money_from_string_parses_signs() {
        let expected_money = Money::from_minor(-300, test::GBP);
        let money = Money::from_str("-3", test::GBP).unwrap();
        assert_eq!(money, expected_money);

        let expected_money = Money::from_minor(300, test::GBP);
        let money = Money::from_str("+3", test::GBP).unwrap();
        assert_eq!(money, expected_money);
    }

    #[test]
    fn money_from_string_ignores_separators() {
        let expected_money = Money::from_minor(100000000, test::GBP);
        let money = Money::from_str("1,000,000", test::GBP).unwrap();
        assert_eq!(money, expected_money);
    }

    #[test]
    fn money_from_string_decimal_sanity() {
        let money = Money::from_str("1,00.00", test::GBP);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);

        let money = Money::from_str("1.00,00", test::EUR);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);

        let money = Money::from_str("1.00.000,00", test::EUR);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);

        let money = Money::from_str("1.00.000.000,00", test::EUR);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);

        let money = Money::from_str("1,00.00", test::INR);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);

        let money = Money::from_str("1.000.000.00", test::INR);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);
    }

    #[test]
    fn money_from_string_parse_errs() {
        // If the delimiter preceeds the separators
        let money = Money::from_str("1.0000,000", test::GBP);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);

        // If there are multiple delimiters
        let money = Money::from_str("1.0000.000", test::GBP);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);

        // If there is an unrecognized character
        let money = Money::from_str("1.0000!000", test::GBP);
        assert_eq!(money.unwrap_err(), MoneyError::InvalidAmount);

        // If there are no characters other than separators
        let exponent_separator_only = Money::from_str(",", test::GBP);
        let amount_separator_only = Money::from_str(".", test::GBP);
        let both_separators = Money::from_str(",,.", test::GBP);
        assert_eq!(
            exponent_separator_only.unwrap_err(),
            MoneyError::InvalidAmount
        );
        assert_eq!(
            amount_separator_only.unwrap_err(),
            MoneyError::InvalidAmount
        );
        assert_eq!(both_separators.unwrap_err(), MoneyError::InvalidAmount);
    }

    #[test]
    fn money_format_rounds_exponent() {
        // // 19.999 rounds to 20 for USD
        let money = Money::from_str("19.9999", test::USD).unwrap();
        assert_eq!("$20.00", format!("{}", money));

        // // 29.111 rounds to 29.11 for USD
        let money = Money::from_str("29.111", test::USD).unwrap();
        assert_eq!("$29.11", format!("{}", money));

        // // 39.1155 rounds to 39.116 for BHD
        let money = Money::from_str("39.1155", test::BHD).unwrap();
        assert_eq!("ب.د39.116", format!("{}", money));
    }

    #[test]
    fn money_addition_and_subtraction() {
        // Addition
        assert_eq!(
            Money::from_major(2, test::USD),
            Money::from_major(1, test::USD) + Money::from_major(1, test::USD)
        );
        // Subtraction
        assert_eq!(
            Money::from_major(0, test::USD),
            Money::from_major(1, test::USD) - Money::from_major(1, test::USD)
        );
    }

    #[test]
    #[should_panic]
    fn money_addition_panics_on_different_currencies() {
        let _no_op = Money::from_minor(100, test::USD) + Money::from_minor(100, test::GBP);
    }

    #[test]
    #[should_panic]
    fn money_subtractionpanics_on_different_currencies() {
        let _no_op = Money::from_minor(100, test::USD) - Money::from_minor(100, test::GBP);
    }

    #[test]
    #[should_panic]
    fn money_add_assign_panics_on_different_currencies() {
        let mut money = Money::from_minor(100, test::USD);
        money += Money::from_minor(100, test::GBP);
    }

    #[test]
    #[should_panic]
    fn money_sub_assign_panics_on_different_currencies() {
        let mut money = Money::from_minor(100, test::USD);
        money -= Money::from_minor(100, test::GBP);
    }

    #[test]
    fn money_multiplication_and_division() {
        // Multiplication integer
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(100, test::USD) * 2
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(-100, test::USD) * -2
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            -2 * Money::from_minor(-100, test::USD)
        );

        // Multiplication decimal
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(100, test::USD) * Decimal::new(2, 0)
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(-100, test::USD) * Decimal::new(-2, 0)
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            Decimal::new(-2, 0) * Money::from_minor(-100, test::USD)
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(400, test::USD) * Decimal::new(5, 1)
        );

        // Division integer
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(400, test::USD) / 2
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(-400, test::USD) / -2
        );
        assert_eq!(
            Money::from_minor(50, test::USD),
            -1 / Money::from_minor(-200, test::USD)
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(-200, test::USD) / -1
        );

        // Division decimal
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(400, test::USD) / Decimal::new(2, 0)
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(-400, test::USD) / Decimal::new(-2, 0)
        );
        assert_eq!(
            Money::from_minor(50, test::USD),
            Decimal::new(-1, 0) / Money::from_minor(-200, test::USD)
        );
        assert_eq!(
            Money::from_minor(200, test::USD),
            Money::from_minor(-200, test::USD) / Decimal::new(-1, 0)
        );
        assert_eq!(
            Money::from_minor(400, test::USD),
            Money::from_minor(-200, test::USD) / Decimal::new(-5, 1)
        );

        //MulAssign integer
        let mut money = Money::from_minor(100, test::USD);
        money *= 2;
        assert_eq!(Money::from_minor(200, test::USD), money);

        //MulAssign decimal
        let mut money = Money::from_minor(100, test::USD);
        money *= Decimal::new(2, 0);
        assert_eq!(Money::from_minor(200, test::USD), money);

        //DivAssign integer
        let mut money = Money::from_minor(100, test::USD);
        money /= -2;
        assert_eq!(Money::from_minor(-50, test::USD), money);

        //DivAssign decimal
        let mut money = Money::from_minor(100, test::USD);
        money /= Decimal::new(-2, 0);
        assert_eq!(Money::from_minor(-50, test::USD), money);
    }

    #[test]
    fn money_comparison() {
        // Greater Than
        assert_eq!(
            true,
            Money::from_minor(200, test::USD) > Money::from_minor(100, test::USD)
        );
        // Less Than
        assert_eq!(
            false,
            Money::from_minor(200, test::USD) < Money::from_minor(100, test::USD)
        );
        // Equals
        assert_eq!(
            true,
            Money::from_minor(100, test::USD) == Money::from_minor(100, test::USD)
        );
        assert_eq!(
            false,
            Money::from_minor(100, test::USD) == Money::from_minor(100, test::GBP)
        );
        // is positive
        assert_eq!(true, Money::from_minor(100, test::USD).is_positive());
        assert_eq!(false, Money::from_minor(0, test::USD).is_positive());
        assert_eq!(false, Money::from_minor(-100, test::USD).is_positive());
        // is zero
        assert_eq!(true, Money::from_minor(0, test::USD).is_zero());
        assert_eq!(false, Money::from_minor(100, test::USD).is_zero());
        assert_eq!(false, Money::from_minor(-100, test::USD).is_zero());
        // is negative
        assert_eq!(true, Money::from_minor(-100, test::USD).is_negative());
        assert_eq!(false, Money::from_minor(100, test::USD).is_negative());
        assert_eq!(false, Money::from_minor(0, test::USD).is_negative());
    }

    #[test]
    #[should_panic]
    fn money_ops_greater_than_panics_on_different_currencies() {
        assert!(Money::from_minor(100, test::USD) < Money::from_minor(100, test::GBP));
    }

    #[test]
    #[should_panic]
    fn money_ops_less_than_panics_on_different_currencies() {
        assert!(Money::from_minor(100, test::USD) < Money::from_minor(100, test::GBP));
    }

    #[test]
    fn money_allocate() {
        let money = Money::from_minor(1_100, test::USD);
        let allocs = money.allocate(vec![1, 1, 1]).unwrap();
        let expected_results = vec![
            Money::from_minor(400, test::USD),
            Money::from_minor(400, test::USD),
            Money::from_minor(300, test::USD),
        ];
        assert_eq!(expected_results, allocs);

        // Error if the ratio vector is empty
        let monies = Money::from_minor(100, test::USD).allocate(Vec::new());
        assert_eq!(monies.unwrap_err(), MoneyError::InvalidRatio);

        // Error if any ratio is zero
        let monies = Money::from_minor(100, test::USD).allocate(vec![1, 0]);
        assert_eq!(monies.unwrap_err(), MoneyError::InvalidRatio);
    }

    #[test]
    fn money_allocate_to() {
        let money = Money::from_minor(1_100, test::USD);
        let monies = money.allocate_to(3).unwrap();
        let expected_results = vec![
            Money::from_minor(400, test::USD),
            Money::from_minor(400, test::USD),
            Money::from_minor(300, test::USD),
        ];
        assert_eq!(expected_results, monies);

        let monies = Money::from_minor(100, test::USD).allocate_to(0);
        assert_eq!(monies.unwrap_err(), MoneyError::InvalidRatio);
    }

    #[test]
    fn money_fmt_separates_digits() {
        let usd = Money::from_minor(0, test::USD); // Zero Dollars
        let expected_usd_fmt = "$0.00";
        assert_eq!(format!("{}", usd), expected_usd_fmt);

        let usd = Money::from_minor(10_000_000, test::USD); // One Hundred Thousand Dollars
        let expected_usd_fmt = "$100,000.00";
        assert_eq!(format!("{}", usd), expected_usd_fmt);

        let usd = Money::from_minor(-10_000_000, test::USD); // - One Hundred Thousand Dollars
        let expected_usd_fmt = "-$100,000.00";
        assert_eq!(format!("{}", usd), expected_usd_fmt);

        let usd = Money::from_minor(100_000_000_000, test::USD); // 1 Billion Dollars
        let expected_usd_fmt = "$1,000,000,000.00";
        assert_eq!(format!("{}", usd), expected_usd_fmt);

        let inr = Money::from_minor(10_000_000, test::INR); // 1 Lakh Rupees
        let expected_inr_fmt = "₹1,00,000.00";
        assert_eq!(format!("{}", inr), expected_inr_fmt);

        let inr = Money::from_minor(-1_000_000_000, test::INR); // - 1 Crore Rupees
        let expected_inr_fmt = "-₹1,00,00,000.00";
        assert_eq!(format!("{}", inr), expected_inr_fmt);
    }

    #[test]
    fn money_fmt_places_symbols_correctly() {
        let money = Money::from_minor(0, test::USD);
        let expected_fmt = "$0.00";
        assert_eq!(format!("{}", money), expected_fmt);

        let money = Money::from_minor(0, test::AED);
        let expected_fmt = "0.00د.إ";
        assert_eq!(format!("{}", money), expected_fmt);
    }

    #[test]
    fn money_fmt_uses_correct_separators() {
        let money = Money::from_minor(100_000, test::EUR);
        let expected_fmt = "€1.000,00";
        assert_eq!(format!("{}", money), expected_fmt);
    }

    #[test]
    // Dividing 20 by 3 rounds to 6.67 in USD and 6.667 in BHD
    fn money_precision_and_rounding() {
        let expected_money = Money::from_minor(667, test::USD);
        let mut money = Money::from_minor(2_000, test::USD);
        money /= 3;
        assert_eq!(money.round(2, Round::HalfEven), expected_money);

        let expected_money = Money::from_minor(6_667, test::BHD);
        let mut money = Money::from_minor(20_000, test::BHD);
        money /= 3;
        assert_eq!(money.round(3, Round::HalfEven), expected_money);
    }

    #[test]
    #[cfg(feature = "maths")]
    fn money_integer_amounts() {
        let money = Money::from_minor(667, test::USD);
        assert_eq!(Some(667), money.minor_amount());

        let m1 = Money::from_major(332, test::USD);
        let m2 = Money::from_minor(m1.minor_amount().unwrap(), test::USD);
        assert_eq!(m1, m2);
    }
}
