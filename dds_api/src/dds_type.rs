pub trait DDSType: 'static + Send + Sync {
    fn type_name() -> &'static str;

    fn has_key() -> bool;
    
    fn key(&self) -> Vec<u8>;

    fn serialize(&self) -> Vec<u8>;

    fn deserialize(data: Vec<u8>) -> Self;
}