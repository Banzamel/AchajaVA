//! The Cast v2 wire format: a 4-byte big-endian length, then a protobuf CastMessage
//! (protocol_version=1, source_id=2, destination_id=3, namespace=4, payload_type=5,
//! payload_utf8=6). Only string payloads are used, so a hand-written codec is enough.

#[derive(Debug, Clone, PartialEq)]
pub struct CastMessage {
    pub source: String,
    pub destination: String,
    pub namespace: String,
    pub payload: String,
}

fn varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn string_field(out: &mut Vec<u8>, field: u64, value: &str) {
    varint(out, (field << 3) | 2);
    varint(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

/// The framed bytes to write to the socket.
pub fn encode(msg: &CastMessage) -> Vec<u8> {
    let mut body = Vec::new();
    varint(&mut body, 1 << 3); // protocol_version
    varint(&mut body, 0); // CASTV2_1_0
    string_field(&mut body, 2, &msg.source);
    string_field(&mut body, 3, &msg.destination);
    string_field(&mut body, 4, &msg.namespace);
    varint(&mut body, 5 << 3); // payload_type
    varint(&mut body, 0); // STRING
    string_field(&mut body, 6, &msg.payload);
    let mut out = (body.len() as u32).to_be_bytes().to_vec();
    out.extend(body);
    out
}

fn read_varint(data: &[u8], pos: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *data.get(*pos)?;
        *pos += 1;
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

/// Decodes one message body (without the length prefix).
pub fn decode(body: &[u8]) -> Option<CastMessage> {
    let mut msg = CastMessage { source: String::new(), destination: String::new(), namespace: String::new(), payload: String::new() };
    let mut pos = 0;
    while pos < body.len() {
        let key = read_varint(body, &mut pos)?;
        match key & 7 {
            0 => {
                read_varint(body, &mut pos)?;
            }
            2 => {
                let len = read_varint(body, &mut pos)? as usize;
                let bytes = body.get(pos..pos + len)?;
                pos += len;
                let text = String::from_utf8_lossy(bytes).into_owned();
                match key >> 3 {
                    2 => msg.source = text,
                    3 => msg.destination = text,
                    4 => msg.namespace = text,
                    6 => msg.payload = text,
                    _ => {}
                }
            }
            _ => return None,
        }
    }
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let msg = CastMessage {
            source: "sender-0".into(),
            destination: "receiver-0".into(),
            namespace: "urn:x-cast:com.google.cast.tp.connection".into(),
            payload: "{\"type\":\"CONNECT\"}".into(),
        };
        let framed = encode(&msg);
        let len = u32::from_be_bytes([framed[0], framed[1], framed[2], framed[3]]) as usize;
        assert_eq!(len, framed.len() - 4);
        assert_eq!(decode(&framed[4..]), Some(msg));
    }
}
