extern crate tracing as log;

pub mod auth;
pub mod client;
pub mod error;
pub mod event;
pub mod msg;
pub mod stream;

pub fn simple_de<T>(value: &rkyv::Archived<T>) -> T
where
    T: rkyv::Archive,
    rkyv::Archived<T>: rkyv::Deserialize<T, rkyv::Infallible>,
{
    rkyv::Deserialize::deserialize(value, &mut rkyv::Infallible).unwrap()
}
