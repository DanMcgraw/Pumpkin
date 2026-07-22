use pumpkin_macros::packet;

use crate::serial::PacketRead;

/// A Bedrock latency request or acknowledgement.
#[derive(Debug, PacketRead)]
#[packet(115)]
pub struct SNetworkStackLatency {
    pub timestamp: i64,
    pub from_server: bool,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::serial::PacketRead;

    use super::SNetworkStackLatency;

    #[test]
    fn reads_echoed_latency_packet() {
        let timestamp = -9_876_543_210_i64;
        let mut bytes = timestamp.to_le_bytes().to_vec();
        bytes.push(1);

        let packet = SNetworkStackLatency::read(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(packet.timestamp, timestamp);
        assert!(packet.from_server);
    }
}
