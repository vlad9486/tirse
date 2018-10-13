use serde::{ser, de};
use core::fmt;

pub trait DisplayCollector {
    fn display<T>(msg: &T) -> Self
    where
        T: ?Sized + fmt::Display;
}

#[derive(Debug)]
pub enum ErrorAdapter<E, D>
where
    D: DisplayCollector,
{
    Inner(E),
    Other(D),
}

impl<E, D> ser::Error for ErrorAdapter<E, D>
where
    D: DisplayCollector + fmt::Display + fmt::Debug,
    E: fmt::Display + fmt::Debug,
{
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        use self::ErrorAdapter::*;

        Other(D::display(&msg))
    }
}

impl<E, D> de::Error for ErrorAdapter<E, D>
where
    D: DisplayCollector + fmt::Display + fmt::Debug,
    E: fmt::Display + fmt::Debug,
{
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        use self::ErrorAdapter::*;

        Other(D::display(&msg))
    }
}

impl<E, D> fmt::Display for ErrorAdapter<E, D>
where
    D: DisplayCollector + fmt::Display,
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ErrorAdapter::*;

        match self {
            &Inner(ref e) => write!(f, "{}", e),
            &Other(ref d) => write!(f, "{}", d),
        }
    }
}

#[cfg(feature = "use_std")]
mod std {
    use std::{fmt, error, string};
    use super::{ErrorAdapter, DisplayCollector};

    impl<E, D> error::Error for ErrorAdapter<E, D>
    where
        D: DisplayCollector + fmt::Display + fmt::Debug,
        E: fmt::Display + fmt::Debug,
    {}

    impl DisplayCollector for string::String {
        fn display<T>(msg: &T) -> Self
        where
            T: ?Sized + fmt::Display,
        {
            format!("{}", msg)
        }
    }
}
