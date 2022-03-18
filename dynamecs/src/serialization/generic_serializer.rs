use std::any::{Any, TypeId};
use std::marker::PhantomData;

use erased_serde::{Deserializer, Error, Serialize};

use crate::{Storage, StorageSerializer};

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
        Self { marker: PhantomData }
    }
}

// Factory contains no data whatsoever and is therefore entirely safe to pass around across threads
unsafe impl<Storage> Sync for GenericStorageSerializer<Storage> {}
unsafe impl<Storage> Send for GenericStorageSerializer<Storage> {}

impl<S> StorageSerializer for GenericStorageSerializer<S>
where
    S: 'static + Storage + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    fn storage_tag(&self) -> String {
        S::tag()
    }

    fn serializable_storage<'a>(&self, storage: &'a dyn Any) -> Option<&'a dyn Serialize> {
        storage
            .downcast_ref::<S>()
            .map(|storage| storage as &dyn Serialize)
    }

    fn deserialize_storage<'a>(&self, deserializer: &mut dyn Deserializer) -> Result<Box<dyn Any>, Error> {
        let storage = S::deserialize(deserializer)?;
        Ok(Box::new(storage))
    }

    fn storage_type_id(&self) -> TypeId {
        TypeId::of::<S>()
    }
}
