use serde::{Deserialize, Serialize};

use crate::{chunk::ChunkConfig, lighting::LightingEngineConfig};

/// Configuration for world and level-specific settings.
///
/// Currently, it includes chunk-related options; more settings may be added later.
#[derive(Deserialize, Serialize, Clone)]
pub struct LevelConfig {
    /// Configuration for chunk behaviour and management.
    pub chunk: ChunkConfig,
    #[serde(default)]
    pub lighting: LightingEngineConfig,
    /// Number of ticks between autosave checks. If 0, autosave is disabled.
    #[serde(default = "default_autosave_ticks")]
    pub autosave_ticks: u64,
    /// Horizontal distance in blocks within which dropped items can merge.
    #[serde(default = "default_item_merge_distance")]
    pub item_merge_distance: f64,
    // TODO: More options
}

const fn default_autosave_ticks() -> u64 {
    6000 // Default to 5 minutes at 20 TPS
}

const fn default_item_merge_distance() -> f64 {
    0.5
}

impl Default for LevelConfig {
    fn default() -> Self {
        Self {
            chunk: ChunkConfig::default(),
            lighting: LightingEngineConfig::default(),
            // Preserve the existing derived `Default` behavior for autosaves.
            autosave_ticks: 0,
            item_merge_distance: default_item_merge_distance(),
        }
    }
}

impl LevelConfig {
    pub fn validate(&self) {
        assert!(
            self.item_merge_distance.is_finite() && self.item_merge_distance >= 0.0,
            "Item merge distance must be a finite, non-negative number"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::LevelConfig;

    #[test]
    fn item_merge_distance_defaults_to_vanilla_value() {
        assert_eq!(LevelConfig::default().item_merge_distance, 0.5);
    }

    #[test]
    fn item_merge_distance_can_be_deserialized() {
        let config: LevelConfig = toml::from_str(
            r#"
item_merge_distance = 2.25

[chunk]
type = "pump"
"#,
        )
        .expect("level config should deserialize");

        assert_eq!(config.item_merge_distance, 2.25);
    }
}
