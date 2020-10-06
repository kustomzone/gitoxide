#![forbid(unsafe_code)]
// #![forbid(rust_2018_idioms)]

use std::ops::Range;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
struct Span {
    pub start: usize,
    pub end_inclusive: usize,
}

impl From<Span> for Range<usize> {
    fn from(Span { start, end_inclusive }: Span) -> Self {
        Range {
            start,
            end: end_inclusive + 1,
        }
    }
}

impl Span {
    fn to_range(&self) -> Range<usize> {
        self.clone().into()
    }
}

mod file {
    use crate::{spanned, Span};
    use bstr::{BStr, ByteSlice};

    enum Token {
        Section(spanned::Section),
        Entry(spanned::Entry),
        Comment(spanned::Comment),
    }

    pub struct File {
        buf: Vec<u8>,
        tokens: Vec<Token>, // but how do we get fast lookups and proper value lookup based on decoded values?
                            // On the fly is easier, otherwise we have to deal with a lookup cache of sorts and
                            // many more allocations up front (which might be worth it). Cow<'a, _> would bind to
                            // our buffer so the cache can't be in this type
    }

    impl File {
        pub(crate) fn bytes_at(&self, span: Span) -> &BStr {
            &self.buf[span.to_range()].as_bstr()
        }
    }
}

mod value {
    pub enum Color {
        Red,
        BrightRed,
        Ansi { r: u8, g: u8, c: u8 },
    }

    mod resolve {
        use bstr::BStr;
        use quick_error::quick_error;
        use std::path::PathBuf;

        quick_error! {
            #[derive(Debug)]
            pub enum Error {
                Tbd {
                    display("TBD")
                }
            }
        }
        pub fn path(_value: &BStr) -> Result<PathBuf, Error> {
            unimplemented!("path_resolve")
        }
    }
}

mod decode {
    use crate::{borrowed, value};
    use bstr::BStr;
    use quick_error::quick_error;
    use std::{borrow::Cow, path::PathBuf};

    quick_error! {
        #[derive(Debug)]
        pub enum Error {
            Tbd {
                display("let's see what can go wrong and how we do it")
            }
            NoValue {
                display("Entry has no value (TODO: much more error information)")
            }
        }
    }

    pub fn value(_input: &BStr) -> Result<Cow<'_, BStr>, Error> {
        unimplemented!("decode value from bstr")
    }

    impl<'a> borrowed::Entry<'a> {
        pub fn as_string(&self) -> Result<Cow<'a, BStr>, Error> {
            value(self.parent.bytes_at(self.value.ok_or_else(|| Error::NoValue)?)).map_err(Into::into)
        }
        pub fn as_int(&self) -> Result<i64, Error> {
            unimplemented!("as int")
        }
        pub fn as_bool(&self) -> Result<bool, Error> {
            unimplemented!("as bool")
        }
        pub fn as_path(&self) -> Result<PathBuf, Error> {
            unimplemented!("as bool")
        }
        pub fn as_color(&self) -> Result<value::Color, Error> {
            unimplemented!("as bool")
        }
    }
}

mod spanned {
    use crate::Span;
    // we parse leading and trailing whitespace into comments, avoiding the notion of whitespace.
    // This means we auto-trim whitespace otherwise, which I consider a feature
    pub(crate) type Comment = Span;

    pub(crate) struct Section {
        pub(crate) name: Span,
        pub(crate) sub_name: Option<Span>,
    }

    pub(crate) struct Entry {
        pub(crate) name: Span,
        pub(crate) value: Option<Span>,
    }
}

mod borrowed {
    use crate::{file::File, spanned, Span};

    pub struct Entry<'a> {
        pub(crate) parent: &'a File,
        section: spanned::Section,
        name: Span,
        pub(crate) value: Option<Span>,
    }

    struct Section<'a> {
        parent: &'a File,
        name: Span,
        sub_name: Option<Span>,
        entries: Vec<spanned::Entry>,
    }
}