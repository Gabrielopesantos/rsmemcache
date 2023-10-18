#[derive(Debug)]
pub struct Item {
    // NOTE: Maybe not a `String`?
    pub key: String,
    pub value: Vec<u8>,
    pub flags: u32,
    pub expiration: i32,
    pub cas_id: u64,
}

impl Item {
    pub(crate) fn new(key: String, value: Vec<u8>, flags: u32, expiration: i32) -> Self {
        Self {
            key,
            value,
            flags,
            expiration,
            cas_id: 0, //  NOTE: Add
        }
    }
}
