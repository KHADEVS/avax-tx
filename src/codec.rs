//! The binary encoding avalanchego's linear codec uses for P-Chain
//! transactions and blocks.
//!
//! Integers are big-endian. A slice is its length as a `u32`, then its
//! elements; a fixed-size array has no length. A string is its length as a
//! `u16`, then its bytes. A value of an interface type is the `u32` type ID
//! its concrete type was registered with, then the value. Embedded structs
//! are written in place. A whole message starts with the codec version, a
//! `u16` that is always 0, and must not be followed by anything.

use std::fmt;

/// Why some bytes could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The input ended before the value did.
    UnexpectedEnd,
    /// The codec version is not 0.
    UnsupportedVersion(u16),
    /// A type ID that the interface at this position does not accept.
    UnknownType {
        /// The interface being decoded.
        interface: &'static str,
        /// The type ID found.
        type_id: u32,
    },
    /// A slice claims more elements than the input could hold.
    LengthTooLarge(u32),
    /// Bytes are left after the value.
    TrailingBytes(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnexpectedEnd => write!(f, "unexpected end of input"),
            Error::UnsupportedVersion(v) => write!(f, "unsupported codec version {v}"),
            Error::UnknownType { interface, type_id } => {
                write!(f, "type ID {type_id} is not a known {interface}")
            }
            Error::LengthTooLarge(n) => write!(f, "slice length {n} exceeds the input"),
            Error::TrailingBytes(n) => write!(f, "{n} trailing bytes"),
        }
    }
}

impl std::error::Error for Error {}

/// Reads values from a byte slice.
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Starts reading at the beginning of `data`.
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    /// The number of bytes not read yet.
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Takes the next `n` bytes.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if n > self.remaining() {
            return Err(Error::UnexpectedEnd);
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    /// Fails unless every byte has been read.
    pub fn finish(&self) -> Result<(), Error> {
        match self.remaining() {
            0 => Ok(()),
            n => Err(Error::TrailingBytes(n)),
        }
    }
}

/// Collects encoded bytes.
#[derive(Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    /// An empty writer.
    pub fn new() -> Self {
        Writer::default()
    }

    /// Appends raw bytes.
    pub fn put(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// The bytes written so far.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

/// A value with a codec encoding.
pub trait Codec: Sized {
    /// Appends the encoding of `self`.
    fn encode(&self, w: &mut Writer);
    /// Reads one value.
    fn decode(r: &mut Reader<'_>) -> Result<Self, Error>;
}

macro_rules! int_codec {
    ($($t:ty),*) => {$(
        impl Codec for $t {
            fn encode(&self, w: &mut Writer) {
                w.put(&self.to_be_bytes());
            }
            fn decode(r: &mut Reader<'_>) -> Result<Self, Error> {
                let b = r.take(std::mem::size_of::<$t>())?;
                Ok(<$t>::from_be_bytes(b.try_into().expect("length checked")))
            }
        }
    )*};
}
int_codec!(u8, u16, u32, u64);

impl<const N: usize> Codec for [u8; N] {
    fn encode(&self, w: &mut Writer) {
        w.put(self);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(r.take(N)?.try_into().expect("length checked"))
    }
}

/// Slices, including `Vec<u8>` for Go's `[]byte`: a `u32` length, then the
/// elements. Every element takes at least one byte, so a length above the
/// bytes left is rejected before anything is allocated.
impl<T: Codec> Codec for Vec<T> {
    fn encode(&self, w: &mut Writer) {
        let n = u32::try_from(self.len()).expect("slice longer than u32::MAX");
        n.encode(w);
        for item in self {
            item.encode(w);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::decode(r)?;
        if n as usize > r.remaining() {
            return Err(Error::LengthTooLarge(n));
        }
        let mut out = Vec::with_capacity(n as usize);
        for _ in 0..n {
            out.push(T::decode(r)?);
        }
        Ok(out)
    }
}

/// A Go `string`: a `u16` length, then bytes. Go does not require them to be
/// UTF-8, so they are kept as bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Str(pub Vec<u8>);

impl Codec for Str {
    fn encode(&self, w: &mut Writer) {
        let n = u16::try_from(self.0.len()).expect("string longer than u16::MAX");
        n.encode(w);
        w.put(&self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u16::decode(r)?;
        Ok(Str(r.take(n as usize)?.to_vec()))
    }
}

/// Decodes a whole message: the codec version, then one value, then nothing.
pub fn from_bytes<T: Codec>(bytes: &[u8]) -> Result<T, Error> {
    let mut r = Reader::new(bytes);
    let version = u16::decode(&mut r)?;
    if version != 0 {
        return Err(Error::UnsupportedVersion(version));
    }
    let value = T::decode(&mut r)?;
    r.finish()?;
    Ok(value)
}

/// Encodes a whole message: codec version 0, then the value.
pub fn to_bytes<T: Codec>(value: &T) -> Vec<u8> {
    let mut w = Writer::new();
    0u16.encode(&mut w);
    value.encode(&mut w);
    w.into_bytes()
}

/// Declares a struct whose encoding is its fields, in order.
macro_rules! codec_struct {
    ($(#[$m:meta])* pub struct $name:ident { $($(#[$fm:meta])* pub $f:ident : $t:ty),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name { $($(#[$fm])* pub $f: $t),* }

        impl $crate::codec::Codec for $name {
            #[allow(unused_variables)]
            fn encode(&self, w: &mut $crate::codec::Writer) {
                $($crate::codec::Codec::encode(&self.$f, w);)*
            }
            #[allow(unused_variables)]
            fn decode(r: &mut $crate::codec::Reader<'_>) -> Result<Self, $crate::codec::Error> {
                Ok(Self { $($f: $crate::codec::Codec::decode(r)?),* })
            }
        }
    };
}

/// Declares an interface: an enum whose encoding is the variant's type ID,
/// then the variant's value.
macro_rules! codec_enum {
    ($(#[$m:meta])* pub enum $name:ident { $($(#[$vm:meta])* $var:ident($t:ty) = $id:literal),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum $name { $($(#[$vm])* $var($t)),* }

        impl $name {
            /// The codec type ID of this variant.
            pub fn type_id(&self) -> u32 {
                match self { $($name::$var(_) => $id),* }
            }
        }

        impl $crate::codec::Codec for $name {
            fn encode(&self, w: &mut $crate::codec::Writer) {
                $crate::codec::Codec::encode(&self.type_id(), w);
                match self { $($name::$var(v) => $crate::codec::Codec::encode(v, w)),* }
            }
            fn decode(r: &mut $crate::codec::Reader<'_>) -> Result<Self, $crate::codec::Error> {
                let type_id = <u32 as $crate::codec::Codec>::decode(r)?;
                match type_id {
                    $($id => Ok($name::$var($crate::codec::Codec::decode(r)?)),)*
                    _ => Err($crate::codec::Error::UnknownType { interface: stringify!($name), type_id }),
                }
            }
        }
    };
}

pub(crate) use codec_enum;
pub(crate) use codec_struct;
