/// Parse TLS ClientHello to extract SNI hostname.
/// Returns None if not a TLS ClientHello or SNI extension not found.
pub fn extract_sni(buf: &[u8]) -> Option<String> {
    // TLS record: type=0x16 (handshake), version, length
    if buf.len() < 5 || buf[0] != 0x16 {
        return None;
    }

    let record_len = u16::from_be_bytes([buf[3], buf[4]]) as usize;
    if buf.len() < 5 + record_len {
        return None;
    }

    let hs = &buf[5..];

    // Handshake type: 0x01 = ClientHello
    if hs.is_empty() || hs[0] != 0x01 {
        return None;
    }

    if hs.len() < 4 {
        return None;
    }

    let hs_len = u32::from_be_bytes([0, hs[1], hs[2], hs[3]]) as usize;
    if hs.len() < 4 + hs_len {
        return None;
    }

    let ch = &hs[4..4 + hs_len];

    // Skip: version(2) + random(32) = 34 bytes
    if ch.len() < 34 {
        return None;
    }
    let mut pos = 34;

    // Session ID length
    if pos >= ch.len() {
        return None;
    }
    let session_id_len = ch[pos] as usize;
    pos += 1 + session_id_len;

    // Cipher suites length (2 bytes)
    if pos + 2 > ch.len() {
        return None;
    }
    let cipher_len = u16::from_be_bytes([ch[pos], ch[pos + 1]]) as usize;
    pos += 2 + cipher_len;

    // Compression methods length (1 byte)
    if pos >= ch.len() {
        return None;
    }
    let comp_len = ch[pos] as usize;
    pos += 1 + comp_len;

    // Extensions length (2 bytes)
    if pos + 2 > ch.len() {
        return None;
    }
    let ext_len = u16::from_be_bytes([ch[pos], ch[pos + 1]]) as usize;
    pos += 2;

    let ext_end = pos + ext_len;
    if ext_end > ch.len() {
        return None;
    }

    // Walk extensions
    while pos + 4 <= ext_end {
        let ext_type = u16::from_be_bytes([ch[pos], ch[pos + 1]]);
        let ext_data_len = u16::from_be_bytes([ch[pos + 2], ch[pos + 3]]) as usize;
        pos += 4;

        if ext_type == 0x0000 {
            // SNI extension
            if ext_data_len < 2 || pos + ext_data_len > ch.len() {
                return None;
            }
            let sni_list = &ch[pos..pos + ext_data_len];
            // SNI list length (2 bytes)
            if sni_list.len() < 2 {
                return None;
            }
            let mut sni_pos = 2;

            while sni_pos + 3 <= sni_list.len() {
                let name_type = sni_list[sni_pos];
                let name_len =
                    u16::from_be_bytes([sni_list[sni_pos + 1], sni_list[sni_pos + 2]]) as usize;
                sni_pos += 3;

                if name_type == 0x00 && sni_pos + name_len <= sni_list.len() {
                    return String::from_utf8(sni_list[sni_pos..sni_pos + name_len].to_vec()).ok();
                }
                sni_pos += name_len;
            }
            return None;
        }

        pos += ext_data_len;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_tls() {
        assert_eq!(extract_sni(b"hello"), None);
        assert_eq!(extract_sni(&[]), None);
    }
}
