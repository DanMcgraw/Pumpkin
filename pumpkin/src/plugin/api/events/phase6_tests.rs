#[cfg(test)]
mod tests {
    use std::sync::{Arc, Weak};

    use pumpkin_config::world::LevelConfig;
    use pumpkin_data::{Block, placed_feature::PlacedFeature};
    use pumpkin_util::{
        math::{position::BlockPos, vector3::Vector3},
        world_seed::Seed,
    };
    use pumpkin_world::level::Level;
    use tempfile::tempdir;

    use crate::{
        block::registry::BlockRegistry,
        entity::{Entity, EntityBase},
        plugin::{
            Cancellable,
            api::events::{
                block::{
                    BlockEvent, block_form::BlockFormEvent, structure_grow::StructureGrowEvent,
                },
                entity::{
                    entity_block_form::EntityBlockFormEvent,
                    entity_change_block::EntityChangeBlockEvent,
                },
            },
        },
        world::{LevelData, World},
    };

    fn test_world() -> Arc<World> {
        let temp_dir = tempdir().unwrap();
        let level = Level::from_root_folder(
            &LevelConfig::default(),
            temp_dir.path().to_path_buf(),
            0,
            pumpkin_data::dimension::Dimension::OVERWORLD,
            None,
        );
        let level_info = Arc::new(arc_swap::ArcSwap::new(Arc::new(LevelData::default(Seed(
            0,
        )))));
        Arc::new(World::load(
            level,
            level_info,
            pumpkin_data::dimension::Dimension::OVERWORLD,
            Arc::new(BlockRegistry::default()),
            Weak::new(),
        ))
    }

    fn test_entity(world: Arc<World>) -> Arc<dyn EntityBase> {
        Arc::new(Entity::new(
            world,
            Vector3::new(0.0, 64.0, 0.0),
            &pumpkin_data::entity::EntityType::PIG,
        ))
    }

    #[tokio::test]
    async fn block_form_event_is_cancellable_and_mutable() {
        let world = test_world();
        let mut event = BlockFormEvent::new(
            world,
            &Block::WATER,
            BlockPos::new(0, 64, 0),
            &Block::ICE,
            Block::ICE.default_state.id,
        );
        assert!(!event.cancelled());
        event.new_state_id = Block::PACKED_ICE.default_state.id;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.new_state_id, Block::PACKED_ICE.default_state.id);
    }

    #[tokio::test]
    async fn entity_block_form_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world);
        let mut event = EntityBlockFormEvent::new(
            entity,
            BlockPos::new(0, 64, 0),
            Block::FIRE.default_state.id,
        );
        assert!(!event.cancelled());
        event.new_state_id = Block::SOUL_FIRE.default_state.id;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.new_state_id, Block::SOUL_FIRE.default_state.id);
    }

    #[tokio::test]
    async fn entity_change_block_event_is_cancellable_and_mutable() {
        let world = test_world();
        let entity = test_entity(world);
        let mut event = EntityChangeBlockEvent::new(
            entity,
            BlockPos::new(0, 64, 0),
            Block::GRASS_BLOCK.default_state.id,
            Block::DIRT.default_state.id,
        );
        assert!(!event.cancelled());
        event.new_state_id = Block::COARSE_DIRT.default_state.id;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.new_state_id, Block::COARSE_DIRT.default_state.id);
    }

    #[tokio::test]
    async fn structure_grow_event_is_cancellable_and_mutable() {
        let world = test_world();
        let mut event = StructureGrowEvent::new(
            world,
            &Block::OAK_SAPLING,
            BlockPos::new(0, 64, 0),
            PlacedFeature::OakChecked,
        );
        assert!(!event.cancelled());
        event.placed_feature = PlacedFeature::BirchChecked;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.placed_feature, PlacedFeature::BirchChecked);
    }

    #[tokio::test]
    async fn block_form_event_get_block_returns_original_block() {
        let world = test_world();
        let event = BlockFormEvent::new(
            world,
            &Block::WATER,
            BlockPos::new(0, 64, 0),
            &Block::ICE,
            Block::ICE.default_state.id,
        );
        assert_eq!(event.get_block(), &Block::WATER);
    }

    #[tokio::test]
    async fn structure_grow_event_get_block_returns_origin_block() {
        let world = test_world();
        let event = StructureGrowEvent::new(
            world,
            &Block::OAK_SAPLING,
            BlockPos::new(0, 64, 0),
            PlacedFeature::OakChecked,
        );
        assert_eq!(event.get_block(), &Block::OAK_SAPLING);
    }
}
