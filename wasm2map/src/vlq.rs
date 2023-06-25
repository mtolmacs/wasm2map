// Simple implementation of VLQ (variable-length quality) encoding to avoid
// yet another dependency to accomplish this simple task
//
// TODO(mtolmacs): Use smallvec instead of string
pub(crate) fn encode(value: i64) -> String {
    const VLQ_CHARS: &[u8] =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".as_bytes();
    let mut x = if value >= 0 {
        value << 1
    } else {
        (-value << 1) + 1
    };
    let mut result = String::new();

    while x > 31 {
        let idx: usize = (32 + (x & 31)).try_into().unwrap();
        let ch: char = VLQ_CHARS[idx].into();
        result.push(ch);
        x >>= 5;
    }
    let idx: usize = x.try_into().unwrap();
    let ch: char = VLQ_CHARS[idx].into();
    result.push(ch);

    result
}

pub(crate) fn encode_uint_var(mut n: u32) -> Vec<u8> {
    let mut result = Vec::new();
    while n > 127 {
        result.push((128 | (n & 127)) as u8);
        n >>= 7;
    }
    result.push(n as u8);
    result
}
