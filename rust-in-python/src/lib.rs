use error::StrompyError;
use futures::AsyncRead;
use heapless::Vec as HeaplessVec;
use nalgebra as na;
use pyo3::{types::PyList, Py, Python};
use struson::reader::{JsonReader, JsonStreamReader};

mod error;

type StrompyResult<T> = core::result::Result<T, StrompyError>;

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

    pub async fn deserialize<R: AsyncRead + Unpin>(
        reader: &mut JsonStreamReader<R>,
    ) -> StrompyResult<Self> {
        reader.begin_object().await?;

        // First, read in the data
        let "d" = reader.next_name().await? else {
            return Err(StrompyError::Json(
                r#"Unexpected key encountered, expected "d""#,
            ));
        };
        reader.begin_array().await?;
        let mut d = HeaplessVec::new();
        while reader.has_next().await? {
            d.push(reader.next_number().await??).unwrap();
        }
        reader.end_array().await?;

        // Then, read the number of columns
        let "n" = reader.next_name().await? else {
            return Err(StrompyError::Json(
                r#"Unexpected key encountered, expected "n""#,
            ));
        };
        let n = reader.next_number().await??;

        reader.end_object().await?;

        Ok(Self { d, n })
    }
}

impl std::iter::IntoIterator for MatrixBuf {
    type Item = Py<PyList>;

    type IntoIter = MatrixBufIter;

    fn into_iter(self) -> Self::IntoIter {
        MatrixBufIter { buf: self, i: 0 }
    }
}

pub struct MatrixBufIter {
    buf: MatrixBuf,
    i: usize,
}

impl std::iter::Iterator for MatrixBufIter {
    type Item = Py<PyList>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.buf.n {
            None
        } else {
            let items = self.buf.d.iter().skip(self.i * self.buf.n).take(self.buf.n);
            let item: Py<PyList> = Python::with_gil(|py| PyList::new_bound(py, items).unbind());
            self.i += 1;
            Some(item)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.buf.n, Some(self.buf.n))
    }
}

impl ExactSizeIterator for MatrixBufIter {
    fn len(&self) -> usize {
        self.buf.n
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

    pub async fn deserialize<R: AsyncRead + Unpin>(
        reader: &mut JsonStreamReader<R>,
    ) -> StrompyResult<Self> {
        // Reads the rhs field as a MatrixBuf
        async fn read_rhs<R: AsyncRead + Unpin>(
            reader: &mut JsonStreamReader<R>,
        ) -> StrompyResult<MatrixBuf> {
            let "rhs" = reader.next_name().await? else {
                return Err(StrompyError::Json(
                    r#"Unexpected key encountered, expected "rhs""#,
                ));
            };

            let rhs = MatrixBuf::deserialize(reader).await?;
            Ok(rhs)
        }

        reader.begin_object().await?;

        // Read op code
        let "code" = reader.next_name().await? else {
            return Err(StrompyError::Json(
                r#"Unexpected key encountered, expected "code""#,
            ));
        };
        let code = reader.next_str().await?;

        // Depending on op code, read further data
        let op = match code {
            "dot" => Self::Dot {
                rhs: read_rhs(reader).await?,
            },
            _ => return Err(StrompyError::Json("Unexpected Operation code")),
        };

        reader.end_object().await?;

        Ok(op)
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
    pub async fn exec_streamingly<R: AsyncRead + Unpin>(
        reader: &mut JsonStreamReader<R>,
    ) -> StrompyResult<MatrixBuf> {
        reader.begin_object().await?;

        // First, we need the `lhs` object
        let "lhs" = reader.next_name().await? else {
            return Err(StrompyError::Json(
                r#"Unexpected key encountered, expected "lhs""#,
            ));
        };
        let lhs: MatrixBuf = MatrixBuf::deserialize(reader).await?;

        // Then, we read the `op` array element-by-element
        let "op" = reader.next_name().await? else {
            return Err(StrompyError::Json(
                r#"Unexpected key encountered, expected "op""#,
            ));
        };

        reader.begin_array().await?;

        // We execute operations as they come in
        let mut res = lhs;
        while reader.has_next().await? {
            let op: Operation = Operation::deserialize(reader).await?;
            res = op.eval(res);
        }

        reader.end_array().await?;

        reader.end_object().await?;

        Ok(res)
    }
}

mod strompychan {
    use std::sync::Arc;

    use futures::lock::Mutex;
    use pychan::reader::PyBytesReader;
    use pyo3::{pyclass, pymethods, types::PyList, PyResult, Python};
    use struson::reader::{JsonReader, JsonStreamReader};

    use crate::{MatrixBuf, PieceOfWork, StrompyResult};

    struct StrompyJsonReaderInner {
        reader: JsonStreamReader<PyBytesReader>,
        in_array: bool,
    }

    #[pyclass]
    #[derive(Clone)]
    pub struct StrompyJsonReader {
        inner: Arc<Mutex<StrompyJsonReaderInner>>,
    }

    impl StrompyJsonReader {
        pub fn new(reader: PyBytesReader) -> Self {
            let reader = JsonStreamReader::new(reader);
            let inner = StrompyJsonReaderInner {
                reader,
                in_array: false,
            };

            Self {
                inner: Arc::new(Mutex::new(inner)),
            }
        }

        pub async fn next(&mut self) -> StrompyResult<Option<MatrixBuf>> {
            let mut inner = self.inner.lock().await;
            if !inner.in_array {
                inner.reader.begin_array().await.unwrap();
                inner.in_array = true;
            }
            if inner.reader.has_next().await? {
                let next = PieceOfWork::exec_streamingly(&mut inner.reader).await?;
                Ok(Some(next))
            } else {
                Ok(None)
            }
        }
    }

    #[pymethods]
    impl StrompyJsonReader {
        #[pyo3(name = "next")]
        async fn next_py(&mut self) -> PyResult<Option<pyo3::Py<PyList>>> {
            let next = self
                .next()
                .await?
                .map(|m| Python::with_gil(|py| PyList::new_bound(py, m).unbind()));
            Ok(next)
        }
    }
}

mod py {
    use pychan::py_bytes::PyBytesSender;

    use futures::SinkExt;
    use pyo3::{prelude::*, types::PyBytes};

    use crate::{strompychan::StrompyJsonReader, MatrixBuf, PieceOfWork};

    impl From<MatrixBuf> for Vec<Vec<f64>> {
        fn from(MatrixBuf { d, n }: MatrixBuf) -> Self {
            d.chunks_exact(n).into_iter().map(|c| c.to_vec()).collect()
        }
    }

    #[pyfunction]
    fn exec(json_bytes: &[u8]) -> PyResult<Vec<Vec<Vec<f64>>>> {
        let work: Vec<PieceOfWork> = serde_json::from_reader(json_bytes).unwrap();

        Ok(work.into_iter().map(|p| p.exec().into()).collect())
    }

    #[pyfunction]
    fn channel() -> (PyBytesSender, StrompyJsonReader) {
        let (tx, rx) = pychan::py_bytes::channel(16);
        let reader = rx.into_reader();
        let reader = StrompyJsonReader::new(reader);

        (tx, reader)
    }

    #[pyfunction]
    async fn feed_bytes(mut writer: PyBytesSender, bytes: Py<PyBytes>) -> PyResult<()> {
        writer.send(bytes).await?;
        Ok(())
    }

    #[pymodule]
    fn strompy(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(exec, m)?)?;
        m.add_function(wrap_pyfunction!(channel, m)?)?;
        m.add_class::<StrompyJsonReader>()?;
        m.add_function(wrap_pyfunction!(feed_bytes, m)?)?;
        Ok(())
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

    #[tokio::test]
    async fn it_works_streamingly() {
        use tokio_util::compat::TokioAsyncReadCompatExt;
        let file = tokio::fs::File::open("op.json").await.unwrap().compat();
        let mut json_reader = JsonStreamReader::new(file);

        json_reader.begin_array().await.unwrap();

        let res = PieceOfWork::exec_streamingly(&mut json_reader)
            .await
            .unwrap();
        assert_eq!(res.view(), nalgebra::matrix![1586.0]);

        assert!(!json_reader.has_next().await.unwrap());

        json_reader.end_array().await.unwrap();
    }
}
