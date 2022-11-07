//! Custom base32 encoding used to encode IDs.

use crate::util::getrandom;


/* FIXME: Do we want to use this still?
const MAX_BIN_LEN: usize = 60; //480 bits
const MAX_TXT_LEN: usize = 96;
*/

static FORWARD: &[u8; 32] = b"3456789ABCDEFGHIJKLMNOPQRSTUVWXY";
static REVERSE: &[u8; 256] = &[
    255,255,255,255,255,255,255,255,255,
        // [Original] -> [Rotated]
      0,  // '3' [51] -> [ 9]
      1,  // '4' [52] -> [10]
      2,  // '5' [53] -> [11]
      3,  // '6' [54] -> [12]
      4,  // '7' [55] -> [13]
      5,  // '8' [56] -> [14]
      6,  // '9' [57] -> [15]
    255,  // ':' [58] -> [16]
    255,  // ';' [59] -> [17]
    255,  // '<' [60] -> [18]
    255,  // '=' [61] -> [19]
    255,  // '>' [62] -> [20]
    255,  // '?' [63] -> [21]
    255,  // '@' [64] -> [22]
      7,  // 'A' [65] -> [23]
      8,  // 'B' [66] -> [24]
      9,  // 'C' [67] -> [25]
     10,  // 'D' [68] -> [26]
     11,  // 'E' [69] -> [27]
     12,  // 'F' [70] -> [28]
     13,  // 'G' [71] -> [29]
     14,  // 'H' [72] -> [30]
     15,  // 'I' [73] -> [31]
     16,  // 'J' [74] -> [32]
     17,  // 'K' [75] -> [33]
     18,  // 'L' [76] -> [34]
     19,  // 'M' [77] -> [35]
     20,  // 'N' [78] -> [36]
     21,  // 'O' [79] -> [37]
     22,  // 'P' [80] -> [38]
     23,  // 'Q' [81] -> [39]
     24,  // 'R' [82] -> [40]
     25,  // 'S' [83] -> [41]
     26,  // 'T' [84] -> [42]
     27,  // 'U' [85] -> [43]
     28,  // 'V' [86] -> [44]
     29,  // 'W' [87] -> [45]
     30,  // 'X' [88] -> [46]
     31,  // 'Y' [89] -> [47]
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
];

//const DB32_SET: &str = FORWARD; //encode()???
//static _ASCII: [u8; 128] = [0;128];

fn _text_to_bytes(text: &str) -> std::str::Bytes {
    let b: std::str::Bytes = text.bytes();
    return b;
}

/// Iterates over the 1024 2-character Dbase32 directory names.
/// Will yield "33", "34", ... "YX", "YY".
#[derive(Debug)]
pub struct Name2Iter {
    i: usize,
}

impl Name2Iter {
    pub fn new() -> Self {
        Self {i: 0}
    }
}

impl Iterator for Name2Iter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < 1024 {
            let mut s = String::from("ZZ");
            let a = FORWARD[self.i >> 5];
            let b = FORWARD[self.i & 31];
            unsafe {
                let mut buf = s.as_bytes_mut();
                buf[0] = a;
                buf[1] = b;
            }
            self.i += 1;
            Some(s)
        }
        else {
            None
        }
    }
}

macro_rules! bin_at {
    ($bin:ident, $i:ident, $j:literal) => {
        $bin[$i * 5 + $j]
    }
}

macro_rules! txt_at {
    ($txt:ident, $i:ident, $j:literal) => {
        $txt[$i * 8 + $j]
    }
}

macro_rules! rotate {
    ($txt:ident, $i:ident, $j:literal) => {
        REVERSE[(txt_at!($txt, $i, $j).wrapping_sub(42)) as usize]
    }
}


pub fn db32enc_into(bin: &[u8], txt: &mut [u8]) {
    if bin.len() != 0 && bin.len() % 5 == 0 && txt.len() == bin.len() * 8 / 5 {
        assert!(txt.len() % 8 == 0);
        let mut taxi: u64 = 0;
        for i in 0..bin.len() / 5 {
            /* Pack 40 bits into the taxi (8 bits at a time) */
            taxi = bin_at!(bin, i, 0) as u64;
            taxi = bin_at!(bin, i, 1) as u64 | (taxi << 8);
            taxi = bin_at!(bin, i, 2) as u64 | (taxi << 8);
            taxi = bin_at!(bin, i, 3) as u64 | (taxi << 8);
            taxi = bin_at!(bin, i, 4) as u64 | (taxi << 8);

            /* Unpack 40 bits from the taxi (5 bits at a time) */
            txt_at!(txt, i, 0) = FORWARD[((taxi >> 35) & 31) as usize];
            txt_at!(txt, i, 1) = FORWARD[((taxi >> 30) & 31) as usize];
            txt_at!(txt, i, 2) = FORWARD[((taxi >> 25) & 31) as usize];
            txt_at!(txt, i, 3) = FORWARD[((taxi >> 20) & 31) as usize];
            txt_at!(txt, i, 4) = FORWARD[((taxi >> 15) & 31) as usize];
            txt_at!(txt, i, 5) = FORWARD[((taxi >> 10) & 31) as usize];
            txt_at!(txt, i, 6) = FORWARD[((taxi >>  5) & 31) as usize];
            txt_at!(txt, i, 7) = FORWARD[((taxi >>  0) & 31) as usize];
        }
    }
    else {
        panic!("db32enc_into(): Bad call");
    }
}

pub fn db32enc(bin: &[u8]) -> Vec<u8> {
    let mut txt = vec![0; bin.len() * 8 / 5];
    db32enc_into(bin, &mut txt);
    txt
}

pub fn db32enc_str(bin_src: &[u8]) -> String {
    String::from_utf8(db32enc(bin_src)).unwrap()
}


pub fn isdb32(txt: &[u8]) -> bool {
    if txt.len() != 0 && txt.len() % 8 == 0 {
        let mut r = 0_u8;
        for i in 0..txt.len() / 8 {
            r |= rotate!(txt, i, 0);
            r |= rotate!(txt, i, 1);
            r |= rotate!(txt, i, 2);
            r |= rotate!(txt, i, 3);
            r |= rotate!(txt, i, 4);
            r |= rotate!(txt, i, 5);
            r |= rotate!(txt, i, 6);
            r |= rotate!(txt, i, 7);
        }
        r & 224 == 0
    }
    else {
        false
    }
}


pub fn db32dec_into(txt: &[u8], bin: &mut [u8]) -> bool {
    if txt.len() != 0 && txt.len() % 8 == 0 && bin.len() == txt.len() * 5 / 8 {
        let mut taxi: u64 = 0;
        let mut r: u8 = 0;
        for i in 0..txt.len() / 8 {

            /* Pack 40 bits into the taxi (5 bits at a time) */
            r = rotate!(txt, i, 0) | (r & 224);    taxi = r as u64;
            r = rotate!(txt, i, 1) | (r & 224);    taxi = r as u64 | (taxi << 5);
            r = rotate!(txt, i, 2) | (r & 224);    taxi = r as u64 | (taxi << 5);
            r = rotate!(txt, i, 3) | (r & 224);    taxi = r as u64 | (taxi << 5);
            r = rotate!(txt, i, 4) | (r & 224);    taxi = r as u64 | (taxi << 5);
            r = rotate!(txt, i, 5) | (r & 224);    taxi = r as u64 | (taxi << 5);
            r = rotate!(txt, i, 6) | (r & 224);    taxi = r as u64 | (taxi << 5);
            r = rotate!(txt, i, 7) | (r & 224);    taxi = r as u64 | (taxi << 5);

            /* Unpack 40 bits from the taxi (8 bits at a time) */
            bin_at!(bin, i, 0) = (taxi >> 32) as u8 & 255;
            bin_at!(bin, i, 1) = (taxi >> 24) as u8 & 255;
            bin_at!(bin, i, 2) = (taxi >> 16) as u8 & 255;
            bin_at!(bin, i, 3) = (taxi >>  8) as u8 & 255;
            bin_at!(bin, i, 4) = (taxi >>  0) as u8 & 255;
        }
        /*
            31: 00011111 <= bits set in REVERSE for valid characters
            224: 11100000 <= bits set in REVERSE for invalid characters */
        r & 224 == 0
    }
    else {
        panic!("db32dec_into(): Bad call");
    }
}

pub fn db32dec(txt: &[u8]) -> Option<Vec<u8>> {
    let mut bin = vec![0; txt.len() * 5 / 8];
    if db32dec_into(txt, &mut bin) {
        return Some(bin)
    }
    None
}


//check_db32
pub fn check_db32(text: &str) -> Result<(), String> {
    let valid = isdb32(text.as_bytes());
    
    if !valid {
        return Err("ER".to_string());
    }
    Ok(())
    
}

//random_id
pub fn random_id() -> String {
    let mut buf: [u8; 15] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
    getrandom(&mut buf); 
    db32enc_str(&buf)
}

//time_id
fn _check_join(string_list: Vec<&str>) -> Result<String, String> {
    let s = string_list.last().unwrap();
    print!("{}",s);
    if isdb32(s.as_bytes()) {
        Ok(s.to_string())
    }
    else {
        Err(s.to_string())
    }
}
//db32_join
pub fn db32_join(string_list: Vec<&str>) -> Result<String, String> {
    let s = "/".to_string() + &string_list.join("/");
    match _check_join(string_list) {
        Ok(_) => Ok(s),
        Err(_) => Err(s),
        
    }
}

//db32_join_2
pub fn db32_join2(string_list: Vec<&str>) -> Result<String, String> {
    let last_part = *(string_list.last().unwrap());
    let i = last_part.chars().map(|c| c.len_utf8()).take(2).sum();
    let parts = match &string_list.len() {
       0..=1 => "".to_string(),
        _ => string_list.as_slice()[..&string_list.len()-1].join("/") + "/",
    };
   
    let s = "/".to_string() + &parts + &last_part[..i] + "/" + &last_part[i..];
    match _check_join(string_list) {
        Ok(_) => Ok(s),
        Err(_) => Err(s),
        
    }
}

#[cfg(test)]
mod tests {
    use crate::util::random_object_id;
    use super::*;

    #[test]
    fn test_name2iter() {
        let names = Vec::from_iter(Name2Iter::new());
        assert_eq!(names.len(), 1024);
        assert_eq!(names[0], "33");
        assert_eq!(names[1], "34");
        assert_eq!(names[2], "35");
        assert_eq!(names[1021], "YW");
        assert_eq!(names[1022], "YX");
        assert_eq!(names[1023], "YY");
    }

    #[test]
    #[should_panic (expected = "db32enc_into(): Bad call")]
    fn test_encode() {
        let bin: &[u8;10] = b"binary foo";
        let mut result: [u8;16] = [0;16];
        
        super::db32enc_into(bin, &mut result);
        assert_eq!(&result, b"FCNPVRELI7J9FUUI");
        
        let mut result: [u8;14] = [0;14];
        super::db32enc_into(bin, &mut result);
    }

    #[test]
    fn test_isdb32() {
        assert_eq!(isdb32(b""), false);
        assert_eq!(isdb32(b"A"), false);
        assert_eq!(isdb32(b"AA"), false);
        assert_eq!(isdb32(b"AAA"), false);
        assert_eq!(isdb32(b"AAAA"), false);
        assert_eq!(isdb32(b"AAAAA"), false);
        assert_eq!(isdb32(b"AAAAAA"), false);
        assert_eq!(isdb32(b"AAAAAAA"), false);
        assert_eq!(isdb32(b"AAAAAAAA"), true);
        assert_eq!(isdb32(b"AAAAAAAAA"), false);

        assert_eq!(isdb32(b"ABCDEFGH"), true);
        assert_eq!(isdb32(b"ZBCDEFGH"), false);
        assert_eq!(isdb32(b"AZCDEFGH"), false);
        assert_eq!(isdb32(b"ABZDEFGH"), false);
        assert_eq!(isdb32(b"ABCZEFGH"), false);
        assert_eq!(isdb32(b"ABCDZFGH"), false);
        assert_eq!(isdb32(b"ABCDEZGH"), false);
        assert_eq!(isdb32(b"ABCDEFZH"), false);
        assert_eq!(isdb32(b"ABCDEFGZ"), false);
    }

    #[test]
    fn test_roundtrip() {
        for _ in 0..50_000 {
            let bin = random_object_id();
            let txt = super::db32enc(&bin);
            let bin2 = super::db32dec(&txt).unwrap();
            assert_eq!(&bin, &bin2[..]);
        }
    }

    #[test]
    fn test_bad_txt() {
        let bin = random_object_id();
        let txt = db32enc(&bin);
        assert_eq!(isdb32(&txt), true);
        for i in 0..bin.len() {
            for v in 0..=255 {
                let mut copy = txt.clone();
                copy[i] = v;
                if FORWARD.contains(&v) {
                    assert_eq!(isdb32(&copy), true);
                    assert_ne!(db32dec(&copy).unwrap(), txt);
                }
                else {
                    assert_eq!(isdb32(&copy), false);
                    assert_eq!(db32dec(&copy), None);
                }
            }
        }
    }

    #[test]
    fn test_db32dec_into() {
        let txt = b"FCNPVRELI7J9FUUI";
        let mut bin = [0_u8; 10];
        assert_eq!(db32dec_into(txt, &mut bin), true);
        assert_eq!(&bin, b"binary foo");
    }

    #[test]
    fn test_db32dec() {
        assert_eq!(db32dec(b"FCNPVRELI7J9FUUI").unwrap(), b"binary foo");
        assert_eq!(db32dec(b"fcnpvreli7j9fuui"), None); 
    }

    #[test]
    fn test_db32_join() {
        //let parts = vec!["first", "second"];
        let parts = vec!["first", "second", "ABRYRYAB"];
        let result = super::db32_join(parts);
        assert_eq!(result, Ok("/first/second/ABRYRYAB".to_string()));
        
        let parts = vec!["first", "second", "11111111ABRYRYAB"];
        let result = super::db32_join(parts);
        assert_eq!(result, Err("/first/second/11111111ABRYRYAB".to_string()));
    }
    
    #[test]
    fn test_db32_join2() {
        //let parts = vec!["first", "second"];
        let parts = vec!["first", "second","ABRYRYAB"];
        let result = super::db32_join2(parts);
        assert_eq!(result, Ok("/first/second/AB/RYRYAB".to_string()));
        
        let parts = vec!["first", "second", "11111111ABRYRYAB"];
        let result = super::db32_join(parts);
        assert_eq!(result, Err("/first/second/11111111ABRYRYAB".to_string()));
    }
    
}
