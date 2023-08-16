use std::borrow::Cow;
use std::str;

// Inspired by:
// <https://github.com/serde-rs/json/blob/a0ddb25ff6b86f43912f8fc637797bcbb920c61e/src/ser.rs#L1997-L2063>
pub(crate) fn encode(string: &str) -> Cow<'_, str> {
    const BB: u8 = b'b'; // \x08
    const TT: u8 = b't'; // \x09
    const NN: u8 = b'n'; // \x0A
    const FF: u8 = b'f'; // \x0C
    const RR: u8 = b'r'; // \x0D
    const QU: u8 = b'"'; // \x22
    const BS: u8 = b'\\'; // \x5C
    const UU: u8 = b'u'; // \x00...\x1F except the ones above
    const __: u8 = 0;

    // Lookup table of escape sequences. A value of b'x' at index i means that byte
    // i is escaped as "\x" in JSON. A value of 0 means that byte i is not escaped.
    static ESCAPE: [u8; 256] = [
        //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
        UU, UU, UU, UU, UU, UU, UU, UU, BB, TT, NN, UU, FF, RR, UU, UU, // 0
        UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, // 1
        __, __, QU, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 3
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
        __, __, __, __, __, __, __, __, __, __, __, __, BS, __, __, __, // 5
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
    ];

    let mut start = 0;
    let mut new_string = String::new();

    for (index, byte) in string.bytes().enumerate() {
        let escape = ESCAPE[byte as usize];
        if escape == 0 {
            continue;
        }

        if start < index {
            new_string += &string[start..index];
        }

        match escape {
            QU => new_string += r#"\""#,
            BS => new_string += r"\\",
            BB => new_string += r"\b",
            FF => new_string += r"\f",
            NN => new_string += r"\n",
            RR => new_string += r"\r",
            TT => new_string += r"\t",
            UU => {
                static HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";

                new_string += str::from_utf8(&[
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    HEX_DIGITS[(byte >> 4) as usize],
                    HEX_DIGITS[(byte & 0xF) as usize],
                ])
                .unwrap()
            }
            _ => unreachable!(),
        }

        start = index + 1;
    }

    if new_string.is_empty() {
        string.into()
    } else {
        new_string.into()
    }
}
