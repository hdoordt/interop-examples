use std::{
    ffi::{c_char, CStr},
    fmt::Display,

    marker::PhantomData,
    slice, str,
};

#[derive(Debug)]
#[repr(C)]
pub enum Error {
    InvalidBsn,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidBsn => write!(f, "Invalid BSN number"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct Bsn<'inner> {
    inner: *const u8,
    len: usize,
    // &str is represented as a pointer and a length,
    // but pointers have no lifetime associated with them,
    // so we add a PhantomData to allow Rust code using the Bsn
    // to be correct.
    _marker: PhantomData<&'inner ()>,
}

impl<'inner> Bsn<'inner> {
    // This constructor ensures that the lifetime of `Bsn`
    // corresponds to the lifetime of the passed `&str`
    pub fn try_new(bsn: &'inner str) -> Result<Self, Error> {
        if Self::validate(bsn) {
            Ok(Self {
                inner: bsn.as_ptr(),
                len: bsn.len(),
                _marker: PhantomData,
            })
        } else {
            Err(Error::InvalidBsn)
        }
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
                    return Err(Error::InvalidBsn);
                };
                Ok(sum + (multiplier * digit as i32))
            });

        let Ok(sum) = sum else {
            return false;
        };

        sum % 11 == 0
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            // Note (unsafe): Bsn can only be created from valid, UTF-8 encoded
            // strings by calling [Bsn::try_new]
            let s: &[u8] = slice::from_raw_parts(self.inner, self.len);
            str::from_utf8_unchecked(s)
        }
    }
}

#[repr(C)]
enum BsnTryNewResult {
    BsnTryNewResultOk(Bsn<'static>),
    BsnTryNewResultErr(Error),
}

impl From<Result<Bsn<'static>, Error>> for BsnTryNewResult {
    fn from(res: Result<Bsn<'static>, Error>) -> Self {
        match res {
            Ok(bsn) => Self::BsnTryNewResultOk(bsn),
            Err(e) => Self::BsnTryNewResultErr(e),
        }
    }
}

/// Validate a BSN string and create a Bsn object. If the BSN is invalid,
/// or if the passed string is not valid UTF-8, returns an Error.
///
/// # Safety:
/// This function uses [CStr::from_ptr] to convert the char pointer into a CStr,
/// and as such the caller must uphold the same invariants. Furthermore you
/// _must_ ensure that the produced `Bsn` does not outlive the string data passed
/// to this function.
#[no_mangle]
unsafe extern "C" fn bsn_try_new(bsn: *const c_char) -> BsnTryNewResult {
    let Ok(bsn): Result<&'static str, _> = CStr::from_ptr(bsn).to_str() else {
        return Err(Error::InvalidBsn).into();
    };
    Bsn::try_new(bsn).into()
}


/// Checks whether the passed string represents a valid BSN.
/// Returns `false` if the passed string is not UTF-8 encoded.
///
/// # Safety
/// This function uses [CStr::from_ptr] to create a [CStr]
/// out of the passed raw pointer, and
/// this function exhibits Undefined Behavior in the same cases as
/// `from_ptr`.
#[no_mangle]
unsafe extern "C" fn bsn_validate(bsn: *const c_char) -> bool {
    let Ok(bsn) = CStr::from_ptr(bsn).to_str() else {
        return false
    };
    Bsn::validate(bsn)
}

/// Formats the error message into the passed buffer, returning the length of
/// the message.
///
/// # Safety:
/// This function uses [slice::from_raw_parts_mut] to create a byte slice from
/// `buf` and `len`, and as such the caller must uphold the same invariants.
#[no_mangle]
unsafe extern "C" fn error_display(error: &Error, buf: *mut c_char, len: usize) -> usize {
    use std::io::Write;

    let buf = buf as *mut u8;
    let buf = slice::from_raw_parts_mut(buf, len);
    // A Cursor allows us to use `write!` on an in-memory buffer. Neat!
    let mut buf = std::io::Cursor::new(buf);
    // Don't forget to nul-terminate
    write!(&mut buf, "{}\0", error).unwrap();
    buf.position() as usize
}
