use crate::serial::PacketRead;
use pumpkin_macros::packet;

#[derive(PacketRead)]
#[packet(129)]
pub struct SClientCacheStatus {
    // https://mojang.github.io/bedrock-protocol-docs/html/ClientCacheStatusPacket.html
    pub cache_supported: bool,
}

#[cfg(test)]
mod tests {
    use crate::serial::PacketRead;

    use super::SClientCacheStatus;

    #[test]
    fn reads_client_capability_without_enabling_cache_protocol() {
        assert!(
            SClientCacheStatus::read(&mut &[1][..])
                .unwrap()
                .cache_supported
        );
        assert!(
            !SClientCacheStatus::read(&mut &[0][..])
                .unwrap()
                .cache_supported
        );
    }
}
