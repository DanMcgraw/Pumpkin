#[cfg(test)]
mod tests {
    use std::sync::{Arc, Weak};

    use pumpkin_config::world::LevelConfig;
    use pumpkin_data::{Block, item_stack::ItemStack};
    use pumpkin_util::{
        math::{position::BlockPos, vector3::Vector3},
        world_seed::Seed,
    };
    use pumpkin_world::level::Level;
    use tempfile::tempdir;

    use crate::{
        plugin::{
            Cancellable,
            api::events::{
                block::{
                    brew::BrewEvent, furnace_burn::FurnaceBurnEvent,
                    furnace_smelt::FurnaceSmeltEvent,
                },
                inventory::inventory_move_item::InventoryMoveItemEvent,
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
        World::load(
            &level,
            level_info,
            pumpkin_data::dimension::Dimension::OVERWORLD,
            Arc::new(crate::block::registry::BlockRegistry::default()),
            Weak::new(),
        )
    }

    #[tokio::test]
    async fn inventory_move_item_event_is_cancellable() {
        let mut event = InventoryMoveItemEvent::new(
            ItemStack::EMPTY.clone(),
            None,
            None,
            Some(BlockPos(Vector3::new(1, 2, 3))),
            Some(BlockPos(Vector3::new(4, 5, 6))),
        );
        assert!(!event.cancelled());
        event.item = ItemStack::new(1, &pumpkin_data::item::Item::STONE);
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.source_pos, Some(BlockPos(Vector3::new(1, 2, 3))));
        assert_eq!(event.destination_pos, Some(BlockPos(Vector3::new(4, 5, 6))));
    }

    #[tokio::test]
    async fn furnace_burn_event_is_cancellable() {
        let world = test_world();
        let mut event = FurnaceBurnEvent::new(
            &Block::FURNACE,
            BlockPos(Vector3::new(0, 64, 0)),
            world,
            ItemStack::new(1, &pumpkin_data::item::Item::COAL),
            1600,
        );
        assert!(!event.cancelled());
        event.burn_time = 800;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.burn_time, 800);
    }

    #[tokio::test]
    async fn furnace_smelt_event_is_cancellable() {
        let world = test_world();
        let mut event = FurnaceSmeltEvent::new(
            &Block::FURNACE,
            BlockPos(Vector3::new(0, 64, 0)),
            world,
            ItemStack::new(1, &pumpkin_data::item::Item::IRON_ORE),
            ItemStack::new(1, &pumpkin_data::item::Item::COAL),
            ItemStack::new(1, &pumpkin_data::item::Item::IRON_INGOT),
        );
        assert!(!event.cancelled());
        event.output = ItemStack::new(2, &pumpkin_data::item::Item::IRON_INGOT);
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.output.item_count, 2);
    }

    #[tokio::test]
    async fn brew_event_is_cancellable() {
        let world = test_world();
        let mut event = BrewEvent::new(
            &Block::BREWING_STAND,
            BlockPos(Vector3::new(0, 64, 0)),
            world,
            ItemStack::new(1, &pumpkin_data::item::Item::NETHER_WART),
            vec![ItemStack::new(1, &pumpkin_data::item::Item::POTION)],
            20,
        );
        assert!(!event.cancelled());
        event.fuel = 10;
        event.set_cancelled(true);
        assert!(event.cancelled());
        assert_eq!(event.fuel, 10);
    }
}
