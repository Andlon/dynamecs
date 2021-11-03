use std::any::{Any, TypeId};
use std::error::Error;
use std::marker::PhantomData;

use erased_serde::Serialize;

use crate::serialization::{EntityDeserialize, EntitySerializationMap};
use crate::StorageFactory;

#[derive(Debug, Default)]
pub struct GenericFactory<Storage> {
    marker: PhantomData<Storage>,
}

impl<Storage> GenericFactory<Storage> {
    pub fn new() -> Self {
        Self { marker: PhantomData }
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

    fn serializable_storage<'a>(&self, storage: &'a dyn Any) -> Result<&'a dyn Serialize, Box<dyn Error>> {
        storage
            .downcast_ref::<Storage>()
            .map(|storage| storage as &dyn Serialize)
            .ok_or_else(|| Box::from("provided storage is not known to factory"))
    }

    fn deserialize_storage(
        &self,
        deserializer: &mut dyn erased_serde::Deserializer,
        id_map: &mut EntitySerializationMap,
    ) -> Result<Box<dyn Any>, Box<dyn Error>> {
        let storage = Storage::entity_deserialize(deserializer, id_map)?;
        Ok(Box::new(storage))
    }
}
