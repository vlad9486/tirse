#![cfg(not(feature = "use_std"))]

use serde::Serialize;
use serde_derive::Serialize;

use tirse::DefaultBinarySerializer;
use tirse::DisplayCollector;
use tirse::Write;

use core::fmt;

const SMALL_BUFFER_SIZE: usize = 8;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct SmallBuffer {
    raw: [u8; SMALL_BUFFER_SIZE],
    position: usize,
}

#[derive(Debug, Clone)]
pub enum Error {
    SizeLimit,
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            &SizeLimit => write!(fmt, "size limit reached"),
        }
    }
}

impl Write for SmallBuffer {
    type Error = Error;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let position = self.position + bytes.len();
        if position > SMALL_BUFFER_SIZE {
            Err(Error::SizeLimit)
        } else {
            self.raw.copy_from_slice(bytes);
            self.position = position;
            Ok(())
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FakeDisplayCollector;

impl fmt::Display for FakeDisplayCollector {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "some internal error")
    }
}

impl DisplayCollector for FakeDisplayCollector {
    fn display<T>(msg: &T) -> Self
    where
        T: ?Sized + fmt::Display,
    {
        let _ = msg;
        FakeDisplayCollector
    }
}


#[test]
fn test() {
    #[derive(Serialize)]
    pub struct Point3d {
        x: f32,
        y: f32,
        z: f32,
    }

    let _p = Point3d {
        x: 0.4,
        y: 7.5,
        z: 0.0,
    };
    let w = SmallBuffer::default();

    let serializer = DefaultBinarySerializer::<SmallBuffer, FakeDisplayCollector>::new(w);

    let buffer = Serialize::serialize(&5u64, serializer)
        .map(DefaultBinarySerializer::consume)
        .unwrap();

    assert_eq!(
        buffer,
        SmallBuffer {
            raw: [5, 0, 0, 0, 0, 0, 0, 0],
            position: 8,
        }
    );
    println!("{:?}", buffer)
}
