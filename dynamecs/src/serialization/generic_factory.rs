use std::any::{Any, TypeId};
use std::marker::PhantomData;

use erased_serde::Serialize;
use eyre::eyre;

use crate::serialization::{EntityDeserialize, EntitySerializationMap};
use crate::StorageFactory;

#[derive(Debug, Default)]
pub struct GenericFactory<Storage> {
    marker: PhantomData<Storage>,
}

impl<Storage> GenericFactory<Storage> {
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

// Factory contains no data whatsoever and is therefore entirely safe to pass around across threads
unsafe impl<Storage> Sync for GenericFactory<Storage> {}
unsafe impl<Storage> Send for GenericFactory<Storage> {}

impl<Storage> StorageFactory for GenericFactory<Storage>
where
    for<'de> Storage: 'static + serde::Serialize + EntityDeserialize<'de>,
{
    fn storage_tag(&self) -> String {
        std::any::type_name::<Storage>().to_string()
    }

    fn storage_type_id(&self) -> TypeId {
        TypeId::of::<Storage>()
    }

    fn serializable_storage<'a>(&self, storage: &'a dyn Any) -> eyre::Result<&'a dyn Serialize> {
        storage
            .downcast_ref::<Storage>()
            .map(|storage| storage as &dyn Serialize)
            .ok_or_else(|| eyre!("provided storage is not known to factory"))
    }

    fn deserialize_storage(
        &self,
        deserializer: &mut dyn erased_serde::Deserializer,
        id_map: &mut EntitySerializationMap,
    ) -> eyre::Result<Box<dyn Any>> {
        let storage = Storage::entity_deserialize(deserializer, id_map)?;
        Ok(Box::new(storage))
    }
}
