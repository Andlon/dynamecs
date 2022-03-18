use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::Deref;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, SerializeTuple};
use serde::{Deserialize, Deserializer, Serializer};

use crate::universe::{Storages, TaggedTypeErasedStorage};
use crate::{SerializableStorage, StorageSerializer, Universe};

static REGISTRY: Lazy<Mutex<HashMap<String, Box<dyn StorageSerializer>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RegistrationStatus {
    /// Indicates that the factory did not already exist in the registry, so it was inserted.
    Inserted,
    /// Indicates that a factory was already registered for the given typename, but it was
    /// replaced by the new factory.
    Replaced,
}

pub fn register_serializer(serializer: Box<dyn StorageSerializer>) -> RegistrationStatus {
    let mut hash_map = REGISTRY
        .lock()
        .expect("Internal error: Lock should never fail");
    if hash_map
        .insert(serializer.storage_tag(), serializer)
        .is_some()
    {
        RegistrationStatus::Replaced
    } else {
        RegistrationStatus::Inserted
    }
}

pub fn register_storage<S>() -> RegistrationStatus
where
    S: SerializableStorage,
{
    let serializer = S::create_serializer();
    register_serializer(serializer)
}

fn look_up_serializer<R>(tag: &str, f: impl FnOnce(&dyn StorageSerializer) -> R) -> Option<R> {
    let hash_map = REGISTRY
        .lock()
        .expect("Internal error: Lock should never fail");
    let serializer = hash_map.get(tag)?;
    Some(f(serializer.deref()))
}

impl serde::Serialize for TaggedTypeErasedStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;

        tuple.serialize_element(&self.tag)?;

        // Note: We have two layers of errors that we have to unravel:
        // 1. the possibility of a serializer not having been registered
        // 2. the serialization itself failing
        look_up_serializer(&self.tag, |storage_serializer| -> Result<(), S::Error> {
            let serializable = storage_serializer
                .serializable_storage(self.storage.as_ref())
                .ok_or_else(|| {
                    let msg = format!(
                        "Internal error: Mismatch between storage tag '{}' and serializer",
                        &self.tag
                    );
                    serde::ser::Error::custom(msg)
                })?;
            tuple.serialize_element(&serializable)
        })
        .ok_or_else(|| {
            let msg = format!(
                "Could not serialize as no serializer is registered for tag {}",
                &self.tag
            );
            serde::ser::Error::custom(msg)
        })??;

        tuple.end()
    }
}

struct TaggedTypeErasedStorageVisitor;

impl<'de> Visitor<'de> for TaggedTypeErasedStorageVisitor {
    type Value = TaggedTypeErasedStorage;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a tag followed by a serialized storage")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // We use DeserializeSeed in order to "seed" deserialization with the storage tag
        struct TypeErasedStorageSeed<'a> {
            tag: &'a str,
        }

        impl<'a, 'de> DeserializeSeed<'de> for TypeErasedStorageSeed<'a> {
            type Value = Box<dyn Any + 'static>;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                look_up_serializer(&self.tag, |storage_serializer| {
                    let erased_deserializer = &mut <dyn erased_serde::Deserializer>::erase(deserializer);
                    storage_serializer.deserialize_storage(erased_deserializer)
                })
                .ok_or_else(|| {
                    let msg = format!(
                        "Could not deserialize as no serializer is registered for tag {}",
                        &self.tag
                    );
                    serde::de::Error::custom(msg)
                })?
                .map_err(serde::de::Error::custom)
            }
        }

        let tag: String = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::custom("missing tag in sequence"))?;

        let erased_storage = seq
            .next_element_seed(TypeErasedStorageSeed { tag: &tag })?
            .ok_or_else(|| serde::de::Error::custom("missing storage in sequence"))?;

        Ok(TaggedTypeErasedStorage {
            tag,
            storage: erased_storage,
        })
    }
}

impl<'de> serde::Deserialize<'de> for TaggedTypeErasedStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, TaggedTypeErasedStorageVisitor)
    }
}

impl serde::Serialize for Storages {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let storages = self.storages.borrow();
        let mut seq = serializer.serialize_seq(Some(storages.len()))?;
        for (_, storage) in storages.iter() {
            seq.serialize_element(&storage)?;
        }
        seq.end()
    }
}

impl<'de> serde::Deserialize<'de> for Storages {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let storages = <Vec<TaggedTypeErasedStorage> as Deserialize<'de>>::deserialize(deserializer)?;
        let mut hash_map = HashMap::new();
        for storage in storages {
            let type_id = look_up_serializer(&storage.tag, |storage_serializer| storage_serializer.storage_type_id())
                .expect("Internal error: Serializer must exist since we managed to successfully ");
            hash_map.insert(type_id, storage);
        }
        Ok(Self {
            storages: RefCell::new(hash_map),
        })
    }
}

impl Universe {
    /// Returns tags of component storages that are currently present in this `Universe` but which are not registered (for serialization).
    ///
    /// This function can be helpful during development to ensure that all components are registered, e.g. by printing
    /// a warning or error with the non-registered components.
    pub fn unregistered_components(&self) -> Vec<String> {
        let storages = RefCell::borrow(&self.storages);
        storages
            .iter()
            .filter_map(|(_, TaggedTypeErasedStorage { tag, .. })| {
                look_up_serializer(&tag, |_| {}).is_none().then(|| tag)
            })
            .cloned()
            .collect()
    }
}
