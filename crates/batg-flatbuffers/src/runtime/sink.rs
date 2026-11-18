use std::os::raw::{c_int, c_void};
use std::ptr;

use serde::Serialize;
use serde::ser::{Impossible, SerializeSeq, SerializeStruct, SerializeTuple, Serializer};

use crate::error::FlatBufferError;

pub const SINK_VERSION: u32 = 1;

pub const TAG_NULL: u32 = 0;
pub const TAG_BOOL: u32 = 1;
pub const TAG_I8: u32 = 2;
pub const TAG_I16: u32 = 3;
pub const TAG_I32: u32 = 4;
pub const TAG_I64: u32 = 5;
pub const TAG_U8: u32 = 6;
pub const TAG_U16: u32 = 7;
pub const TAG_U32: u32 = 8;
pub const TAG_U64: u32 = 9;
pub const TAG_F32: u32 = 10;
pub const TAG_F64: u32 = 11;
pub const TAG_STR: u32 = 12;
pub const TAG_ENUM: u32 = 13;

pub const SCALAR: i64 = -1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Cell {
    pub name: *const u8,
    pub name_len: usize,
    pub text: *const u8,
    pub text_len: usize,
    pub index: i64,
    pub signed: i64,
    pub unsigned: u64,
    pub real: f64,
    pub tag: u32
}

pub type RowFn = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type FieldFn = unsafe extern "C" fn(*mut c_void, *const Cell) -> c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sink {
    pub version: u32,
    pub size: u32,
    pub userdata: *mut c_void,
    pub begin_row: Option<RowFn>,
    pub field: Option<FieldFn>,
    pub end_row: Option<RowFn>
}

pub struct SinkRef {
    userdata: *mut c_void,
    begin_row: RowFn,
    field: FieldFn,
    end_row: RowFn
}

pub fn size() -> u32 { u32::try_from(size_of::<Sink>()).unwrap_or(u32::MAX) }

impl Cell {
    #[inline]
    const fn new(name: &str, index: i64, tag: u32) -> Self {
        Self {
            name: name.as_ptr(),
            name_len: name.len(),
            text: ptr::null(),
            text_len: 0,
            index,
            signed: 0,
            unsigned: 0,
            real: 0.0,
            tag
        }
    }

    #[inline]
    const fn signed(name: &str, index: i64, tag: u32, value: i64) -> Self {
        Self {
            signed: value,
            ..Self::new(name, index, tag)
        }
    }

    #[inline]
    const fn unsigned(name: &str, index: i64, tag: u32, value: u64) -> Self {
        Self {
            unsigned: value,
            ..Self::new(name, index, tag)
        }
    }

    #[inline]
    const fn real(name: &str, index: i64, tag: u32, value: f64) -> Self {
        Self {
            real: value,
            ..Self::new(name, index, tag)
        }
    }

    #[inline]
    const fn text(name: &str, index: i64, tag: u32, value: &str) -> Self {
        Self {
            text: value.as_ptr(),
            text_len: value.len(),
            ..Self::new(name, index, tag)
        }
    }
}

impl SinkRef {
    pub unsafe fn new(sink: *const Sink) -> Result<Self, FlatBufferError> {
        if sink.is_null() {
            return Err(FlatBufferError::SinkHook("sink"));
        }

        let sink = unsafe { *sink };
        if sink.version != SINK_VERSION {
            return Err(FlatBufferError::SinkVersion(sink.version));
        }
        if sink.size != size() {
            return Err(FlatBufferError::SinkSize(sink.size));
        }

        Ok(Self {
            userdata: sink.userdata,
            begin_row: sink.begin_row.ok_or(FlatBufferError::SinkHook("begin_row"))?,
            field: sink.field.ok_or(FlatBufferError::SinkHook("field"))?,
            end_row: sink.end_row.ok_or(FlatBufferError::SinkHook("end_row"))?
        })
    }

    #[inline]
    fn check(label: &'static str, code: c_int) -> Result<(), FlatBufferError> {
        if code == 0 { Ok(()) } else { Err(FlatBufferError::SinkFailed(label, code)) }
    }

    #[inline]
    fn begin_row(&self) -> Result<(), FlatBufferError> {
        Self::check("begin_row", unsafe { (self.begin_row)(self.userdata) })
    }

    #[inline]
    fn end_row(&self) -> Result<(), FlatBufferError> {
        Self::check("end_row", unsafe { (self.end_row)(self.userdata) })
    }

    #[inline]
    fn emit(&self, cell: &Cell) -> Result<(), FlatBufferError> {
        Self::check("field", unsafe { (self.field)(self.userdata, cell) })
    }
}

pub fn visit_row<T>(value: &T, out: &SinkRef) -> Result<(), FlatBufferError>
where
    T: Serialize + ?Sized
{
    out.begin_row()?;
    value.serialize(Row { out })?;
    out.end_row()
}

pub fn visit_table<T>(value: &T, out: &SinkRef) -> Result<(), FlatBufferError>
where
    T: Serialize + ?Sized
{
    value.serialize(Table { out })
}

macro_rules! reject {
    ($($method:ident($type:ty)),* $(,)?) => {
        $(
            fn $method(self, _value: $type) -> Result<Self::Ok, Self::Error> {
                Err(FlatBufferError::Unsupported(stringify!($method)))
            }
        )*
    };
}

macro_rules! reject_scalars {
    () => {
        reject!(
            serialize_bool(bool),
            serialize_i8(i8),
            serialize_i16(i16),
            serialize_i32(i32),
            serialize_i64(i64),
            serialize_u8(u8),
            serialize_u16(u16),
            serialize_u32(u32),
            serialize_u64(u64),
            serialize_f32(f32),
            serialize_f64(f64),
            serialize_char(char),
            serialize_str(&str),
            serialize_bytes(&[u8]),
            serialize_unit_struct(&'static str)
        );

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Err(FlatBufferError::Unsupported("none"))
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Err(FlatBufferError::Unsupported("unit"))
        }

        fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize + ?Sized
        {
            Err(FlatBufferError::Unsupported("some"))
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str
        ) -> Result<Self::Ok, Self::Error> {
            Err(FlatBufferError::Unsupported("unit variant"))
        }

        fn serialize_newtype_struct<T>(
            self,
            _name: &'static str,
            _value: &T
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize + ?Sized
        {
            Err(FlatBufferError::Unsupported("newtype struct"))
        }

        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _value: &T
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize + ?Sized
        {
            Err(FlatBufferError::Unsupported("newtype variant"))
        }
    };
}

macro_rules! reject_compounds {
    () => {
        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Err(FlatBufferError::Unsupported("tuple struct"))
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _len: usize
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Err(FlatBufferError::Unsupported("tuple variant"))
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Err(FlatBufferError::Unsupported("map"))
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _len: usize
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Err(FlatBufferError::Unsupported("struct variant"))
        }
    };
}

macro_rules! reject_sequences {
    ($label:literal) => {
        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Err(FlatBufferError::Unsupported($label))
        }

        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Err(FlatBufferError::Unsupported($label))
        }
    };
}

macro_rules! sequence {
    ($target:ident) => {
        impl SerializeSeq for $target<'_> {
            type Error = FlatBufferError;
            type Ok = ();

            fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
            where
                T: Serialize + ?Sized
            {
                self.element(value)
            }

            fn end(self) -> Result<Self::Ok, Self::Error> { self.finish() }
        }

        impl SerializeTuple for $target<'_> {
            type Error = FlatBufferError;
            type Ok = ();

            fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
            where
                T: Serialize + ?Sized
            {
                self.element(value)
            }

            fn end(self) -> Result<Self::Ok, Self::Error> { self.finish() }
        }
    };
}

struct Table<'a> {
    out: &'a SinkRef
}

impl<'a> Serializer for Table<'a> {
    type Error = FlatBufferError;
    type Ok = ();
    type SerializeMap = Impossible<(), FlatBufferError>;
    type SerializeSeq = Impossible<(), FlatBufferError>;
    type SerializeStruct = Wrapper<'a>;
    type SerializeStructVariant = Impossible<(), FlatBufferError>;
    type SerializeTuple = Impossible<(), FlatBufferError>;
    type SerializeTupleStruct = Impossible<(), FlatBufferError>;
    type SerializeTupleVariant = Impossible<(), FlatBufferError>;

    reject_scalars!();

    reject_compounds!();

    reject_sequences!("root sequence");

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(Wrapper { out: self.out })
    }
}

struct Wrapper<'a> {
    out: &'a SinkRef
}

impl SerializeStruct for Wrapper<'_> {
    type Error = FlatBufferError;
    type Ok = ();

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized
    {
        value.serialize(List { out: self.out })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> { Ok(()) }
}

struct List<'a> {
    out: &'a SinkRef
}

impl<'a> Serializer for List<'a> {
    type Error = FlatBufferError;
    type Ok = ();
    type SerializeMap = Impossible<(), FlatBufferError>;
    type SerializeSeq = Elements<'a>;
    type SerializeStruct = Impossible<(), FlatBufferError>;
    type SerializeStructVariant = Impossible<(), FlatBufferError>;
    type SerializeTuple = Elements<'a>;
    type SerializeTupleStruct = Impossible<(), FlatBufferError>;
    type SerializeTupleVariant = Impossible<(), FlatBufferError>;

    reject_scalars!();

    reject_compounds!();

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(Elements { out: self.out })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(Elements { out: self.out })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(FlatBufferError::Unsupported("data list shape"))
    }
}

struct Elements<'a> {
    out: &'a SinkRef
}

impl Elements<'_> {
    #[inline]
    fn element<T>(&mut self, value: &T) -> Result<(), FlatBufferError>
    where
        T: Serialize + ?Sized
    {
        visit_row(value, self.out)
    }

    #[inline]
    const fn finish(self) -> Result<(), FlatBufferError> { Ok(()) }
}

sequence!(Elements);

struct Row<'a> {
    out: &'a SinkRef
}

impl<'a> Serializer for Row<'a> {
    type Error = FlatBufferError;
    type Ok = ();
    type SerializeMap = Impossible<(), FlatBufferError>;
    type SerializeSeq = Impossible<(), FlatBufferError>;
    type SerializeStruct = Fields<'a>;
    type SerializeStructVariant = Impossible<(), FlatBufferError>;
    type SerializeTuple = Impossible<(), FlatBufferError>;
    type SerializeTupleStruct = Impossible<(), FlatBufferError>;
    type SerializeTupleVariant = Impossible<(), FlatBufferError>;

    reject_scalars!();

    reject_compounds!();

    reject_sequences!("row sequence");

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(Fields::new(self.out))
    }
}

struct Fields<'a> {
    out: &'a SinkRef,
    prefix: Option<&'a str>,
    scratch: String
}

impl<'a> Fields<'a> {
    fn new(out: &'a SinkRef) -> Self {
        Self {
            out,
            prefix: None,
            scratch: String::new()
        }
    }

    fn nested(out: &'a SinkRef, prefix: &'a str) -> Self {
        Self {
            out,
            prefix: Some(prefix),
            scratch: String::new()
        }
    }

    fn qualify(&mut self, key: &'a str) -> &str {
        let Some(prefix) = self.prefix else {
            return key;
        };

        self.scratch.clear();
        self.scratch.push_str(prefix);
        self.scratch.push('.');
        self.scratch.push_str(key);
        &self.scratch
    }
}

impl SerializeStruct for Fields<'_> {
    type Error = FlatBufferError;
    type Ok = ();

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized
    {
        let out = self.out;
        let name = self.qualify(key);

        value.serialize(Value { out, name, index: SCALAR })
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        let out = self.out;
        let name = self.qualify(key);

        out.emit(&Cell::new(name, SCALAR, TAG_NULL))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> { Ok(()) }
}

struct Value<'a> {
    out: &'a SinkRef,
    name: &'a str,
    index: i64
}

impl<'a> Value<'a> {
    fn items(self) -> Items<'a> {
        Items {
            out: self.out,
            name: self.name,
            index: 0
        }
    }
}

macro_rules! emit {
    ($constructor:ident, $cast:ty, $($method:ident($type:ty) => $tag:expr),* $(,)?) => {
        $(
            fn $method(self, value: $type) -> Result<Self::Ok, Self::Error> {
                let cell = Cell::$constructor(self.name, self.index, $tag, <$cast>::from(value));
                self.out.emit(&cell)
            }
        )*
    };
}

impl<'a> Serializer for Value<'a> {
    type Error = FlatBufferError;
    type Ok = ();
    type SerializeMap = Impossible<(), FlatBufferError>;
    type SerializeSeq = Items<'a>;
    type SerializeStruct = Fields<'a>;
    type SerializeStructVariant = Impossible<(), FlatBufferError>;
    type SerializeTuple = Items<'a>;
    type SerializeTupleStruct = Impossible<(), FlatBufferError>;
    type SerializeTupleVariant = Impossible<(), FlatBufferError>;

    reject_compounds!();

    emit!(
        signed, i64,
        serialize_i8(i8) => TAG_I8,
        serialize_i16(i16) => TAG_I16,
        serialize_i32(i32) => TAG_I32,
        serialize_i64(i64) => TAG_I64
    );

    emit!(
        unsigned, u64,
        serialize_bool(bool) => TAG_BOOL,
        serialize_u8(u8) => TAG_U8,
        serialize_u16(u16) => TAG_U16,
        serialize_u32(u32) => TAG_U32,
        serialize_u64(u64) => TAG_U64
    );

    emit!(
        real, f64,
        serialize_f32(f32) => TAG_F32,
        serialize_f64(f64) => TAG_F64
    );

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        let mut buffer = [0u8; 4];
        let cell = Cell::text(self.name, self.index, TAG_STR, value.encode_utf8(&mut buffer));

        self.out.emit(&cell)
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        let cell = Cell::text(self.name, self.index, TAG_STR, value);

        self.out.emit(&cell)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        let mut items = self.items();
        for byte in value {
            items.element(byte)?;
        }

        items.finish()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.out.emit(&Cell::new(self.name, self.index, TAG_NULL))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> { self.serialize_none() }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized
    {
        value.serialize(self)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str
    ) -> Result<Self::Ok, Self::Error> {
        let cell = Cell::text(self.name, self.index, TAG_ENUM, variant);

        self.out.emit(&cell)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        value: &T
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self.items())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(self.items())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(Fields::nested(self.out, self.name))
    }
}

struct Items<'a> {
    out: &'a SinkRef,
    name: &'a str,
    index: i64
}

impl Items<'_> {
    #[inline]
    fn element<T>(&mut self, value: &T) -> Result<(), FlatBufferError>
    where
        T: Serialize + ?Sized
    {
        value.serialize(Value {
            out: self.out,
            name: self.name,
            index: self.index
        })?;
        self.index += 1;

        Ok(())
    }

    fn finish(self) -> Result<(), FlatBufferError> {
        if self.index != 0 {
            return Ok(());
        }

        self.out.emit(&Cell::new(self.name, 0, TAG_NULL))
    }
}

sequence!(Items);
