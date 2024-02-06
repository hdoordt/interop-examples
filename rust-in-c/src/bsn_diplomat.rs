use std::fmt::Display;

pub use ffi::*;

// Errors are pretty vague
#[diplomat::bridge]
pub mod ffi {
    use diplomat_runtime::DiplomatWriteable;
    use std::fmt::Write;

    // Clone does not get exposed, even though it could
    #[derive(Debug, Clone)]
    pub enum BsnError {
        InvalidBsn,
    }

    impl BsnError {
        // For some reason can't receive &self?
        #[allow(clippy::result_unit_err)]
        pub fn fmt_display(this: &Self, w: &mut DiplomatWriteable) -> Result<(), ()> {
            write!(w, "{}", this).map_err(|_e| ())
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    #[diplomat::opaque]
    // Lifetimes not supported
    pub struct Bsn {
        pub(crate) inner: String,
    }

    impl Bsn {
        // Can only return boxed refs of Self
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
// Need to define trait impls outside bridge block

impl std::error::Error for BsnError {}

impl Display for BsnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BsnError::InvalidBsn => write!(f, "Invalid BSN number"),
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
