mod adapters;
mod basic_api;
mod join;
mod serialization;

pub mod dummy_components {
    use dynamecs::storages::VecStorage;
    use dynamecs::Component;
    use serde::{Deserialize, Serialize};

    macro_rules! generate_dummy_components {
    ($($name:ident),*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $name(pub usize);

            impl Component for $name {
                type Storage = VecStorage<Self>;
            }
        )*
    }
}

    generate_dummy_components!(A, B, C, D, E, F, G, H);
}
