use std::num::{ParseFloatError, ParseIntError};

use futures::AsyncRead;
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

    pub async fn deserialize<R: AsyncRead + Unpin>(
        reader: &mut JsonStreamReader<R>,
    ) -> Result<Self> {
        reader.begin_object().await?;

        let mut d = None;
        let mut n = None;
        for _ in 0..2 {
            match reader.next_name().await? {
                "d" => {
                    reader.begin_array().await?;
                    let mut d_vec = HeaplessVec::new();
                    while reader.has_next().await? {
                        d_vec.push(reader.next_number().await??).unwrap();
                    }
                    reader.end_array().await?;
                    d = Some(d_vec);
                }
                "n" => n = Some(reader.next_number().await??),
                _ => return Err(Error::Json("Unexpected field name in MatrixBuf")),
            }
        }

        reader.end_object().await?;
        let (Some(d), Some(n)) = (d, n) else {
            return Err(Error::Json(
                "Not all fields of MatrixBuf were given, or too many fields",
            ));
        };
        Ok(Self { d, n })
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
    ) -> Result<Self> {
        reader.begin_object().await?;
        let mut code = None;
        let mut rhs = None;

        for _ in 0..2 {
            match reader.next_name().await? {
                "code" => {
                    code = Some(reader.next_string().await?);
                }
                "rhs" => rhs = Some(MatrixBuf::deserialize(reader).await?),
                _ => return Err(Error::Json("Unexpected field name in Operation object")),
            }
        }

        reader.end_object().await?;

        let (Some(code), Some(rhs)) = (code, rhs) else {
            return Err(Error::Json(
                "Not all fields of Operation were given, or too many fields",
            ));
        };
        match code.as_str() {
            "dot" => Ok(Self::Dot { rhs }),
            _ => return Err(Error::Json("Unexpected Operation code")),
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
    pub async fn exec_streamingly<R: AsyncRead + Unpin>(
        reader: &mut JsonStreamReader<R>,
    ) -> Result<MatrixBuf> {
        reader.begin_object().await?;

        // First, we need the `lhs` object
        let "lhs" = reader.next_name().await? else {
            return Err(Error::Json("lhs"));
        };
        let lhs: MatrixBuf = MatrixBuf::deserialize(reader).await?;

        // Then, we read the `op` array element-by-element
        // We execute operations as they come in
        let "op" = reader.next_name().await? else {
            return Err(Error::Json("op"));
        };

        reader.begin_array().await?;

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

#[derive(Debug)]
pub enum Error {
    Json(&'static str),
    Struson(struson::reader::ReaderError),
    ParseFloat(ParseFloatError),
    ParseInt(ParseIntError),
    Io(std::io::Error),
}

impl From<struson::reader::ReaderError> for Error {
    fn from(e: struson::reader::ReaderError) -> Self {
        Self::Struson(e)
    }
}

impl From<ParseFloatError> for Error {
    fn from(e: ParseFloatError) -> Self {
        Self::ParseFloat(e)
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

mod strompychan {
    use std::sync::Arc;

    use futures::lock::Mutex;
    use pychan::reader::PyBytesReader;
    use pyo3::pyclass;
    use struson::reader::{JsonReader, JsonStreamReader};

    use crate::{MatrixBuf, PieceOfWork, Result};

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

        pub async fn next(&mut self) -> Result<MatrixBuf> {
            let mut inner = self.inner.lock().await;
            if !inner.in_array {
                println!("Begin array");
                inner.reader.begin_array().await.unwrap();
                inner.in_array = true;
            }
            PieceOfWork::exec_streamingly(&mut inner.reader).await
        }
    }
}

mod py {
    use std::io;

    use pychan::py_bytes::PyBytesSender;

    use futures::SinkExt;
    use pyo3::{prelude::*, types::PyBytes};

    use crate::{strompychan::StrompyJsonReader, Error, MatrixBuf, PieceOfWork};

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
        let (tx, rx) = pychan::py_bytes::channel();
        let reader = rx.into_reader();
        let reader = StrompyJsonReader::new(reader);

        (tx, reader)
    }

    #[pyfunction]
    async fn feed_bytes(mut writer: PyBytesSender, bytes: Py<PyBytes>) -> PyResult<()> {
        writer.send(bytes).await.unwrap();
        Ok(())
    }

    #[pyfunction]
    async fn poll_next(mut reader: StrompyJsonReader) -> PyResult<Option<Vec<Vec<f64>>>> {
        match reader.next().await {
            Ok(r) => Ok(Some(r.into())),
            Err(Error::Struson(struson::reader::ReaderError::IoError { error, .. }))
                if error.kind() == io::ErrorKind::BrokenPipe =>
            {
                Ok(None)
            }
            e @ Err(_) => {
                // TODO return err instead of panicking here
                e.unwrap();
                unreachable!()
            }
        }
    }

    #[pymodule]
    fn strompy(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(exec, m)?)?;
        m.add_function(wrap_pyfunction!(channel, m)?)?;
        m.add_function(wrap_pyfunction!(feed_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(poll_next, m)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use futures::{io::Cursor, AsyncReadExt, AsyncWriteExt};

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

    #[tokio::test]
    async fn cursor_test() {
        let (mut reader, mut writer) =
            futures::AsyncReadExt::split(Cursor::new(Vec::with_capacity(1024)));

        let read_fut = async move {
            let buf = &mut [0u8; 100];
            loop {
                let n = reader.read(buf).await.unwrap();
                println!("Read {n} bytes: {:?}", &buf[..n]);
            }
        };

        let write_fut = async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            writer.write(&[1, 2, 3, 4, 5, 6]).await.unwrap();
            drop(writer);
        };

        tokio::select! {
            _ = write_fut => {println!("Write won!")}
            _ = read_fut => {println!("read won!")}
        };
    }
}
