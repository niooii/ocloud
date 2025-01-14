use crate::error::Result;
use std::pin::Pin;
use bytes::Bytes;
use futures_util::Stream;

pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>;

pub trait StorageBackend: Send + Sync {
    async fn put_object(
        &self, 
        key: &str, 
        data: impl Stream<Item = Result<Bytes>> + Send + 'static
    ) -> Result<()>;

    async fn stream_object(&self, key: &str) -> Result<ByteStream>;
    
    async fn stream_object_range(
        &self, 
        key: &str,
        range: std::ops::Range<u64>
    ) -> Result<ByteStream>;

    async fn delete_object(&self, key: &str) -> Result<()>;
}