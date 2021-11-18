use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Mutex;

use eyre::eyre;
use once_cell::sync::Lazy;
use serde::de::DeserializeSeed;

use crate::serialization::EntitySerializationMap;
use crate::universe::TaggedTypeErasedStorage;
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

pub fn register_factory(factory: Box<dyn StorageSerializer>) -> eyre::Result<RegistrationStatus> {
    let mut hash_map = REGISTRY
        .lock()
        .map_err(|_| eyre!("failed to obtain registry lock"))?;
    // TODO: Handle collision, i.e. if the tag has already been registered
    if hash_map.insert(factory.storage_tag(), factory).is_some() {
        Ok(RegistrationStatus::Replaced)
    } else {
        Ok(RegistrationStatus::Inserted)
    }
}

pub fn register_storage<S>() -> eyre::Result<RegistrationStatus>
where
    S: SerializableStorage,
{
    let factory = S::create_serializer();
    register_factory(factory)
}

fn look_up_factory<R>(tag: &str, f: impl FnOnce(&dyn StorageSerializer) -> R) -> eyre::Result<R> {
    let hash_map = REGISTRY
        .lock()
        .map_err(|_| eyre!("failed to obtain registry lock"))?;
    let factory = hash_map
        .get(tag)
        .ok_or_else(|| eyre!("no factory registered for given tag {}", tag))?;
    Ok(f(factory.deref()))
}

// TODO: Naming
struct FactoryWrapper<'a> {
    factory: &'a dyn StorageSerializer,
    id_map: &'a mut EntitySerializationMap,
}

impl<'a, 'de> serde::de::DeserializeSeed<'de> for FactoryWrapper<'a> {
    type Value = Box<dyn Any>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let erased_deserializer = &mut <dyn erased_serde::Deserializer>::erase(deserializer);
        self.factory
            .deserialize_storage(erased_deserializer, self.id_map)
            .map_err(serde::de::Error::custom)
    }
}

struct TaggedStorage<'a> {
    id_map: &'a mut EntitySerializationMap,
}

impl<'a, 'b, 'de> DeserializeSeed<'de> for &'b mut TaggedStorage<'a> {
    type Value = (String, Box<dyn Any>);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, self)
    }
}

impl<'a, 'b, 'de> serde::de::Visitor<'de> for &'b mut TaggedStorage<'a> {
    type Value = (String, Box<dyn Any>);

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "2-element sequence (tag, storage)")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let tag: String = seq
            .next_element()?
            .ok_or_else(|| "missing tag in sequence")
            .map_err(serde::de::Error::custom)?;

        look_up_factory(&tag, |factory| -> eyre::Result<_> {
            let wrapper = FactoryWrapper {
                factory: factory.deref(),
                id_map: self.id_map,
            };

            let storage: Box<dyn Any> = seq
                .next_element_seed(wrapper)
                .map_err(|e| eyre!("{}", e))?
                .ok_or_else(|| eyre!("missing storage in sequence"))?;
            Ok(storage)
        })
        // First set of errors is error from locking and looking up the factory
        .map_err(serde::de::Error::custom)?
        .map(|storage| (tag, storage))
        // Second set is from the deserialization of the storage
        .map_err(serde::de::Error::custom)
    }
}

struct StorageContainerVisitor(EntitySerializationMap);

impl<'de> serde::de::Visitor<'de> for StorageContainerVisitor {
    type Value = Universe;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "sequence of tuples (tag, storage)")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut storages = HashMap::new();
        let mut tagged = TaggedStorage { id_map: &mut self.0 };

        while let Some((tag, storage)) = seq.next_element_seed(&mut tagged)? {
            let type_id = look_up_factory(&tag, |factory| factory.storage_type_id()).map_err(|err| {
                serde::de::Error::custom(format!(
                    "No factory registered for tag. Cannot deserialize. Internal error: {}",
                    err
                ))
            })?;
            storages.insert(type_id, TaggedTypeErasedStorage { tag, storage });
        }

        let container = Universe {
            storages: RefCell::new(storages),
        };

        Ok(container)
    }
}

impl<'de> serde::Deserialize<'de> for Universe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id_map = EntitySerializationMap::new();
        deserializer.deserialize_seq(StorageContainerVisitor(id_map))
    }
}

impl serde::Serialize for Universe {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        let storages = self.storages.borrow();
        let mut seq = serializer.serialize_seq(Some(storages.len()))?;
        for (_, TaggedTypeErasedStorage { tag, storage }) in storages.iter() {
            look_up_factory(&tag, |factory| {
                let serialize = factory
                    .serializable_storage(storage.as_ref())
                    .map_err(serde::ser::Error::custom)?;
                seq.serialize_element(&(tag, &serialize))
            })
            // Handle error from factory look-up
            .map_err(serde::ser::Error::custom)?
            // Handle error from serialization
            .map_err(serde::ser::Error::custom)?;
        }
        seq.end()
    }
}
