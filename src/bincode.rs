// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Support for Bincode integration. Enable this with the `bincode` feature.

use crate::{Compact, LazyCompact, SmartString, SmartStringMode, MAX_INLINE};
use std::ops::Deref;

use bincode::{
    de::Decoder,
    enc::Encoder,
    error::{DecodeError, EncodeError},
    impl_borrow_decode, Decode, Encode,
};

impl<T: SmartStringMode> Encode for SmartString<T> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.as_bytes().encode(encoder)
    }
}

impl<T: SmartStringMode> Decode for SmartString<T> {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = <Vec<u8> as Decode>::decode(decoder)?;
        let string = String::from_utf8(bytes).map_err(|e| DecodeError::Utf8 {
            inner: e.utf8_error(),
        })?;
        Ok(if string.len() > MAX_INLINE {
            Self::from_boxed(string.into())
        } else {
            Self::from_inline(string.deref().into())
        })
    }
}

impl_borrow_decode!(SmartString<Compact>);
impl_borrow_decode!(SmartString<LazyCompact>);

#[cfg(test)]
mod test {
    use crate::{Compact, LazyCompact, SmartString};

    #[test]
    fn test_bincode_compact() {
        let mut buf: [u8; 64] = [0; 64];
        let short_str = "Hello world";
        let long_str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit";

        let config = bincode::config::standard();
        let smartstring = SmartString::<Compact>::from(short_str);
        let len = bincode::encode_into_slice(smartstring, &mut buf, config).unwrap();
        let smartstring: SmartString<Compact> =
            bincode::decode_from_slice(&buf[..len], config).unwrap().0;
        assert_eq!(smartstring, short_str);

        let smartstring = SmartString::<Compact>::from(long_str);
        let len = bincode::encode_into_slice(smartstring, &mut buf, config).unwrap();
        let smartstring: SmartString<Compact> =
            bincode::decode_from_slice(&buf[..len], config).unwrap().0;
        assert_eq!(smartstring, long_str);
    }

    #[test]
    fn test_bincode_lazy_compact() {
        let mut buf: [u8; 64] = [0; 64];
        let short_str = "Hello world";
        let long_str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit";

        let config = bincode::config::standard();
        let smartstring = SmartString::<LazyCompact>::from(short_str);
        let len = bincode::encode_into_slice(smartstring, &mut buf, config).unwrap();
        let smartstring: SmartString<Compact> =
            bincode::decode_from_slice(&buf[..len], config).unwrap().0;
        assert_eq!(smartstring, short_str);

        let smartstring = SmartString::<LazyCompact>::from(long_str);
        let len = bincode::encode_into_slice(smartstring, &mut buf, config).unwrap();
        let smartstring: SmartString<Compact> =
            bincode::decode_from_slice(&buf[..len], config).unwrap().0;
        assert_eq!(smartstring, long_str);
    }
}
