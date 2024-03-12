use std::num::{ParseFloatError, ParseIntError};

use futures::AsyncRead;
use heapless::Vec as HeaplessVec;
use nalgebra as na;
use struson::reader::{JsonReader, JsonStreamReader};

mod chan;
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

pub(crate) mod channel {
    use std::sync::Arc;

    use crate::chan::Channel;

    use super::*;
    use futures::{
        io::Cursor,
        lock::Mutex,
        AsyncWriteExt,
        // AsyncReadExt, AsyncWriteExt,
    };
    use pyo3::prelude::*;
    use struson::reader::JsonStreamReader;

    #[pyclass]
    #[derive(Clone)]
    pub struct StrompyWriter {
        // inner: Arc<Mutex<StrompyWriterInner>>,
        writer: Channel,
    }

    struct StrompyWriterInner {
        // writer: WriteHalf<Cursor<Vec<u8>>>,
        writer: Channel,
    }

    impl Drop for StrompyWriterInner {
        fn drop(&mut self) {
            println!("Dropped StrompyWriterInner!");
        }
    }

    impl StrompyWriter {
        pub async fn feed(&mut self, bytes: &[u8]) -> Result<()> {
            println!("Feeding writer:\n{}", String::from_utf8_lossy(&bytes));
            // let mut writer = self.writer.lock().await;
            // writer.write_all(bytes).await?;
            // let mut inner = self.inner.lock().await;
            // inner.writer.write(bytes).await?;
            self.writer.write_all(bytes).await?;
            Ok(())
        }
    }

    #[pyclass]
    #[derive(Clone)]
    pub struct StrompyJsonReader {
        // reader: Arc<Mutex<tokio::io::DuplexStream>>,
        inner: Arc<Mutex<StrompyJsonReaderInner>>,
    }

    struct StrompyJsonReaderInner {
        in_array: bool,
        // reader: JsonStreamReader<ReadHalf<Cursor<Vec<u8>>>>,
        reader: JsonStreamReader<Channel>,
    }

    impl Drop for StrompyJsonReaderInner {
        fn drop(&mut self) {
            println!("Dropped JSON Reader Inner");
        }
    }

    impl StrompyJsonReader {
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

    pub fn channel() -> (StrompyJsonReader, StrompyWriter) {
        // let (reader, writer) = Cursor::new(Vec::with_capacity(1024)).split();
        let chan = Channel::new();

        let reader = JsonStreamReader::new(chan.clone());
        let inner = StrompyJsonReaderInner {
            in_array: false,
            reader,
        };
        let inner = Arc::new(Mutex::new(inner));
        let reader = StrompyJsonReader { inner };

        // let inner = StrompyWriterInner { writer: chan };
        // let inner = Arc::new(Mutex::new(inner));
        let writer = StrompyWriter { writer: chan };

        (reader, writer)
    }
}

mod py {
    use std::{future::Future, pin::pin, task::Context};

    use futures::future::poll_fn;
    use pyo3::{prelude::*, types::PyBytes};

    use crate::{
        channel::{StrompyJsonReader, StrompyWriter},
        MatrixBuf, PieceOfWork,
    };

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
    fn channel<'py>(_py: Python<'py>) -> PyResult<(StrompyJsonReader, StrompyWriter)> {
        Ok(crate::channel::channel())
    }

    /*
    #[pyfunction]
    fn feed_bytes<'py>(
        py: Python<'py>,
        mut writer: StrompyWriter,
        read_coro: &PyAny,
    ) -> PyResult<&'py PyAny> {
        let read_fut = into_future(read_coro)?;

        future_into_py(py, async move {
            let bytes = read_fut.await.unwrap();
            let bytes: Vec<u8> = Python::with_gil(|py| {
                let bytes: &PyBytes = bytes.extract(py).unwrap();

                // This probably copies everything, and therefore defeats the purpose
                bytes.extract().unwrap()
            });

            writer.feed(&bytes).await.unwrap();

            Ok(())
        })
    }

    #[pyfunction]
    fn poll_next<'py>(py: Python<'py>, mut reader: StrompyJsonReader) -> PyResult<&'py PyAny> {
        future_into_py(py, async move {
            println!("Polling");
            let m = reader.next().await.unwrap();
            let res: Vec<Vec<f64>> = m.into();
            Ok(res)
        })
    }
    */

    #[pyfunction]
    async fn feed_bytes<'py>(mut writer: StrompyWriter, bytes: Py<PyBytes>) -> PyResult<()> {
        println!("{}:{}:{:?}", file!(), line!(), std::thread::current().id());
        poll_fn(|cx| {
            let waker = cx.waker();
            Python::with_gil(|py| {
                let bytes = bytes.as_bytes(py);
                let fut = writer.feed(bytes);
                let fut = pin!(fut);
                py.allow_threads(|| {
                    println!("{}:{}:{:?}", file!(), line!(), std::thread::current().id());
                    let p = fut.poll(&mut Context::from_waker(waker));
                    p
                })
            })
        })
        .await
        .unwrap();

        Ok(())
    }

    #[pyfunction]
    async fn poll_next(mut reader: StrompyJsonReader) -> PyResult<Vec<Vec<f64>>> {
        let res: MatrixBuf = poll_fn(|cx| {
            println!("{}:{}:{:?}", file!(), line!(), std::thread::current().id());
            let waker = cx.waker();
            Python::with_gil(|py| {
                let fut = reader.next();
                let fut = pin!(fut);
                py.allow_threads(|| {
                    let p = fut.poll(&mut Context::from_waker(waker));
                    p
                })
            })
        })
        .await
        .unwrap();

        dbg!(&res);

        Ok(res.into())
    }

    #[pymodule]
    fn strompy(_py: Python, m: &PyModule) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(exec, m)?)?;
        m.add_function(wrap_pyfunction!(channel, m)?)?;
        m.add_function(wrap_pyfunction!(feed_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(poll_next, m)?)?;
        m.add_class::<StrompyWriter>()?;
        m.add_class::<StrompyJsonReader>()?;
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
