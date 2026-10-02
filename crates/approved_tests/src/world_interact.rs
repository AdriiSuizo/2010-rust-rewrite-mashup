//! Use on the Minecraft world as the MW2 soldier: a crafting table in
//! sight takes the Use key and opens; with nothing in sight Use stays
//! MW2's; a knife into a table opens it too.
use frame::{InteractKind, InteractTarget, WorldInteract};

fn table(block: [i32; 3]) -> InteractTarget {
    InteractTarget {
        kind: InteractKind::CraftingTable,
        label: "Crafting Table".into(),
        block,
    }
}

#[test]
fn use_on_a_crafting_table_opens_it() {
    let mut world = WorldInteract {
        target: Some(table([4, 64, -2])),
        ..Default::default()
    };
    assert!(world.press_use());
    let used = world.take(&[]);
    assert_eq!(used, Some(table([4, 64, -2])));
    assert!(!world.pressed);
}

#[test]
fn use_with_nothing_in_sight_stays_mw2s() {
    let mut world = WorldInteract::default();
    assert!(!world.claims_use());
    assert!(!world.press_use());
    assert_eq!(world.take(&[]), None);
}

#[test]
fn a_knife_into_a_crafting_table_opens_it() {
    let mut world = WorldInteract::default();
    let used = world.take(&[table([0, 70, 3])]);
    assert_eq!(used.map(|t| t.kind), Some(InteractKind::CraftingTable));
}

/// A press is used once: the next frame without one opens nothing.
#[test]
fn a_press_is_taken_once() {
    let mut world = WorldInteract {
        target: Some(table([1, 2, 3])),
        ..Default::default()
    };
    world.press_use();
    assert!(world.take(&[]).is_some());
    assert!(world.take(&[]).is_none());
}
