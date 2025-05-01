#[derive(Debug)]
pub(crate) struct SecretReference {
    src: String,
}

#[derive(Debug)]
enum SecretReferenceError {
}
