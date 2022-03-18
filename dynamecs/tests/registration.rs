use dynamecs::serialization::GenericStorageSerializer;
use dynamecs::{register_serializer, RegistrationStatus};

#[test]
fn register() {
    // Important: registration is global, so we must run this test in a separate binary,
    // which we do when we make it a separate integration test
    let make_serializer = || Box::new(GenericStorageSerializer::<i32>::default());
    let make_serializer2 = || Box::new(GenericStorageSerializer::<i64>::default());

    assert_eq!(register_serializer(make_serializer()), RegistrationStatus::Inserted);
    assert_eq!(register_serializer(make_serializer()), RegistrationStatus::Replaced);
    assert_eq!(register_serializer(make_serializer()), RegistrationStatus::Replaced);

    assert_eq!(register_serializer(make_serializer2()), RegistrationStatus::Inserted);
    assert_eq!(register_serializer(make_serializer2()), RegistrationStatus::Replaced);

    assert_eq!(register_serializer(make_serializer()), RegistrationStatus::Replaced);
}
