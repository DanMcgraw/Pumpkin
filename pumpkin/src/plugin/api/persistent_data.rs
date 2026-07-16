use pumpkin_nbt::{compound::NbtCompound, tag::NbtTag};

/// Maximum serialized data retained for one plugin namespace on one player.
pub const MAX_PLAYER_NAMESPACE_BYTES: usize = 64 * 1024;

/// Errors returned by the namespaced player-data API.
#[derive(Debug, thiserror::Error)]
pub enum PluginDataError {
    #[error("plugin data namespace or key is invalid")]
    InvalidKey,
    #[error("plugin data exceeds the {MAX_PLAYER_NAMESPACE_BYTES}-byte namespace limit")]
    NamespaceTooLarge,
    #[error("player data could not be encoded: {0}")]
    Encode(String),
    #[error("player data could not be loaded or saved: {0}")]
    Storage(String),
}

/// Errors returned by the bounded plugin batch-block primitive.
#[derive(Debug, thiserror::Error)]
pub enum BatchBlockError {
    #[error("a batch block action may contain at most 128 unique positions")]
    TooLarge,
}

pub(crate) fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

pub(crate) fn namespace_with_value(
    root: &NbtCompound,
    namespace: &str,
    key: &str,
    value: NbtTag,
) -> Result<NbtCompound, PluginDataError> {
    if !valid_component(namespace) || !valid_component(key) {
        return Err(PluginDataError::InvalidKey);
    }

    let mut namespace_data = root
        .get(namespace)
        .and_then(NbtTag::extract_compound)
        .cloned()
        .unwrap_or_default();
    namespace_data.child_tags.insert(key.into(), value);

    let mut encoded = Vec::new();
    pumpkin_nbt::to_bytes_unnamed(&namespace_data, &mut encoded)
        .map_err(|error| PluginDataError::Encode(error.to_string()))?;
    if encoded.len() > MAX_PLAYER_NAMESPACE_BYTES {
        return Err(PluginDataError::NamespaceTooLarge);
    }

    Ok(namespace_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_namespaced_components() {
        assert!(valid_component("cabbage_mmo"));
        assert!(valid_component("skills.v2"));
        assert!(!valid_component("Cabbage"));
        assert!(!valid_component("contains:separator"));
        assert!(!valid_component(""));
    }

    #[test]
    fn enforces_the_per_namespace_quota() {
        let root = NbtCompound::new();
        let oversized = NbtTag::ByteArray(vec![0; MAX_PLAYER_NAMESPACE_BYTES + 1].into());
        assert!(matches!(
            namespace_with_value(&root, "cabbage", "skills", oversized),
            Err(PluginDataError::NamespaceTooLarge)
        ));
    }
}
