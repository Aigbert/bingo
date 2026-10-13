use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LinesCodec, LinesCodecError};

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("JSON-RPC line too long")]
    LineTooLong,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<LinesCodecError> for CodecError {
    fn from(error: LinesCodecError) -> Self {
        match error {
            LinesCodecError::MaxLineLengthExceeded => Self::LineTooLong,
            LinesCodecError::Io(error) => Self::Io(error),
        }
    }
}

pub struct Lines(LinesCodec);

pub fn lines(max: usize) -> Lines {
    Lines(LinesCodec::new_with_max_length(max))
}

impl Decoder for Lines {
    type Item = String;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<String>, CodecError> {
        self.0.decode(src).map_err(Into::into)
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<String>, CodecError> {
        self.0.decode_eof(src).map_err(Into::into)
    }
}

impl Encoder<String> for Lines {
    type Error = CodecError;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), CodecError> {
        self.0.encode(item, dst).map_err(Into::into)
    }
}
