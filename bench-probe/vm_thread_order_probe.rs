pub fn __iw4l_thread_order_probe() {
    use crate::script::{Catalog, NativeRegistry};

    let sources = BTreeMap::from([(
        "probe/order".to_owned(),
        "caller() { wait 100; } waiting() { wait 100; }".to_owned(),
    )]);
    let program = Arc::new(Program::load(&sources, &["probe/order"], &Catalog::iw4()).unwrap());
    let caller = program.names["probe/order::caller"];
    let waiting = program.names["probe/order::waiting"];
    let mut world = World::new();
    world.insert_resource(Runtime::default());
    world.insert_resource(super::host::mechanics::Mechanics::default());
    world.insert_resource(NativeRegistry::default());
    world.resource_mut::<Runtime>().program = Some(program.clone());
    world.resource_mut::<Runtime>().next_serial = 1;
    for serial in 1..=4 {
        let mut thread = new_thread(&mut world, &program, caller, Value::level(), Vec::new()).unwrap();
        assert_eq!(thread.serial, serial);
        if serial == 1 || serial == 4 {
            thread.frames.push(frame(&program, waiting, Vec::new(), 0, Value::level()));
        }
        park(&mut world, thread);
    }
    world.resource_mut::<Runtime>().buckets.entry(0).or_default().push_back(1);
    run_ready(&mut world, &program, 0);
    assert!(world.resource::<Runtime>().fault.is_none());
    assert_eq!(return_from(&mut world, "probe/order::waiting", 0), 2);
    let queue: Vec<_> = world.resource::<Runtime>().buckets[&0].iter().copied().collect();
    assert_eq!(queue, [1, 4]);

    let mut copied = World::new();
    copy_state(&world, &mut copied);
    let copied_order: Vec<_> = copied.query::<&Thread>().iter(&copied).map(|t| t.serial).collect();
    assert_eq!(copied_order, [4, 2, 3, 1]);

    let entity = find_thread(&mut world, 1).unwrap();
    let detached = __probe_take(&mut world, entity);
    let mut borrowed_copy = World::new();
    copy_state(&world, &mut borrowed_copy);
    let borrowed_order: Vec<_> = borrowed_copy.query::<&Thread>().iter(&borrowed_copy).map(|t| t.serial).collect();
    assert_eq!(borrowed_order, [4, 2, 3]);
    assert_eq!(borrowed_copy.resource::<Runtime>().thread_entities.len(), 3);
    __probe_put(&mut world, entity, detached);
    println!("wake={queue:?}; copied={copied_order:?}; copied while detached={borrowed_order:?}");
}
