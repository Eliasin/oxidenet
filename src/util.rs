use serde::de::DeserializeOwned;
use serde::Serialize;
use smol::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::io::{Read, Write};

pub async fn send_length_prefixed_object_async<T: Serialize, W: AsyncWrite + Unpin>(
    obj: &T,
    w: &mut W,
) -> anyhow::Result<()> {
    let obj_size: u64 = bincode::serialized_size(&obj)?;

    w.write_all(&obj_size.to_le_bytes()).await?;

    let obj_bytes = bincode::serialize(obj)?;
    w.write_all(&obj_bytes).await?;

    Ok(())
}

pub async fn receive_length_prefixed_object_async<T: DeserializeOwned, R: AsyncRead + Unpin>(
    r: &mut R,
) -> anyhow::Result<T> {
    let mut size_buf = [0; std::mem::size_of::<u64>()];

    r.read_exact(&mut size_buf).await?;

    let size = u64::from_le_bytes(size_buf);

    let mut buf = vec![0; size.try_into().expect("u64 to fit into usize")];

    r.read_exact(&mut buf).await?;

    Ok(bincode::deserialize(&buf)?)
}

pub fn send_length_prefixed_object<T: Serialize, W: Write>(
    obj: &T,
    w: &mut W,
) -> anyhow::Result<()> {
    let obj_size: u64 = bincode::serialized_size(&obj)?;

    w.write_all(&obj_size.to_le_bytes())?;

    let obj_bytes = bincode::serialize(obj)?;
    w.write_all(&obj_bytes)?;

    Ok(())
}

pub fn receive_length_prefixed_object<T: DeserializeOwned, R: Read>(
    r: &mut R,
) -> anyhow::Result<T> {
    let mut size_buf = [0; std::mem::size_of::<u64>()];

    r.read_exact(&mut size_buf)?;

    let size = u64::from_le_bytes(size_buf);

    let mut buf = vec![0; size.try_into().expect("u64 to fit into usize")];

    r.read_exact(&mut buf)?;

    Ok(bincode::deserialize(&buf)?)
}
