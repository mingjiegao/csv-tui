pub fn column_name(mut index: usize) -> String {
    let mut chars = Vec::new();
    loop {
        chars.push((b'a' + (index % 26) as u8) as char);
        index /= 26;
        if index == 0 { break; }
        index -= 1;
    }
    chars.iter().rev().collect()
}

pub fn column_names(count: usize) -> Vec<String> {
    (0..count).map(column_name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generates_spreadsheet_style_names() {
        assert_eq!(column_name(0), "a");
        assert_eq!(column_name(25), "z");
        assert_eq!(column_name(26), "aa");
        assert_eq!(column_name(27), "ab");
        assert_eq!(column_name(51), "az");
        assert_eq!(column_name(52), "ba");
        assert_eq!(column_name(701), "zz");
        assert_eq!(column_name(702), "aaa");
    }
}
