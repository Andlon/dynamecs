use dynamecs::serialization::GenericStorageSerializer;
use dynamecs::{register_serializer, RegistrationStatus};

#[test]
fn register() {
    // Important: registration is global, so we must run this test in a separate binary,
    // which we do when we make it a separate integration test
    let make_factory = || Box::new(GenericStorageSerializer::<i32>::default());
    let make_factory2 = || Box::new(GenericStorageSerializer::<i64>::default());

    assert_eq!(register_serializer(make_factory()), RegistrationStatus::Inserted);
    assert_eq!(register_serializer(make_factory()), RegistrationStatus::Replaced);
    assert_eq!(register_serializer(make_factory()), RegistrationStatus::Replaced);

    assert_eq!(register_serializer(make_factory2()), RegistrationStatus::Inserted);
    assert_eq!(register_serializer(make_factory2()), RegistrationStatus::Replaced);

    assert_eq!(register_serializer(make_factory()), RegistrationStatus::Replaced);
}
