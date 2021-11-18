use std::any::{Any, TypeId};
use std::marker::PhantomData;

use erased_serde::Serialize;
use eyre::eyre;

use crate::serialization::{EntityDeserialize, EntitySerializationMap};
use crate::{StorageSerializer, Storage};

/// Generic storage serializer.
///
/// Not intended to be used outside this crate. It is currently public with hidden docs because it is needed
/// for integration tests.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct GenericStorageSerializer<Storage> {
    marker: PhantomData<Storage>,
}

impl<Storage> GenericStorageSerializer<Storage> {
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

// Factory contains no data whatsoever and is therefore entirely safe to pass around across threads
unsafe impl<Storage> Sync for GenericStorageSerializer<Storage> {}
unsafe impl<Storage> Send for GenericStorageSerializer<Storage> {}

impl<S> StorageSerializer for GenericStorageSerializer<S>
where
    S: 'static + Storage + serde::Serialize,
    for<'de> S: EntityDeserialize<'de>,
{
    fn storage_tag(&self) -> String {
        S::tag()
    }

    fn serializable_storage<'a>(&self, storage: &'a dyn Any) -> eyre::Result<&'a dyn Serialize> {
        storage
            .downcast_ref::<S>()
            .map(|storage| storage as &dyn Serialize)
            .ok_or_else(|| eyre!("provided storage is not known to factory"))
    }

    fn deserialize_storage(
        &self,
        deserializer: &mut dyn erased_serde::Deserializer,
        id_map: &mut EntitySerializationMap,
    ) -> eyre::Result<Box<dyn Any>> {
        let storage = S::entity_deserialize(deserializer, id_map)?;
        Ok(Box::new(storage))
    }

    fn storage_type_id(&self) -> TypeId {
        TypeId::of::<S>()
    }
}
