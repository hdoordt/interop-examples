use std::io::Read;

use heapless::Vec as HeaplessVec;
use nalgebra as na;
use struson::reader::{JsonReader, JsonStreamReader};

type Result<T> = core::result::Result<T, Error>;

/// A [nalgebra::Matrix] that is backed by some other means of storage.
/// Allows for backing [nalgebra::Matrix] with some stack-based
/// storage, like [HeaplessVec]
pub type MatrixView<'buf> = na::Matrix<
    f64,
    na::Dyn,
    na::Dyn,
    na::ViewStorage<'buf, f64, na::Dyn, na::Dyn, na::Const<1>, na::Dyn>,
>;

/// A buffer into which matrix data can be stored
#[derive(serde::Deserialize, Debug)]
pub struct MatrixBuf {
    d: HeaplessVec<f64, { 6 * 6 }>,
    n: usize,
}

impl MatrixBuf {
    pub fn view<'buf>(&'buf self) -> MatrixView<'buf> {
        let rows = self.d.len() / self.n;
        let cols = self.n;
        MatrixView::from_slice_generic(&self.d, na::Dyn(rows), na::Dyn(cols))
    }
}

/// An operation that can be performed on a Matrix
#[derive(serde::Deserialize, Debug)]
#[serde(tag = "code", rename_all = "lowercase")]
enum Operation {
    /// Perform the dot product of some matrix with `rhs`
    Dot { rhs: MatrixBuf },
    // TODO support other operations
}

impl Operation {
    /// Evaluate the operation, given a [MatrixBuf]
    fn eval(self, lhs: MatrixBuf) -> MatrixBuf {
        match self {
            Operation::Dot { rhs } => {
                let dot = lhs.view().dot(&rhs.view());
                MatrixBuf {
                    d: HeaplessVec::from_slice(&[dot]).unwrap(),
                    n: 1,
                }
            }
        }
    }
}

/// A single piece of work
#[derive(serde::Deserialize, Debug)]
pub struct PieceOfWork {
    lhs: MatrixBuf,
    op: HeaplessVec<Operation, 5>,
}

impl PieceOfWork {
    /// Execute a single [PieceOfWork] that
    /// has already been read fully into memory.
    pub fn exec(self) -> MatrixBuf {
        let res = self
            .op
            .into_iter()
            .fold(self.lhs, |rhs: MatrixBuf, op| op.eval(rhs));

        res
    }

    /// Read and execute a single [PieceOfWork]
    pub fn exec_streamingly<R: Read>(reader: &mut JsonStreamReader<R>) -> Result<MatrixBuf> {
        reader.begin_object()?;

        // First, we need the `lhs` object
        let "lhs" = reader.next_name()? else {
            return Err(Error::Json("lhs"));
        };
        let lhs: MatrixBuf = reader.deserialize_next()?;

        // Then, we read the `op` array element-by-element
        // We execute operations as they come in
        let "op" = reader.next_name()? else {
            return Err(Error::Json("op"));
        };

        reader.begin_array()?;

        let mut res = lhs;
        while reader.has_next()? {
            let op: Operation = reader.deserialize_next()?;
            res = op.eval(res);
        }

        reader.end_array()?;

        reader.end_object()?;

        Ok(res)
    }
}

#[derive(Debug)]
pub enum Error {
    Json(&'static str),
    Struson(struson::reader::ReaderError),
    Serde(struson::serde::DeserializerError),
}

impl From<struson::reader::ReaderError> for Error {
    fn from(e: struson::reader::ReaderError) -> Self {
        Self::Struson(e)
    }
}

impl From<struson::serde::DeserializerError> for Error {
    fn from(e: struson::serde::DeserializerError) -> Self {
        Self::Serde(e)
    }
}

#[cfg(test)]
mod test {
    use struson::reader::{JsonReader, JsonStreamReader};

    use crate::PieceOfWork;

    #[test]
    fn it_deserializes() {
        let json = include_str!("../op.json");
        let [_work]: [PieceOfWork; 1] = dbg!(serde_json::from_str(json).unwrap());
    }

    #[test]
    fn it_works() {
        let json = include_str!("../op.json");
        let [work]: [PieceOfWork; 1] = serde_json::from_str(json).unwrap();
        let res = work.exec();
        assert_eq!(res.view(), nalgebra::matrix![1586.0]);
    }

    #[test]
    fn it_deserializes_streamingly() {
        let file = std::fs::File::open("op.json").unwrap();
        let mut json_reader = JsonStreamReader::new(file);

        json_reader.begin_array().unwrap();
        let _work: PieceOfWork = json_reader.deserialize_next().unwrap();
    }

    #[test]
    fn it_works_streamingly() {
        let file = std::fs::File::open("op.json").unwrap();
        let mut json_reader = JsonStreamReader::new(file);

        json_reader.begin_array().unwrap();

        let res = PieceOfWork::exec_streamingly(&mut json_reader).unwrap();
        assert_eq!(res.view(), nalgebra::matrix![1586.0]);

        assert!(!json_reader.has_next().unwrap());

        json_reader.end_array().unwrap();
    }
}
