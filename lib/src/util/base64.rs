const BASE64_TABLE: [char; 64] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z', '-', '_',
];

pub fn format_base64(bytes: &[u8]) -> String {
    let mut string = String::with_capacity(bytes.len() * 4 / 3);
    let mut trailing: u8 = 0;
    let mut step = 0;
    for b in bytes {
        if step == 0 {
            string.push(BASE64_TABLE[(b % 64) as usize]);
            trailing = b >> 6;
            step = 1;
        } else if step == 1 {
            string.push(BASE64_TABLE[(((b << 2) + trailing) % 64) as usize]);
            trailing = b >> 4;
            step = 2;
        } else {
            string.push(BASE64_TABLE[(((b << 4) + trailing) % 64) as usize]);
            string.push(BASE64_TABLE[(b >> 2) as usize]);
            step = 0;
        }
    }
    string
}
