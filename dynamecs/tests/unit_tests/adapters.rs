use dynamecs::{
    adapters::{FilterSystem, FnOnceSystem, FnSystem, SingleShotSystem},
    storages::SingularStorage,
    Component, System, Universe,
};

#[test]
fn fn_system() {
    let mut value = 0;
    let mut system = FnSystem::new("test", |_universe| {
        value = 27;
        Ok(())
    });

    assert_eq!(system.name(), "test");

    let mut universe = Universe::default();
    let res = system.run(&mut universe);

    assert!(res.is_ok());
    assert_eq!(value, 27);
}

#[test]
fn fn_once_system() {
    struct Test {
        value: i32,
    }

    let value_source = Test { value: 27 };
    let mut value_target = Test { value: 0 };
    let value_target_mut = &mut value_target;

    let mut system = FnOnceSystem::new("test", move |_universe| {
        *value_target_mut = value_source;
        Ok(())
    });

    assert_eq!(system.has_run(), false);
    assert_eq!(system.name(), "test");

    let mut universe = Universe::default();
    let res = system.run(&mut universe);

    assert!(res.is_ok());
    assert_eq!(system.has_run(), true);
    assert_eq!(value_target.value, 27);
}

#[derive(Debug)]
struct MockSystem {}

impl MockSystem {
    fn runs(universe: &Universe) -> usize {
        universe
            .get_component_storage::<MocComponent>()
            .get_component()
            .system_runs
    }
}

impl System for MockSystem {
    fn run(&mut self, universe: &mut Universe) -> eyre::Result<()> {
        universe
            .get_component_storage_mut::<MocComponent>()
            .get_component_mut()
            .system_runs += 1;
        Ok(())
    }
}

#[derive(Default)]
struct MocComponent {
    system_runs: usize,
}

impl Component for MocComponent {
    type Storage = SingularStorage<Self>;
}

#[test]
fn moc_system() {
    let mut universe = Universe::default();

    assert_eq!(MockSystem::runs(&universe), 0);

    let mut system = MockSystem {};

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 1);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 2);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 3);
}

#[test]
fn single_shot_system() {
    let mut universe = Universe::default();

    let mut system = SingleShotSystem::new(MockSystem {});

    assert_eq!(MockSystem::runs(&universe), 0);
    assert_eq!(system.has_run(), false);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(system.has_run(), true);
    assert_eq!(MockSystem::runs(&universe), 1);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 1);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 1);
}

#[test]
fn single_shot_system_combinator() {
    let mut universe = Universe::default();

    let mut system = MockSystem {}.single_shot();

    assert_eq!(MockSystem::runs(&universe), 0);
    assert_eq!(system.has_run(), false);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(system.has_run(), true);
    assert_eq!(MockSystem::runs(&universe), 1);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 1);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 1);
}

#[test]
fn filter_system() {
    let mut universe = Universe::default();

    assert_eq!(MockSystem::runs(&universe), 0);

    let mut runs = 0;
    let mut system = FilterSystem::new(MockSystem {}, move |_| {
        runs += 1;
        Ok(runs > 2)
    });

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 0);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 0);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 1);
}

#[test]
fn filter_system_combinator() {
    let mut universe = Universe::default();

    assert_eq!(MockSystem::runs(&universe), 0);

    let mut runs = 0;
    let mut system = MockSystem {}.filter(move |_| {
        runs += 1;
        Ok(runs > 2)
    });

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 0);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 0);

    let res = system.run(&mut universe);
    assert!(res.is_ok());
    assert_eq!(MockSystem::runs(&universe), 1);
}
