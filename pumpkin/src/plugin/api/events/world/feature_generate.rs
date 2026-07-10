use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::{position::BlockPos, vector2::Vector2};
use std::sync::Arc;

/// Fired when the world generator is about to place a feature
/// (ores, trees, structures, etc.) during chunk population.
#[cancellable]
#[derive(Event, Clone)]
pub struct FeatureGenerateEvent {
    /// The world in which the feature is being generated.
    pub world: Arc<World>,
    /// The chunk position being populated.
    pub chunk_pos: Vector2<i32>,
    /// The placed feature about to be generated.
    pub feature: pumpkin_data::placed_feature::PlacedFeature,
    /// The origin position of the feature placement.
    pub origin: BlockPos,
}

impl FeatureGenerateEvent {
    #[must_use]
    pub const fn new(
        world: Arc<World>,
        chunk_pos: Vector2<i32>,
        feature: pumpkin_data::placed_feature::PlacedFeature,
        origin: BlockPos,
    ) -> Self {
        Self {
            world,
            chunk_pos,
            feature,
            origin,
            cancelled: false,
        }
    }
}
