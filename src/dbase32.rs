use libc;

const DB32ALPHABET: &str = "3456789ABCDEFGHIJKLMNOPQRSTUVWXY";
const MAX_BIN_LEN: usize = 60; //480 bits
const MAX_TXT_LEN: usize = 96;

const DB32_START: u32 = 51;
const DB32_END: u32 = 89;
const DB32_FORWARD: &str = DB32ALPHABET;
const DB32_REVERSE: [u8; 256] = [
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

const DB32_SET: &str = DB32_FORWARD; //encode()???
static _ASCII: [u8; 128] = [0;128];

fn _text_to_bytes(text: &str) -> std::str::Bytes {
    let b: std::str::Bytes = text.bytes();
    return b;
}

fn _check_length(text: &str) -> Result<&str, &str> {
    if text.len() < 8 || text.len() > MAX_TXT_LEN {
        return Err(text)
    }
    if text.len() % 8 != 0 {
        return Err(text)
    }
    
    Ok(text)
    
}

macro_rules! rotate {
    ($i:ident) => {
        DB32_REVERSE[$i as usize - 42]
    }
}

fn _validate(text: &str) -> bool {
    let block: usize;
    let count: usize;
    let mut r: u8 = 0;
    
    if text.len() < 8 || text.len() > MAX_TXT_LEN || text.len() % 8 != 0 {
        return false
    }
    
    count = text.len() / 8;
    for block in 0..count {
        let mut i = text.bytes().nth(8*block + 0).unwrap();    r != rotate!(i);
        i = text.bytes().nth(8*block + 1).unwrap();    r |= rotate!(i);
        i = text.bytes().nth(8*block + 2).unwrap();    r |= rotate!(i);
        i = text.bytes().nth(8*block + 3).unwrap();    r |= rotate!(i);
        i = text.bytes().nth(8*block + 4).unwrap();    r |= rotate!(i);
        i = text.bytes().nth(8*block + 5).unwrap();    r |= rotate!(i);
        i = text.bytes().nth(8*block + 6).unwrap();    r |= rotate!(i);
        i = text.bytes().nth(8*block + 7).unwrap();    r |= rotate!(i);
    }
    
    if (r & 224) != 0 { 
        false 
    }
    else { 
        true 
    }
    
}

//make one where you give both buffers  (this way with the string is handy too)
pub fn encode(bin_text: &[u8]) -> String {
    let block: usize;
    let count: usize;
    let mut taxi: u64;
    
    let mut text: String = String::new();
    
    count = bin_text.len()/5;
    for block in 0..count {
        taxi = bin_text[5*block + 0] as u64;
        taxi = bin_text[5*block + 1] as u64 | (taxi << 8);
        taxi = bin_text[5*block + 2] as u64 | (taxi << 8);
        taxi = bin_text[5*block + 3] as u64 | (taxi << 8);
        taxi = bin_text[5*block + 4] as u64 | (taxi << 8);
        
        text.push(DB32_FORWARD.bytes().nth(((taxi >> 35) & 31) as usize).unwrap() as char);
        text.push(DB32_FORWARD.bytes().nth(((taxi >> 30) & 31) as usize).unwrap() as char);
        text.push(DB32_FORWARD.bytes().nth(((taxi >> 25) & 31) as usize).unwrap() as char);
        text.push(DB32_FORWARD.bytes().nth(((taxi >> 20) & 31) as usize).unwrap() as char);
        text.push(DB32_FORWARD.bytes().nth(((taxi >> 15) & 31) as usize).unwrap() as char);
        text.push(DB32_FORWARD.bytes().nth(((taxi >> 10) & 31) as usize).unwrap() as char);
        text.push(DB32_FORWARD.bytes().nth(((taxi >> 5) & 31) as usize).unwrap() as char);
        text.push(DB32_FORWARD.bytes().nth(((taxi) & 31) as usize).unwrap() as char);
    }
    
    text
    
}


pub fn decode(text: &[u8]) -> String {
    let block: usize;
    let count: usize;
    let mut taxi: u64;
    
    let mut bin_text: String = String::new();
    
    let mut r: u8 = 0;
    count = text.len()/8;
    for block in 0..count {
        let mut i = text[8*block + 0];    r = rotate!(i) | r & 224;    taxi = r as u64;
        i = text[8*block + 1];    r = rotate!(i) | r & 224;    taxi = r as u64 | (taxi << 5);
        i = text[8*block + 2];    r = rotate!(i) | r & 224;    taxi = r as u64 | (taxi << 5);
        i = text[8*block + 3];    r = rotate!(i) | r & 224;    taxi = r as u64 | (taxi << 5);
        i = text[8*block + 4];    r = rotate!(i) | r & 224;    taxi = r as u64 | (taxi << 5);
        i = text[8*block + 5];    r = rotate!(i) | r & 224;    taxi = r as u64 | (taxi << 5);
        i = text[8*block + 6];    r = rotate!(i) | r & 224;    taxi = r as u64 | (taxi << 5);
        i = text[8*block + 7];    r = rotate!(i) | r & 224;    taxi = r as u64 | (taxi << 5);
        
        bin_text.push(((taxi >> 32) & 255) as u8 as char);
        bin_text.push(((taxi >> 24) & 255) as u8 as char);
        bin_text.push(((taxi >> 16) & 255) as u8 as char);
        bin_text.push(((taxi >> 8) & 255) as u8 as char);
        bin_text.push(((taxi >> 0) & 255) as u8 as char);
    }
    bin_text
}

//db32enc
//db32dec
//isdb32
pub fn isdb32(text: &str) -> bool {
    
    let txt_len: usize;
    
    _validate(text)
    
}

//check_db32
pub fn check_db32(text: &str) -> Result<(), String> {
    let valid = _validate(text);
    
    if !valid {
        return Err("ER".to_string());
    }
    Ok(())
    
}

//random_id
pub fn random_id() -> String {
    //wrap the getrandom syscall
    
    let mut buf: [u8; 15] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
    unsafe { libc::getrandom(buf.as_mut_ptr().cast(), buf.len(), 0) }; 
    
    let b: String = std::str::from_utf8(&buf).unwrap().to_string();
    //let b: String = buf.iter().into<char>().collect();
    
    encode(&buf)
}

//time_id
fn _check_join(string_list: Vec<&str>) -> Result<String, String> {
    let pre: &str = "/";
    let s = string_list.last().unwrap();
    print!("{}",s);
    if _validate(&s) {
        Ok(s.to_string())
    }
    else {
        Err(s.to_string())
    }
}
//db32_join
pub fn db32_join(string_list: Vec<&str>) -> Result<String, String> {
    let pre: &str = "/";
    let s = "/".to_string() + &string_list.join("/");
    match _check_join(string_list) {
        Ok(o) => Ok(s),
        Err(e) => Err(s),
        
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
        Ok(o) => Ok(s),
        Err(e) => Err(s),
        
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
    
    #[test]
    fn test_check_length() {
        let short = super::_check_length("SHORT");
        assert_eq!(short, Err("SHORT"));
        
        
        let long = super::_check_length(
            "LONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONG"
        );
        assert_eq!(
            long, 
            Err("LONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONGLONG")
        );
        
        let noteight = super::_check_length("NOTQUITEEIGHT");
        assert_eq!(noteight, Err("NOTQUITEEIGHT"));
        
        let noteight = super::_check_length("IAMEIGHT");
        assert_eq!(noteight, Ok("IAMEIGHT"));
        
    }
    
    #[test]
    fn test_encode() {
        let result = super::encode(b"binary foo");
        assert_eq!(result, "FCNPVRELI7J9FUUI");
    }
    
    #[test]
    fn test_decode() {
        let result = super::decode(b"FCNPVRELI7J9FUUI");
        assert_eq!(result, "binary foo");
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
    }
    
}
