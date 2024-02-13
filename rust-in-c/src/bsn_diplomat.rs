use ffi::*;

#[diplomat::bridge]
mod ffi {

    #[derive(Debug)]
    pub enum BsnError {
        InvalidBsn,
    }

    impl BsnError {
        #[allow(clippy::result_unit_err)]
        pub fn fmt_display(
            this: &Self,
            w: &mut diplomat_runtime::DiplomatWriteable,
        ) -> Result<(), ()> {
            use std::fmt::Write;
            write!(w, "{}", this).map_err(|_e| ())
        }
    }

    /// Represents a valid BSN
    #[diplomat::opaque]
    pub struct Bsn {
        pub(super) inner: String,
    }

    impl Bsn {
        pub fn try_new_boxed(bsn: &str) -> Result<Box<Self>, BsnError> {
            Self::try_new(bsn).map(Box::new)
        }

        pub fn validate(bsn: &str) -> bool {
            if !matches!(bsn.len(), 8 | 9) {
                return false;
            }
            let sum = [9, 8, 7, 6, 5, 4, 3, 2, -1]
                .iter()
                .zip(bsn.chars())
                .try_fold(0, |sum, (multiplier, digit)| {
                    let Some(digit) = digit.to_digit(10) else {
                        return Err(BsnError::InvalidBsn);
                    };
                    Ok(sum + (multiplier * digit as i32))
                });

            let Ok(sum) = sum else {
                return false;
            };

            sum % 11 == 0
        }
    }
}

impl Bsn {
    pub fn try_new(bsn: &str) -> Result<Self, BsnError> {
        let bsn = bsn.to_string();
        if Self::validate(&bsn) {
            Ok(Self { inner: bsn })
        } else {
            Err(BsnError::InvalidBsn)
        }
    }
}

impl std::error::Error for BsnError {}

impl std::fmt::Display for BsnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BsnError::InvalidBsn => write!(f, "Invalid BSN number"),
        }
    }
}
