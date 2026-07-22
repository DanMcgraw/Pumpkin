use pumpkin_macros::packet;

use crate::serial::PacketWrite;

/// Requests a latency acknowledgement from the Bedrock client.
#[derive(PacketWrite)]
#[packet(115)]
pub struct CNetworkStackLatency {
    pub timestamp: i64,
    pub from_server: bool,
}

#[cfg(test)]
mod tests {
    use crate::serial::PacketWrite;

    use super::CNetworkStackLatency;

    #[test]
    fn writes_timestamp_and_server_origin_flag() {
        let packet = CNetworkStackLatency {
            timestamp: -9_876_543_210,
            from_server: true,
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).unwrap();

        let mut expected = packet.timestamp.to_le_bytes().to_vec();
        expected.push(1);
        assert_eq!(bytes, expected);
    }
}
